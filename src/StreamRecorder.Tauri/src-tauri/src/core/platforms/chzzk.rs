use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers = [
        ("accept", "application/json, text/plain, */*"),
        ("origin", "https://chzzk.naver.com"),
        (
            "referer",
            "https://chzzk.naver.com/live/458f6ec20b034f49e0fc6d03921646d2",
        ),
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0",
        ),
    ];
    let json = ctx
        .json(
            &format!(
                "https://api.chzzk.naver.com/service/v3/channels/{}/live-detail",
                encode(&room(&input.live_url)?)
            ),
            &headers,
            Body::None,
        )
        .await?;
    let info = at(&json, "/content")?;
    let mut data = StreamData::new(
        input,
        "CHZZK",
        field(info, "/channel/channelName")?,
        field(info, "/status")? == "OPEN",
    );
    if data.is_live {
        let play = parse_json(&field(info, "/livePlaybackJson")?)?;
        let hls = field(&play, "/media/0/path")?;
        let urls = ctx.variants(&hls, &headers).await?;
        data.urls(pick(&urls, &input.quality)?.clone(), String::new(), false);
        data.m3u8_url = Some(hls);
    }
    Ok(data)
}
