use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let cookie=input.cookies.as_deref().filter(|s|!s.is_empty()).unwrap_or("ttwid=1%7Ctzsy_yRGZ2N8AI7luDz2s9H9a8CQI3ZisibOcuw5OHs%7C1761301927%7C484a25162facd523ee6e2997187c9fad4a512c031d9efc6eaddb2c4bae8ce3fb");
    let headers=[("referer","https://www.tiktok.com/"),("user-agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36 Edg/141.0.0.0"),("cookie",cookie)];
    match web(ctx, input, &headers)
        .await
        .and_then(|v| parse(&v, input))
    {
        Ok((data, streams)) => finish(ctx, input, data, streams, &headers).await,
        Err(web_error) => {
            let uid = capture(r"https://www.tiktok.com/@(.*?)/live", &input.live_url)?;
            let info=ctx.json(&format!("https://www.tiktok.com/api-live/user/room?aid=1988&app_language=en&os=android&referer={}&sourceType=54&uniqueId={}",encode("https://www.tiktok.com/"),encode(&uid)),&headers,Body::None).await.map_err(|e|format!("web: {web_error}; app: {e}"))?;
            let (data, streams) =
                parse(&info, input).map_err(|e| format!("web: {web_error}; app: {e}"))?;
            finish(ctx, input, data, streams, &headers).await
        }
    }
}
async fn web(
    ctx: &ProbeContext,
    input: &ProbeInput,
    headers: &[(&str, &str)],
) -> Result<serde_json::Value, String> {
    let html = ctx.get(&input.live_url, headers).await?;
    if html.contains("We regret to inform you that we have discontinued operating TikTok") {
        return Err("当前网络地区无法访问 TikTok，请检查代理地区".into());
    }
    parse_json(&capture(
        r#"(?s)<script id="SIGI_STATE" type="application/json">(.*?)</script>"#,
        &html,
    )?)
}
struct Variant {
    hls: String,
    flv: String,
    bitrate: u64,
    width: u64,
    height: u64,
}
fn parse(
    info: &serde_json::Value,
    input: &ProbeInput,
) -> Result<(StreamData, Vec<Variant>), String> {
    let room = if info.get("LiveRoom").is_some() {
        at(info, "/LiveRoom/liveRoomUserInfo")?
    } else if info.get("data").is_some() {
        at(info, "/data")?
    } else {
        return Err("TikTok 房间数据缺失".into());
    };
    let user = at(room, "/user")?;
    let name = format!(
        "{}-{}",
        field(user, "/nickname")?,
        field(user, "/uniqueId")?
    );
    let mut data = StreamData::new(
        input,
        "TikTok",
        name,
        user.get("status").and_then(serde_json::Value::as_i64) == Some(2),
    );
    let mut streams = vec![];
    if data.is_live {
        let raw = field(room, "/liveRoom/streamData/pull_data/stream_data")
            .map_err(|_| "TikTok 直播需要登录确认年龄".to_string())?;
        let raw = parse_json(&raw)?;
        data.title = Some(field(room, "/liveRoom/title")?);
        for stream in object(at(&raw, "/data")?)?.values() {
            let main = at(stream, "/main")?;
            let sdk = parse_json(&field(main, "/sdk_params")?)?;
            let bitrate = field(&sdk, "/vbitrate")?
                .parse::<u64>()
                .map_err(|_| "TikTok 码率无效".to_string())?;
            let resolution = field(&sdk, "/resolution")?;
            if bitrate == 0 || resolution.is_empty() {
                continue;
            }
            let (width, height) = resolution.split_once('x').ok_or("TikTok 分辨率无效")?;
            let codec = optional(&sdk, "/VCodec");
            let append = |url: String| {
                if url.is_empty() {
                    url
                } else {
                    let separator = if url.ends_with(".flv") || url.ends_with(".m3u8") {
                        '?'
                    } else {
                        '&'
                    };
                    format!("{url}{separator}codec={codec}")
                }
            };
            streams.push(Variant {
                hls: append(optional(main, "/hls")),
                flv: append(optional(main, "/flv")),
                bitrate,
                width: width.parse().map_err(|_| "TikTok 分辨率无效".to_string())?,
                height: height
                    .parse()
                    .map_err(|_| "TikTok 分辨率无效".to_string())?,
            });
        }
        streams.sort_by_key(|v| std::cmp::Reverse((v.bitrate, v.width, v.height)));
    }
    Ok((data, streams))
}
async fn finish(
    ctx: &ProbeContext,
    input: &ProbeInput,
    mut data: StreamData,
    streams: Vec<Variant>,
    headers: &[(&str, &str)],
) -> Result<StreamData, String> {
    if !data.is_live {
        return Ok(data);
    }
    let mut index = quality_index(&input.quality);
    let selected = pick(&streams, &input.quality)?;
    let check = if selected.hls.is_empty() {
        &selected.flv
    } else {
        &selected.hls
    };
    if ctx
        .request(reqwest::Method::HEAD, check, headers, Body::None)
        .await
        .is_err()
    {
        index = if index < 4 { index + 1 } else { index - 1 };
    }
    let selected = streams
        .get(index.min(streams.len().saturating_sub(1)))
        .ok_or("TikTok 没有可用画质")?;
    data.urls(
        selected.hls.replace("https://", "http://"),
        selected.flv.replace("https://", "http://"),
        false,
    );
    Ok(data)
}
