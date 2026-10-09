use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = input
        .live_url
        .split('?')
        .next()
        .unwrap()
        .rsplit("lailianjie.com/")
        .next()
        .ok_or("房间 ID 缺失")?;
    let json=ctx.json(&format!("https://api.lailianjie.com/ApiServices/service/live/getRoomInfo?&_$t=&_sign=&roomNumber={}",encode(id)),&[],Body::None).await?;
    let info = at(&json, "/data")?;
    let mut data = StreamData::new(
        input,
        "连接直播",
        field(info, "/nickname")?,
        at(info, "/isonline")?.as_i64() == Some(1),
    );
    if data.is_live {
        data.title = Some(field(info, "/defaultRoomTitle")?);
        data.urls(String::new(), field(info, "/videoUrl")?, true);
    }
    Ok(data)
}
