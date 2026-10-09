use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    probe_site(
        ctx,
        input,
        "tlclw.com",
        "畅聊直播",
        "https://wap.tlclw.com/phone/15777?promoters=0",
    )
    .await
}
pub(super) async fn probe_site(
    ctx: &ProbeContext,
    input: &ProbeInput,
    host: &str,
    platform: &str,
    referer: &str,
) -> Result<StreamData, String> {
    let id = room(&input.live_url)?;
    let headers = [
        (
            "user-agent",
            "ios/7.830 (ios 17.0; ; iPhone 15 (A2846/A3089/A3090/A3092))",
        ),
        ("accept", "application/json, text/plain, */*"),
        ("referer", referer),
    ];
    let api = format!(
        "https://wap.{host}/api/ui/room/v1.0.0/live.ashx?roomidx={}&currentUrl={}",
        encode(&id),
        encode(&format!("https://wap.{host}/{id}"))
    );
    let json = ctx.json(&api, &headers, Body::None).await?;
    let info = at(&json, "/data/roomInfo")?;
    let mut data = StreamData::new(
        input,
        platform,
        field(info, "/nickname")?,
        at(info, "/live_stat")?.as_i64() == Some(1),
    );
    if data.is_live {
        let html = ctx.get(&input.live_url, &headers).await?;
        let raw = capture(r"(?s)var config = (.*?)config.webskins", &html)?;
        let config = parse_json(raw.rsplit_once(';').map(|(s, _)| s).unwrap_or(&raw).trim())?;
        let id = field(info, "/liveID")?;
        data.urls(
            format!("{}/{id}.m3u8", field(&config, "/domainpullstream_hls")?),
            format!("{}/{id}.flv", field(&config, "/domainpullstream_flv")?),
            true,
        );
    }
    Ok(data)
}
