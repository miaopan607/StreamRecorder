use crate::core::probe::*;
use crate::core::signing;
use rand::Rng;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = query(&input.live_url, "id")?;
    let charset =
        b"1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ!@#$%^&*()_+-=[]{}|;:,.<>?";
    let mut key = [0u8; 16];
    for byte in &mut key {
        *byte = charset[rand::rngs::OsRng.gen_range(0..charset.len())];
    }
    let payload = format!(
        "{{\"liveRoomNo\": {}}}",
        serde_json::to_string(&id).map_err(|e| e.to_string())?
    );
    let (params, secret) = signing::look_encrypt(&payload, &key);
    let info = ctx
        .json(
            "https://api.look.163.com/weapi/livestream/room/get/v3",
            &[(
                "user-agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0",
            )],
            form(&[("params", &params), ("encSecKey", &secret)]),
        )
        .await?;
    let room = at(&info, "/data")?;
    let mut data = StreamData::new(
        input,
        "Look",
        field(room, "/anchor/nickName")?,
        at(room, "/liveStatus")?.as_i64() == Some(1),
    );
    if data.is_live {
        if at(room, "/roomInfo/liveType")?.as_i64() == Some(1) {
            return Err("Look 当前直播类型没有可录制流地址".into());
        }
        data.title = Some(field(room, "/roomInfo/title")?);
        data.urls(
            field(room, "/roomInfo/liveUrl/hlsPullUrl")?,
            field(room, "/roomInfo/liveUrl/httpPullUrl")?,
            false,
        );
    }
    Ok(data)
}
