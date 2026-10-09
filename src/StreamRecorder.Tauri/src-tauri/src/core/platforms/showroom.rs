use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = if input.live_url.contains("/room/profile") {
        query(&input.live_url, "room_id")?
    } else {
        let html = ctx.get(&input.live_url, &[]).await?;
        capture(r#"href="/room/profile\?room_id=(.*?)""#, &html)?
    };
    let info = ctx
        .json(
            &format!(
                "https://www.showroom-live.com/api/live/live_info?room_id={}",
                encode(&id)
            ),
            &[],
            Body::None,
        )
        .await?;
    let name = info.get("room_name").ok_or("平台响应缺少字段 /room_name")?;
    let mut data = StreamData::new(
        input,
        "ShowRoom",
        String::new(),
        at(&info, "/live_status")?.as_i64() == Some(2),
    );
    data.anchor_name = (!name.is_null()).then(|| text(name));
    if data.is_live {
        let info=ctx.json(&format!("https://www.showroom-live.com/api/live/streaming_url?room_id={}&abr_available=1",encode(&id)),&[],Body::None).await?;
        let streams = array(at(&info, "/streaming_url_list")?)?;
        let stream = streams
            .iter()
            .find(|s| s.get("type").and_then(serde_json::Value::as_str) == Some("hls_all"))
            .ok_or("ShowRoom 没有 HLS 流")?;
        let hls = field(stream, "/url")?;
        let urls = ctx.variants(&hls, &[]).await?;
        data.urls(
            pick(&urls, &input.quality)?.replace("https://", "http://"),
            String::new(),
            false,
        );
        data.m3u8_url = Some(hls);
    }
    Ok(data)
}
