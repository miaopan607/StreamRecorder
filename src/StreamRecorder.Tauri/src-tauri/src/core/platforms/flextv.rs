use crate::core::probe::*;
use serde_json::json;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = input
        .live_url
        .split("/live")
        .next()
        .unwrap()
        .rsplit('/')
        .next()
        .ok_or("FlexTV 房间 ID 缺失")?;
    let url = format!("https://www.ttinglive.com/channels/{id}/live");
    let mut session = ctx.session.fields.lock().await;
    let mut cookie = session
        .get("cookie")
        .cloned()
        .or_else(|| input.cookies.clone())
        .unwrap_or_default();
    let html = ctx.get(&url, &headers(&cookie)).await?;
    let info = parse_json(&capture(
        r#"<script id="__NEXT_DATA__" type=".*">(.*?)</script>"#,
        &html,
    )?)?;
    let mut channel = at(&info, "/props/pageProps/channelStream/channel")?.clone();
    let mut new_cookie = None;
    if optional(&channel, "/message").contains("로그인후 이용이 가능합니다.") {
        let username = input
            .username
            .as_deref()
            .filter(|s| s.len() >= 6)
            .ok_or("FlexTV 需要有效账号")?;
        let password = input
            .password
            .as_deref()
            .filter(|s| s.len() >= 8)
            .ok_or("FlexTV 需要有效密码")?;
        let response=ctx.request(reqwest::Method::POST,"https://api.ttinglive.com/v2/api/auth/signin",&headers(&cookie),Body::Json(json!({"loginId":username,"password":password,"loginKeep":true,"saveId":true,"device":"PCWEB"}))).await?;
        if !response.cookies.contains_key("flx_oauth_access") {
            return Err("FlexTV 登录失败，请检查账号密码".into());
        }
        cookie = cookie_string(&response.cookies);
        session.insert("cookie".into(), cookie.clone());
        new_cookie = Some(cookie.clone());
        let html = ctx.get(&url, &headers(&cookie)).await?;
        let info = parse_json(&capture(
            r#"<script id="__NEXT_DATA__" type=".*">(.*?)</script>"#,
            &html,
        )?)?;
        channel = at(&info, "/props/pageProps/channel")?.clone();
    }
    let mut data = StreamData::new(input, "FlexTV", String::new(), false);
    data.new_cookies = new_cookie;
    if channel.get("message").is_none() {
        data.anchor_name = Some(format!(
            "{}-{}",
            field(&channel, "/owner/nickname")?,
            field(&channel, "/owner/loginId")?
        ));
        let streams = ctx
            .json(
                &format!(
                    "https://api.ttinglive.com/api/channels/{}/stream?option=all",
                    encode(id)
                ),
                &headers(&cookie),
                Body::None,
            )
            .await?;
        if let Some(stream) = streams
            .get("sources")
            .and_then(serde_json::Value::as_array)
            .and_then(|s| s.first())
        {
            let url = field(stream, "/url")?;
            data.is_live = true;
            if url.contains(".m3u8") {
                let urls = ctx.variants(&url, &headers(&cookie)).await?;
                data.urls(pick(&urls, &input.quality)?.clone(), String::new(), false);
                data.m3u8_url = Some(url);
            } else {
                data.urls(String::new(), url, true);
            }
        }
    } else {
        let html = ctx
            .get(
                &format!("https://www.ttinglive.com/channels/{id}"),
                &headers(&cookie),
            )
            .await?;
        data.anchor_name = Some(capture(
            r#"<meta name="twitter:title" content="(.*?)의"#,
            &html,
        )?);
    }
    Ok(data)
}
fn headers(cookie: &str) -> [(&str, &str); 4] {
    [
        ("accept", "application/json, text/plain, */*"),
        ("referer", "https://www.ttinglive.com/"),
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:124.0) Gecko/20100101 Firefox/124.0",
        ),
        ("cookie", cookie),
    ]
}
