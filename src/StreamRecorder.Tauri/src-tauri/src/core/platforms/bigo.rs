use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers = [(
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0",
    )];
    let id = if !input.live_url.contains("bigo.tv") {
        let html = ctx.get(&input.live_url, &headers).await?;
        capture(r#"<meta data-n-head="ssr" data-hid="al:web:url" property="al:web:url" content="(.*?)">"#,&html)?.rsplit("&amp;h=").next().unwrap().to_owned()
    } else if input.live_url.contains("&h=") {
        input.live_url.rsplit("&h=").next().unwrap().to_owned()
    } else {
        room(&input.live_url)?
    };
    let json = ctx
        .json(
            "https://ta.bigo.tv/official_website/studio/getInternalStudioInfo",
            &headers,
            form(&[("siteId", &id)]),
        )
        .await?;
    let info = at(&json, "/data")?;
    let name = info.get("nick_name").ok_or("平台响应缺少字段 /nick_name")?;
    let mut data = StreamData::new(
        input,
        "Bigo",
        String::new(),
        at(info, "/alive")?.as_i64() == Some(1),
    );
    data.anchor_name = (!name.is_null()).then(|| text(name));
    if data.is_live {
        data.title = Some(field(info, "/roomTopic")?);
        data.urls(field(info, "/hls_src")?, String::new(), false);
    } else if data.anchor_name.as_deref() == Some("") {
        let country = url::Url::parse(&input.live_url)
            .map_err(|e| e.to_string())?
            .path_segments()
            .and_then(|mut x| x.next())
            .unwrap_or("")
            .to_owned();
        let html = ctx
            .get(&format!("https://www.bigo.tv/{country}/{id}"), &headers)
            .await?;
        data.anchor_name=Some(capture(r"(?s)<title>欢迎来到(.*?)的直播间</title>",&html).or_else(|_|capture(r#"(?s)<meta data-n-head="ssr" data-hid="og:title" property="og:title" content="(.*?) - BIGO LIVE">"#,&html))?);
    }
    Ok(data)
}
