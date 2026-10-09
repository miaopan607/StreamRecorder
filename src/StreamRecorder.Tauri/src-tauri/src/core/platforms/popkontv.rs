use crate::core::probe::*;
use serde_json::{json, Value};
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = query(&input.live_url, "mcid").or_else(|_| query(&input.live_url, "castId"))?;
    let mut session = ctx.session.fields.lock().await;
    let mut partner = String::from("P-00001");
    let mut token = session.get("token").cloned().unwrap_or_default();
    let search = ctx
        .json(
            "https://www.popkontv.com/api/proxy/broadcast/v1.1/search/all",
            &headers(&token),
            Body::Json(json!({"partnerCode":"P-00001","searchKeyword":id,"signId":input.username})),
        )
        .await?;
    let list = array(at(&search, "/data/broadCastList")?)?;
    let name = if let Some(info) = list.iter().find(|v| optional(v, "/mcSignId") == id) {
        partner = field(info, "/mcPartnerCode")?;
        format!("{}-{id}", field(info, "/nickName")?)
    } else {
        partner = query(&input.live_url, "mcPartnerCode")
            .or_else(|_| query(&input.live_url, "partnerCode"))
            .unwrap_or(partner);
        let html = ctx
            .get(
                &format!(
                    "https://www.popkontv.com/channel/notices?mcid={}&mcPartnerCode={}",
                    encode(&id),
                    encode(&partner)
                ),
                &headers(&token),
            )
            .await?;
        format!(
            "{id}-{}",
            capture(r#""mcNickName":"([^"]+)""#, &html).unwrap_or_else(|_| "Unknown".into())
        )
    };
    let html = ctx
        .get(
            &format!(
                "https://www.popkontv.com/live/view?castId={}&partnerCode={}",
                encode(&id),
                encode(&partner)
            ),
            &headers(&token),
        )
        .await?;
    let info = parse_json(&capture(
        r#"<script id="__NEXT_DATA__" type="application/json">(.*?)</script>"#,
        &html,
    )?)?;
    let Some(room) = info.pointer("/props/pageProps/mcData/data") else {
        return Ok(StreamData::new(input, "PopkonTV", name, false));
    };
    let private = field(room, "/mc_isPrivate")?
        .parse::<i64>()
        .map_err(|_| "PopkonTV 私密状态无效".to_string())?;
    let password = query(&input.live_url, "pwd").ok();
    if private != 0 && password.is_none() {
        return Err("PopkonTV 私密房间需要房间密码".into());
    }
    let mut date = field(room, "/mc_castStartDate")?;
    let mut reply = play(
        ctx,
        input,
        room,
        &partner,
        "P-00001",
        &date,
        password.as_deref(),
        &token,
    )
    .await;
    let needs_login = match &reply {
        Ok(value) => optional(value, "/statusCd") == "E5000",
        Err(error) => error.contains("HTTP 状态 400"),
    };
    let mut new_token = None;
    if needs_login {
        let username = input
            .username
            .as_deref()
            .filter(|s| s.len() >= 4)
            .ok_or("PopkonTV 需要有效账号")?;
        let secret = input
            .password
            .as_deref()
            .filter(|s| s.len() >= 10)
            .ok_or("PopkonTV 需要有效密码")?;
        let login=ctx.json("https://www.popkontv.com/api/proxy/member/v1/login",&[("authorization","Basic FpAhe6mh8Qtz116OENBmRddbYVirNKasktdXQiuHfm88zRaFydTsFy63tzkdZY0u"),("origin","https://www.popkontv.com"),("user-agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:124.0) Gecko/20100101 Firefox/124.0")],Body::Json(json!({"partnerCode":"P-00001","signId":username,"signPwd":secret}))).await?;
        if field(&login, "/statusCd")? != "S2000" {
            return Err("PopkonTV 登录失败，请检查账号密码".into());
        }
        let raw_token = field(&login, "/data/token")?;
        if raw_token.len() != 640 {
            return Err("PopkonTV 登录令牌无效".into());
        }
        let login_partner = field(&login, "/data/partnerCode")?;
        token = format!("Bearer {raw_token}");
        session.insert("token".into(), token.clone());
        new_token = Some(raw_token);
        reply = play(
            ctx,
            input,
            room,
            &partner,
            &login_partner,
            &date,
            password.as_deref(),
            &token,
        )
        .await;
    }
    let mut reply = reply?;
    let code = field(&reply, "/statusCd")?;
    let message = field(&reply, "/statusMsg")?;
    if message != "SUCEESS" {
        return Err(format!("PopkonTV 播放失败：{message}"));
    }
    match code.as_str() {
        "L000A" => return Err("PopkonTV 账号需要完成手机号验证".into()),
        "L0001" => {
            date = (date
                .parse::<i64>()
                .map_err(|_| "PopkonTV 开播时间编码无效".to_string())?
                - 1)
            .to_string();
            reply = play(
                ctx,
                input,
                room,
                &partner,
                "P-00001",
                &date,
                password.as_deref(),
                &token,
            )
            .await?;
        }
        "L0000" => {}
        _ => return Err(format!("PopkonTV 播放失败：{code}")),
    }
    let url = optional(&reply, "/data/castHlsUrl");
    let mut data = StreamData::new(input, "PopkonTV", name, !url.is_empty());
    data.new_token = new_token;
    if data.is_live {
        data.urls(url, String::new(), false);
    }
    Ok(data)
}
fn headers(token: &str) -> Vec<(&str, &str)> {
    let mut result=vec![("accept","application/json, text/plain, */*"),("clientKey","Client FpAhe6mh8Qtz116OENBmRddbYVirNKasktdXQiuHfm88zRaFydTsFy63tzkdZY0u"),("content-type","application/json"),("isNew","true"),("origin","https://www.popkontv.com"),("referer","https://www.popkontv.com/search?keyword=143s2"),("user-agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36 Edg/145.0.0.0")];
    if !token.is_empty() {
        result.push(("authorization", token));
    }
    result
}
async fn play(
    ctx: &ProbeContext,
    input: &ProbeInput,
    room: &Value,
    cast_partner: &str,
    partner: &str,
    date: &str,
    password: Option<&str>,
    token: &str,
) -> Result<Value, String> {
    let cast_sign = field(room, "/mc_signId")?;
    let payload = json!({"androidStore":0,"castCode":format!("{cast_sign}-{date}"),"castPartnerCode":cast_partner,"castSignId":cast_sign,"castType":at(room,"/castType")?,"commandType":0,"exePath":5,"isSecret":at(room,"/mc_isPrivate")?,"partnerCode":partner,"password":password,"signId":input.username,"version":"4.6.2"});
    let request_headers = headers(token);
    ctx.json(
        "https://www.popkontv.com/api/proxy/broadcast/v1/castwatchonoffguest",
        &request_headers,
        Body::Json(payload),
    )
    .await
}
