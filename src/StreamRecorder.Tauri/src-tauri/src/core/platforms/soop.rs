use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let parts: Vec<_> = input.live_url.split('/').collect();
    let id = if parts.len() < 6 {
        parts.get(3)
    } else {
        parts.get(5)
    }
    .ok_or("SOOP 房间 ID 缺失")?;
    let mut session = ctx.session.fields.lock().await;
    let mut cookie = session
        .get("cookie")
        .cloned()
        .or_else(|| input.cookies.clone())
        .unwrap_or_default();
    let mut new_cookie = None;
    let nick = ctx
        .json(
            &format!(
                "https://st.sooplive.com/api/get_station_status.php?szBjId={}",
                encode(id)
            ),
            &headers(&cookie),
            Body::None,
        )
        .await?;
    let mut channel = fetch_channel(ctx, input, id, "", &cookie).await?;
    if !matches!(
        channel.get("RESULT").and_then(serde_json::Value::as_i64),
        Some(0 | 1)
    ) {
        let username = input
            .username
            .as_deref()
            .filter(|s| s.len() >= 6)
            .ok_or("SOOP 需要有效账号")?;
        let password = input
            .password
            .as_deref()
            .filter(|s| s.len() >= 10)
            .ok_or("SOOP 需要有效密码")?;
        let response = ctx
            .request(
                reqwest::Method::POST,
                "https://login.sooplive.com/app/LoginAction.php",
                &headers(&cookie),
                form(&[
                    ("szWork", "login"),
                    ("szType", "json"),
                    ("szUid", username),
                    ("szPassword", password),
                    ("isSaveId", "true"),
                    ("isSavePw", "true"),
                    ("isSaveJoin", "true"),
                    ("isLoginRetain", "Y"),
                ]),
            )
            .await?;
        if response.cookies.is_empty() {
            return Err("SOOP 登录失败，请检查账号密码".into());
        }
        cookie = cookie_string(&response.cookies);
        session.insert("cookie".into(), cookie.clone());
        new_cookie = Some(cookie.clone());
        channel = fetch_channel(ctx, input, id, "", &cookie).await?;
        if !matches!(
            channel.get("RESULT").and_then(serde_json::Value::as_i64),
            Some(0 | 1)
        ) {
            return Err("SOOP 登录后仍无法访问直播间".into());
        }
    }
    let mut data = StreamData::new(
        input,
        "SOOP",
        format!("{}-{id}", field(&nick, "/DATA/user_nick")?),
        channel.get("VIEWPRESET").is_some_and(truth),
    );
    data.new_cookies = new_cookie;
    if data.is_live {
        data.title = Some(optional(&channel, "/TITLE"));
        let broad = field(&channel, "/BNO")?;
        let info=ctx.json(&format!("http://livestream-manager.sooplive.com/broad_stream_assign.html?return_type=gcp_cdn&use_cors=false&cors_origin_url=play.sooplive.com&broad_key={}&time=3061.2892404235236",encode(&format!("{broad}-common-master-hls"))),&headers(&cookie),Body::None).await?;
        let token = fetch_channel(ctx, input, id, "aid", &cookie).await?;
        let hls = format!(
            "{}?aid={}",
            field(&info, "/view_url")?,
            field(&token, "/AID")?
        );
        let urls=ctx.variants(&hls,&[("user-agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36 Edg/141.0.0.0")]).await?;
        data.urls(pick(&urls, &input.quality)?.clone(), String::new(), false);
        data.m3u8_url = Some(hls);
    }
    Ok(data)
}
fn headers(cookie: &str) -> [(&str, &str); 5] {
    [
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:122.0) Gecko/20100101 Firefox/122.0",
        ),
        (
            "content-type",
            "application/x-www-form-urlencoded; charset=UTF-8",
        ),
        ("origin", "https://play.sooplive.com"),
        ("referer", "https://play.sooplive.com/superbsw123/277837074"),
        ("cookie", cookie),
    ]
}
async fn fetch_channel(
    ctx: &ProbeContext,
    input: &ProbeInput,
    id: &str,
    kind: &str,
    cookie: &str,
) -> Result<serde_json::Value, String> {
    let password = query(&input.live_url, "pwd").unwrap_or_default();
    let info = ctx
        .json(
            &format!(
                "https://live.sooplive.com/afreeca/player_live_api.php?bjid={}",
                encode(id)
            ),
            &headers(cookie),
            form(&[
                ("bid", id),
                ("bno", ""),
                ("type", kind),
                ("pwd", &password),
                ("player_type", "html5"),
                ("stream_type", "common"),
                ("quality", "master"),
                ("mode", "landing"),
                ("from_api", "0"),
                ("is_revive", "false"),
            ]),
        )
        .await?;
    Ok(at(&info, "/CHANNEL")?.clone())
}
