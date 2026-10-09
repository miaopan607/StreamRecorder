use crate::core::probe::*;
use crate::core::signing;
use serde_json::{json, Value};
const UA:&str="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36";
const DEFAULT_COOKIE:&str="ttwid=1%7CmDcInbJ7AJ-2PGtsgrG4xj7SOiNMzePqQBF1LMO2Qkg%7C1761107324%7Cbbf97c2cd9f8eae8e8c36db4ef50c323deaa4b161179170aaf659590867c162d";
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    match web(ctx, input, &room(&input.live_url)?).await {
        Ok(data) => Ok(data),
        Err(web_error) => app(ctx, input)
            .await
            .map_err(|app_error| format!("web: {web_error}; app: {app_error}")),
    }
}
async fn web(ctx: &ProbeContext, input: &ProbeInput, id: &str) -> Result<StreamData, String> {
    let cookie = input
        .cookies
        .as_deref()
        .filter(|v| v.contains("ttwid="))
        .unwrap_or(DEFAULT_COOKIE);
    let headers = [
        ("referer", "https://live.douyin.com/335354047186"),
        ("user-agent", UA),
        ("cookie", cookie),
    ];
    let query=format!("aid=6383&app_name=douyin_web&live_id=1&device_platform=web&language=zh-CN&browser_language=zh-CN&browser_platform=Win32&browser_name=Chrome&browser_version=116.0.0.0&web_rid={}&is_need_double_stream=false&msToken=",encode(id));
    let signature = signing::douyin_ab(&query, UA, chrono::Utc::now().timestamp_millis() as u64);
    let info = ctx
        .json(
            &format!("https://live.douyin.com/webcast/room/web/enter/?{query}&a_bogus={signature}"),
            &headers,
            Body::None,
        )
        .await?;
    let root = at(&info, "/data")?;
    let room = at(root, "/data/0").map_err(|_| {
        let message = optional(root, "/prompts");
        if message.is_empty() {
            "抖音风控或不支持的 VR 直播".into()
        } else {
            message
        }
    })?;
    convert(
        ctx,
        input,
        room,
        field(root, "/user/nickname")?,
        format!("https://live.douyin.com/{id}"),
        &headers,
    )
    .await
}
async fn app(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers = [
        ("user-agent", MOBILE_UA),
        (
            "cookie",
            input
                .cookies
                .as_deref()
                .unwrap_or("s_v_web_id=verify_lk07kv74_QZYCUApD_xhiB_405x_Ax51_GYO9bUIyZQVf"),
        ),
    ];
    let redirected = ctx
        .request(reqwest::Method::GET, &input.live_url, &headers, Body::None)
        .await?
        .final_url;
    if !redirected.contains("reflow/") {
        let uid = room(&redirected)?;
        let cookie="ttwid=1%7C4ejCkU2bKY76IySQENJwvGhg1IQZrgGEupSyTKKfuyk%7C1740470403%7Cbc9ad2ee341f1a162f9e27f4641778030d1ae91e31f9df6553a8f2efa3bdb7b4; __ac_nonce=0683e59f3009cc48fbab0; __ac_signature=_02B4Z6wo00f01mG6waQAAIDB9JUCzFb6.TZhmsUAAPBf34; __ac_referer=__ac_blank";
        let html = ctx
            .get(
                &format!("https://www.iesdouyin.com/share/user/{}", encode(&uid)),
                &[("user-agent", MOBILE_UA), ("cookie", cookie)],
            )
            .await?;
        let ids = captures(r#"unique_id":"(.*?)","verification_type"#, &html)?;
        return web(ctx, input, ids.last().ok_or("抖音用户主页缺少直播 ID")?).await;
    }
    let id = room(&redirected)?;
    let user = query(&redirected, "sec_user_id")?;
    let query=format!("verifyFp=verify_lxj5zv70_7szNlAB7_pxNY_48Vh_ALKF_GA1Uf3yteoOY&type_id=0&live_id=1&room_id={}&sec_user_id={}&version_code=99.99.99&app_id=1128&is_need_double_stream=True",encode(&id),encode(&user));
    let signature = signing::douyin_ab(
        &query,
        MOBILE_UA,
        chrono::Utc::now().timestamp_millis() as u64,
    );
    let info = ctx
        .json(
            &format!(
                "https://webcast.amemv.com/webcast/room/reflow/info/?{query}&a_bogus={signature}"
            ),
            &headers,
            Body::None,
        )
        .await?;
    let room = at(&info, "/data/room")?;
    let web_id = optional(room, "/owner/web_rid");
    convert(
        ctx,
        input,
        room,
        field(room, "/owner/nickname")?,
        if web_id.is_empty() {
            input.live_url.clone()
        } else {
            format!("https://live.douyin.com/{web_id}")
        },
        &headers,
    )
    .await
}
async fn convert(
    ctx: &ProbeContext,
    input: &ProbeInput,
    room: &Value,
    name: String,
    live_url: String,
    headers: &[(&str, &str)],
) -> Result<StreamData, String> {
    let mut data = StreamData::new(
        input,
        "抖音",
        name,
        room.get("status").and_then(Value::as_i64) == Some(2),
    );
    data.live_url = Some(live_url);
    data.extra = Some(json!({"stream_orientation":1}));
    if !data.is_live {
        return Ok(data);
    }
    let stream = at(room, "/stream_url")?;
    let core = parse_json(&field(stream, "/live_core_sdk_data/pull_data/stream_data")?)?;
    let origin = at(&core, "/data/origin/main")?;
    let sdk = at(origin, "/sdk_params")?;
    let codec = if let Some(s) = sdk.as_str() {
        optional(&parse_json(s)?, "/VCodec")
    } else {
        optional(sdk, "/VCodec")
    };
    let hls = ordered_urls(
        at(stream, "/hls_pull_url_map")?,
        format!("{}&codec={codec}", field(origin, "/hls")?),
    )?;
    let flv = ordered_urls(
        at(stream, "/flv_pull_url")?,
        format!("{}&codec={codec}", field(origin, "/flv")?),
    )?;
    let mut index = quality_index(&input.quality);
    let selected = pick(&hls, &input.quality)?;
    if ctx
        .request(reqwest::Method::HEAD, selected, headers, Body::None)
        .await
        .is_err()
    {
        index = if index < 4 { index + 1 } else { index - 1 };
    }
    let hls = hls
        .get(index.min(hls.len().saturating_sub(1)))
        .cloned()
        .ok_or("抖音没有可用 HLS 画质")?;
    let flv = flv
        .get(index.min(flv.len().saturating_sub(1)))
        .cloned()
        .ok_or("抖音没有可用 FLV 画质")?;
    data.title = Some(optional(room, "/title"));
    data.urls(hls, flv, false);
    Ok(data)
}
fn ordered_urls(map: &Value, origin: String) -> Result<Vec<String>, String> {
    let map = object(map)?;
    let mut result = Vec::with_capacity(map.len() + 1);
    result.push(map.get("ORIGIN").map(text).unwrap_or(origin));
    result.extend(
        map.iter()
            .filter(|(key, _)| key.as_str() != "ORIGIN")
            .map(|(_, value)| text(value)),
    );
    Ok(result)
}
