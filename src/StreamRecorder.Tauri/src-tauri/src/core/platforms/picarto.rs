use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let json = ctx
        .json(
            &format!(
                "https://ptvintern.picarto.tv/api/channel/detail/{}",
                encode(&room(&input.live_url)?)
            ),
            &[],
            Body::None,
        )
        .await?;
    let info = at(&json, "/channel")?;
    let mut data = StreamData::new(
        input,
        "Picarto",
        field(info, "/name")?,
        truth(at(info, "/online")?),
    );
    if data.is_live {
        data.title = Some(field(info, "/title")?);
        data.urls(
            format!(
                "https://1-edge1-us-newyork.picarto.tv/stream/hls/golive+{}/index.m3u8",
                data.anchor_name.as_deref().unwrap()
            ),
            String::new(),
            false,
        );
    }
    Ok(data)
}
