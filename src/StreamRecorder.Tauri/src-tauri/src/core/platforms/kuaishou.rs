use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers=[("user-agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36 Edg/145.0.0.0"),("referer","https://live.kuaishou.com/profile/cym030000")];
    if input
        .cookies
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty())
    {
        let uid = input
            .live_url
            .strip_prefix("https://live.kuaishou.com/u/")
            .unwrap_or(&input.live_url);
        let info=ctx.json(&format!("https://live.kuaishou.com/live_api/baseuser/userinfo/byid?__NS_hxfalcon=&caver=2&principalId={}",encode(uid)),&headers,Body::None).await?;
        let user = at(&info, "/data/userInfo")?;
        let name = optional(user, "/name");
        if info
            .pointer("/data/result")
            .and_then(serde_json::Value::as_i64)
            == Some(2)
            && !name.is_empty()
        {
            return Err("快手账号异常或触发风控".into());
        }
        if !truth(at(user, "/living")?) {
            return Ok(StreamData::new(input, "快手直播", name, false));
        }
    }
    let html = ctx.get(&input.live_url, &headers).await?;
    let raw = capture(
        r"<script>window.__INITIAL_STATE__=(.*?);\(function\(\)\{var s;",
        &html,
    )?;
    let info = parse_json(&format!(
        "{}}}",
        capture(r#"(\{"liveStream".*?),"gameInfo"#, &raw)?
    ))?;
    if info.get("errorType").is_some() {
        return Err(format!(
            "快手风控：{}{}",
            optional(&info, "/errorType/title"),
            optional(&info, "/errorType/content")
        ));
    }
    let live = at(&info, "/liveStream")?;
    if !truth(live) {
        return Err("快手 IP 已受限，请检查网络".into());
    }
    let mut data = StreamData::new(input, "快手直播", optional(&info, "/author/name"), false);
    if let Some(play) = live.get("playUrls") {
        let representations = if let Some(h264) = play.get("h264") {
            if h264.get("adaptationSet").is_none() {
                return Ok(data);
            }
            at(h264, "/adaptationSet/representation")?
        } else {
            at(play, "/0/adaptationSet/representation")?
        };
        let mut streams = array(representations)?.iter().collect::<Vec<_>>();
        if streams.is_empty() {
            return Err("快手没有可用画质".into());
        }
        let selected = if streams[0].get("bitrate").is_some() {
            streams.sort_by_key(|v| {
                std::cmp::Reverse(
                    v.get("bitrate")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                )
            });
            let target = match input.quality.to_ascii_uppercase().as_str() {
                "BD" => 4000,
                "UHD" => 2000,
                "HD" => 1000,
                "SD" => 800,
                "LD" => 600,
                _ => 99999,
            };
            streams
                .iter()
                .find(|v| {
                    v.get("bitrate")
                        .and_then(serde_json::Value::as_u64)
                        .is_some_and(|b| b <= target)
                })
                .copied()
                .unwrap_or(*streams.last().unwrap())
        } else {
            streams.reverse();
            *pick(&streams, &input.quality)?
        };
        data.is_live = true;
        data.urls(String::new(), field(selected, "/url")?, true);
    }
    Ok(data)
}
