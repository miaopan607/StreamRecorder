use crate::core::probe::*;
use serde_json::json;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let uid = room(&input.live_url)?;
    let device = uuid::Uuid::new_v4().simple().to_string()[..16].to_owned();
    let headers = [
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0",
        ),
        ("accept-language", "en-US"),
        ("referer", "https://www.twitch.tv/"),
        ("client-id", "kimne78kx3ncx6brgo4mv6wki5h1ko"),
        ("client-integrity", ""),
        ("content-type", "text/plain;charset=UTF-8"),
        ("device-id", device.as_str()),
        ("cookie", input.cookies.as_deref().unwrap_or("")),
    ];
    let token=ctx.json("https://gql.twitch.tv/gql",&headers,Body::Json(json!({"operationName":"PlaybackAccessToken_Template","query":PLAY_QUERY,"variables":{"isLive":true,"login":uid,"isVod":false,"vodID":"","playerType":"site"}}))).await?;
    let token = at(&token, "/data/streamPlaybackAccessToken")?;
    if token
        .pointer("/authorization/isForbidden")
        .is_some_and(truth)
    {
        return Err("Twitch 播放授权被拒绝".into());
    }
    let info=ctx.json("https://gql.twitch.tv/gql",&headers,Body::Json(json!([{"operationName":"ComscoreStreamingQuery","variables":{"channel":uid.to_lowercase(),"clipSlug":"","isClip":false,"isLive":true,"isVodOrCollection":false,"vodID":""},"extensions":{"persistedQuery":{"version":1,"sha256Hash":"e1edae8122517d013405f237ffcc124515dc6ded82480a88daef69c83b53ac01"}}}]))).await?;
    let user = at(&info, "/0/data/user")?;
    let mut data = StreamData::new(
        input,
        "Twitch",
        format!("{}-{uid}", field(user, "/displayName")?),
        user.get("stream").is_some_and(truth),
    );
    data.title = Some(field(user, "/broadcastSettings/title")?);
    if !data.is_live {
        return Ok(data);
    }
    let master=format!("https://usher.ttvnw.net/api/channel/hls/{}.m3u8?acmb=e30%3D&allow_audio_only=true&allow_source=true&browser_family=firefox&browser_version=124.0&cdm=wv&fast_bread=true&os_name=Windows&os_version=NT%252010.0&p=3553732&platform=web&play_session_id=bdd22331a986c7f1073628f2fc5b19da&player_backend=mediaplayer&player_version=1.28.0-rc.1&playlist_include_framerate=true&reassignments_supported=true&sig={}&token={}&transcode_mode=cbr_v1",encode(&uid),encode(&field(token,"/signature")?),encode(&field(token,"/value")?));
    let text = ctx.get(&master, &headers).await?;
    let mut streams = vec![];
    let mut group = String::new();
    let mut pending = None;
    for line in text.lines().map(str::trim) {
        if line.starts_with("#EXT-X-MEDIA:") {
            group = capture(r#"GROUP-ID="([^"]+)""#, line).unwrap_or_default();
        } else if line.starts_with("#EXT-X-STREAM-INF:") {
            pending = Some(
                capture(r"BANDWIDTH=(\d+)", line)?
                    .parse::<u64>()
                    .map_err(|_| "Twitch 码率无效".to_string())?,
            );
        } else if !line.starts_with('#') && !line.is_empty() {
            if let Some(rate) = pending.take() {
                let url = url::Url::parse(&master)
                    .map_err(|e| e.to_string())?
                    .join(line)
                    .map_err(|e| e.to_string())?
                    .to_string();
                streams.push((rate, url, group == "audio_only"));
                group.clear();
            }
        }
    }
    streams.sort_by_key(|v| std::cmp::Reverse(v.0));
    if streams.is_empty() {
        data.urls(master, String::new(), false);
        return Ok(data);
    }
    let selected = if input.quality == "AD" {
        streams
            .iter()
            .find(|v| v.2)
            .unwrap_or(pick(&streams, &input.quality)?)
    } else {
        pick(&streams, &input.quality)?
    };
    data.urls(selected.1.clone(), String::new(), false);
    data.m3u8_url = Some(master);
    if input.quality == "AD" && selected.2 {
        data.extra = Some(json!({"bandwidth":selected.0,"is_audio_only":true}));
    }
    Ok(data)
}

const PLAY_QUERY:&str="query PlaybackAccessToken_Template($login: String!, $isLive: Boolean!, $vodID: ID!, $isVod: Boolean!, $playerType: String!) {  streamPlaybackAccessToken(channelName: $login, params: {platform: \"web\", playerBackend: \"mediaplayer\", playerType: $playerType}) @include(if: $isLive) {    value    signature   authorization { isForbidden forbiddenReasonCode }   __typename  }  videoPlaybackAccessToken(id: $vodID, params: {platform: \"web\", playerBackend: \"mediaplayer\", playerType: $playerType}) @include(if: $isVod) {    value    signature   __typename  }}";
