use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers = [
        (
            "user-agent",
            "ios/7.830 (ios 17.0; ; iPhone 15 (A2846/A3089/A3090/A3092))",
        ),
        (
            "xy-common-params",
            "platform=iOS&sid=session.1722166379345546829388",
        ),
        ("referer", "https://app.xhs.cn/"),
    ];
    let url = if input.live_url.contains("xhslink.com") {
        ctx.request(reqwest::Method::GET, &input.live_url, &headers, Body::None)
            .await?
            .final_url
    } else {
        input.live_url.clone()
    };
    let id = capture(r"/user/profile/([^/?]+)", &url).or_else(|_| query(&url, "host_id"))?;
    let html = ctx.get(&url, &headers).await?;
    let mut data = StreamData::new(input, "小红书", String::new(), false);
    data.live_url = Some(url);
    if let Ok(raw) = capture(r"<script>window.__INITIAL_STATE__=(.*?)</script>", &html) {
        let info = parse_json(&raw.replace("undefined", "null"))?;
        if info
            .pointer("/liveStream/liveStatus")
            .and_then(serde_json::Value::as_str)
            == Some("success")
        {
            let room = at(&info, "/liveStream/roomData/roomInfo")?;
            let title = optional(room, "/roomTitle");
            if !title.is_empty() && !title.contains("回放") {
                let link = field(room, "/deeplink")?;
                let flv = query(&link, "flvUrl")?;
                let id = flv
                    .split("live/")
                    .nth(1)
                    .and_then(|v| v.split('.').next())
                    .ok_or("小红书直播流 ID 缺失")?;
                data.anchor_name = Some(query(&link, "host_nickname")?);
                data.is_live = true;
                data.title = Some(title);
                data.urls(
                    format!("http://live-source-play.xhscdn.com/live/{id}.m3u8"),
                    format!("http://live-source-play.xhscdn.com/live/{id}.flv"),
                    true,
                );
                return Ok(data);
            }
        }
    }
    let html = ctx
        .get(
            &format!("https://www.xiaohongshu.com/user/profile/{}", encode(&id)),
            &headers,
        )
        .await?;
    data.anchor_name = Some(
        capture(r"<title>@(.*?) 的个人主页</title>", &html)
            .map_err(|_| "小红书页面缺少主播数据".to_string())?,
    );
    Ok(data)
}
