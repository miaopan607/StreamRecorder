use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = room(&input.live_url)?;
    let headers = [
        ("origin", "https://www.lang.live"),
        ("referer", "https://www.lang.live/"),
        (
            "user-agent",
            "ios/7.830 (ios 17.0; ; iPhone 15 (A2846/A3089/A3090/A3092))",
        ),
    ];
    let json = ctx
        .json(
            &format!(
                "https://api.lang.live/langweb/v1/room/liveinfo?room_id={}",
                encode(&id)
            ),
            &headers,
            Body::None,
        )
        .await?;
    let info = at(&json, "/data/live_info")?;
    let mut data = StreamData::new(
        input,
        "浪Live",
        field(info, "/nickname")?,
        at(info, "/live_status")?.as_i64() == Some(1),
    );
    if data.is_live {
        data.urls(
            field(info, "/liveurl_hls")?,
            field(info, "/liveurl")?,
            false,
        );
    }
    Ok(data)
}
