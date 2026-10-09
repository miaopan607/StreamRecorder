use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let api = format!(
        "https://webapi.busi.inke.cn/web/live_share_pc?uid={}&id={}&_t={}",
        encode(&query(&input.live_url, "uid")?),
        encode(&query(&input.live_url, "id")?),
        chrono::Utc::now().timestamp()
    );
    let json = ctx.json(&api, &[], Body::None).await?;
    let info = at(&json, "/data")?;
    let mut data = StreamData::new(
        input,
        "映客直播",
        field(info, "/media_info/nick")?,
        at(info, "/status")?.as_i64() == Some(1),
    );
    if data.is_live {
        data.urls(
            field(info, "/live_addr/0/hls_stream_addr")?,
            field(info, "/live_addr/0/stream_addr")?,
            false,
        );
    }
    Ok(data)
}
