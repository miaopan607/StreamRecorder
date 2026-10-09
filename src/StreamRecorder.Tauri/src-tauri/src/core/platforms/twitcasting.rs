use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let parsed =
        url::Url::parse(&input.live_url).map_err(|_| "TwitCasting URL 无效".to_string())?;
    let id = parsed
        .path_segments()
        .and_then(|mut s| s.next())
        .ok_or("TwitCasting 用户 ID 缺失")?;
    let mut session = ctx.session.fields.lock().await;
    let mut cookie=session.get("cookie").cloned().or_else(||input.cookies.clone()).unwrap_or_else(||"hl=zh; did=377eda93b5320f104357ab1bc98dfe4d; _ga=GA1.1.869052351.1747879503; keep=1; chid=relay_trade_jp;".into());
    let mut new_cookie = None;
    if query(&input.live_url, "login").ok().as_deref() == Some("true") {
        cookie = login(ctx, input, &cookie).await?;
        new_cookie = Some(cookie.clone());
        session.insert("cookie".into(), cookie.clone());
    }
    let page = ctx.get(&input.live_url, &headers(&cookie)).await?;
    let fields = match parse(&page) {
        Ok(v) => v,
        Err(_) => {
            cookie = login(ctx, input, &cookie).await?;
            new_cookie = Some(cookie.clone());
            session.insert("cookie".into(), cookie.clone());
            parse(&ctx.get(&input.live_url, &headers(&cookie)).await?)?
        }
    };
    let (name, live, title) = fields;
    let mut data = StreamData::new(input, "TwitCasting", name, live);
    data.new_cookies = new_cookie;
    if live {
        data.title = Some(title);
        let info = ctx
            .json(
                &format!(
                    "https://twitcasting.tv/streamserver.php?target={}&mode=client&player=pc_web",
                    encode(id)
                ),
                &headers(&cookie),
                Body::None,
            )
            .await?;
        let streams = object(at(&info, "/tc-hls/streams")?)?;
        let urls = ["high", "medium", "low"]
            .into_iter()
            .filter_map(|k| streams.get(k).map(text))
            .collect::<Vec<_>>();
        data.urls(pick(&urls, &input.quality)?.clone(), String::new(), false);
    }
    Ok(data)
}
fn headers(cookie: &str) -> [(&str, &str); 4] {
    [("user-agent","ios/7.830 (ios 17.0; ; iPhone 15 (A2846/A3089/A3090/A3092))"),("content-type","application/x-www-form-urlencoded"),("referer","https://twitcasting.tv/indexcaslogin.php?redir=%2Findexloginwindow.php%3Fnext%3D%252F&keep=1"),("cookie",cookie)]
}
fn parse(html: &str) -> Result<(String, bool, String), String> {
    let name = capture(r"<title>(.*?) \(@.*?\)  的直播 - Twit", html)?;
    let id = capture(r"<title>.*? \(@(.*?)\)  的直播 - Twit", html)?;
    let movie = capture(r#"data-movie-id="(.*?)" data-audience-id"#, html)?;
    let title = capture(
        r#"<meta name="twitter:title" content="(.*?)">\s+<meta"#,
        html,
    )?;
    let status = capture(r#"data-is-onlive="(.*?)"\s+data-view-mode"#, html)?;
    Ok((
        format!("{}-{id}-{movie}", name.trim()),
        status == "true",
        title,
    ))
}
async fn login(ctx: &ProbeContext, input: &ProbeInput, cookie: &str) -> Result<String, String> {
    let username = input
        .username
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("TwitCasting 需要登录账号")?;
    let password = input
        .password
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("TwitCasting 需要登录密码")?;
    let (url, api) = if input.account_type.as_deref() == Some("twitter") {
        ("https://twitcasting.tv/indexpasswordlogin.php","https://twitcasting.tv/indexpasswordlogin.php?redir=/indexloginwindow.php?next=%2F&keep=1")
    } else {
        (
            "https://twitcasting.tv/indexcaslogin.php?redir=%2F&keep=1",
            "https://twitcasting.tv/indexcaslogin.php?redir=/indexloginwindow.php?next=%2F&keep=1",
        )
    };
    let html = ctx.get(url, &headers(cookie)).await?;
    let session = capture(
        r#"<input type="hidden" name="cs_session_id" value="(.*?)">"#,
        &html,
    )?;
    let response = ctx
        .request(
            reqwest::Method::POST,
            api,
            &headers(cookie),
            form(&[
                ("username", username),
                ("password", password),
                ("action", "login"),
                ("cs_session_id", &session),
            ]),
        )
        .await?;
    if !response.cookies.contains_key("tc_ss") {
        return Err("TwitCasting 登录失败，请检查账号密码".into());
    }
    Ok(cookie_string(&response.cookies))
}
