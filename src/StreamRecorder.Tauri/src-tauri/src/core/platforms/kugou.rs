use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = if input.live_url.contains("roomId") {
        query(&input.live_url, "roomId")?
    } else {
        room(&input.live_url)?
    };
    let json=ctx.json(&format!("https://service2.fanxing.kugou.com/roomcen/room/web/cdn/getEnterRoomInfo?roomId={}",encode(&id)),&[],Body::None).await?;
    let info = at(&json, "/data")?;
    let name = field(info, "/normalRoomInfo/nickName")?;
    if name.is_empty() {
        return Err("音乐频道房间不支持录制，请更换直播间".into());
    }
    let mut data = StreamData::new(input, "酷狗直播", name, false);
    if at(info, "/liveType")?.as_i64() != Some(-1) {
        let api=format!("https://fx1.service.kugou.com/video/pc/live/pull/mutiline/streamaddr?std_rid={}&std_plat=7&std_kid=0&streamType=1-2-4-5-8&ua=fx-flash&targetLiveTypes=1-5-6&version=1000&supportEncryptMode=1&appid=1010&_={}",encode(&id),chrono::Utc::now().timestamp_millis());
        let json = ctx.json(&api, &[], Body::None).await?;
        let lines = array(at(&json, "/data/lines")?)?;
        if let Some(line) = lines.last() {
            data.is_live = true;
            data.urls(
                String::new(),
                field(line, "/streamProfiles/0/httpsFlv/0")?,
                true,
            );
        }
    }
    Ok(data)
}
