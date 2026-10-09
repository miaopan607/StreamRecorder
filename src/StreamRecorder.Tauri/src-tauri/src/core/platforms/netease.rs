use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let url = if input.live_url.ends_with('/') {
        input.live_url.clone()
    } else {
        format!("{}/", input.live_url)
    };
    let html = ctx.get(&url, &[]).await?;
    let json = parse_json(&capture(
        r#"(?s)<script id="__NEXT_DATA__" .* crossorigin="anonymous">(.*?)</script></body>"#,
        &html,
    )?)?;
    let room = at(&json, "/props/pageProps/roomInfoInitData")?;
    let live = at(room, "/live")?;
    let name = live
        .get("nickname")
        .or_else(|| room.get("nickname"))
        .ok_or("平台响应缺少昵称")?;
    let mut data = StreamData::new(
        input,
        "网易CC直播",
        text(name),
        live.get("status").and_then(serde_json::Value::as_i64) == Some(1),
    );
    data.live_url = Some(url);
    if data.is_live {
        data.title = Some(field(live, "/title")?);
        let hls = optional(live, "/sharefile");
        let mut flv = String::new();
        if let Some(resolution) = live.pointer("/quickplay/resolution") {
            let qualities: Vec<_> = ["blueray", "ultra", "high", "standard"]
                .into_iter()
                .filter_map(|k| resolution.get(k))
                .collect();
            let selected = pick(&qualities, &input.quality)?;
            let cdn = object(at(selected, "/cdn")?)?;
            flv = cdn.values().next().map(text).ok_or("平台没有可用 CDN")?;
        }
        data.urls(hls, flv, true);
    }
    Ok(data)
}
