use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = query(&input.live_url, "roomId")?;
    let headers = [
        (
            "user-agent",
            "ios/7.830 (ios 17.0; ; iPhone 15 (A2846/A3089/A3090/A3092))",
        ),
        ("access-control-request-method", "GET"),
        ("origin", "https://h5webcdn-pro.vvxqiu.com"),
        ("referer", "https://h5webcdn-pro.vvxqiu.com/"),
    ];
    let info = ctx
        .json(
            &format!(
                "https://h5p.vvxqiu.com/room/video/getRoomData.do?roomId={}",
                encode(&id)
            ),
            &headers,
            Body::None,
        )
        .await?;
    if info.get("status").and_then(serde_json::Value::as_i64) == Some(100)
        && !optional(&info, "/videoUrl").is_empty()
    {
        let mut data = StreamData::new(input, "VV星球直播", optional(&info, "/nickName"), true);
        data.urls(field(&info, "/videoUrl")?, String::new(), false);
        return Ok(data);
    }
    let info=ctx.json(&format!("https://h5p.vvxqiu.com/activity-center/fanclub/activity/captain/banner?roomId={}&product=vvstar",encode(&id)),&headers,Body::None).await?;
    let value = info
        .pointer("/data/anchorName")
        .ok_or("平台响应缺少字段 /data/anchorName")?;
    let mut name = (!value.is_null()).then(|| text(value));
    if name.as_deref().is_none_or(str::is_empty) {
        let info=ctx.json(&format!("https://h5p.vvxqiu.com/activity-center/halloween2023/banner?sessionId=&userId=&product=vvstar&tickToken=&roomId={}",encode(&id)),&headers,Body::None).await?;
        let value = info
            .pointer("/data/memberVO/memberName")
            .ok_or("平台响应缺少字段 /data/memberVO/memberName")?;
        name = (!value.is_null()).then(|| text(value));
    }
    let mut data = StreamData::new(input, "VV星球直播", String::new(), false);
    data.anchor_name = name;
    Ok(data)
}
