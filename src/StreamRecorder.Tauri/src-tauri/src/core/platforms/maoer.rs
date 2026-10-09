use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let json = ctx
        .json(
            &format!(
                "https://fm.missevan.com/api/v2/live/{}",
                encode(&room(&input.live_url)?)
            ),
            &[],
            Body::None,
        )
        .await?;
    let info = at(&json, "/info")?;
    let live = info.pointer("/room/status/broadcasting").is_some_and(truth);
    let mut data = StreamData::new(input, "猫耳FM直播", field(info, "/creator/username")?, live);
    if live {
        data.title = Some(field(info, "/room/name")?);
        data.urls(
            field(info, "/room/channel/hls_pull_url")?,
            field(info, "/room/channel/flv_pull_url")?,
            true,
        );
    }
    Ok(data)
}
