use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let html = ctx.get(&input.live_url, &[]).await?;
    let info = parse_json(&capture(
        r"var ytInitialPlayerResponse = (.*?);var meta = document\.createElement",
        &html,
    )?)?;
    let details = info
        .get("videoDetails")
        .ok_or("YouTube 需要有效的登录 Cookie")?;
    let mut data = StreamData::new(
        input,
        "Youtube",
        field(details, "/author")?,
        details.get("isLive").is_some_and(truth),
    );
    if data.is_live {
        data.title = Some(field(details, "/title")?);
        let hls = field(&info, "/streamingData/hlsManifestUrl")?;
        let urls = ctx.variants(&hls, &[]).await?;
        data.urls(pick(&urls, &input.quality)?.clone(), String::new(), false);
        data.m3u8_url = Some(hls);
    }
    Ok(data)
}
