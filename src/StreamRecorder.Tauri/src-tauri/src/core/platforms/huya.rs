use crate::core::probe::*;
use crate::core::signing;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::Rng;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    match web(ctx, input).await {
        Ok(data) => Ok(data),
        Err(error) => app(ctx, input)
            .await
            .map_err(|app| format!("web: {error}; app: {app}")),
    }
}
async fn web(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let html = ctx.get(&input.live_url, &[]).await?;
    let info = parse_json(&format!(
        "{}}}",
        capture(r#"stream: (\{"data".*?),"iWebDefaultBitRate"#, &html)?
    ))?;
    let live = at(&info, "/data/0/gameLiveInfo")?;
    let streams = array(at(&info, "/data/0/gameStreamInfoList")?)?;
    let mut data = StreamData::new(
        input,
        "虎牙直播",
        optional(live, "/nick"),
        !streams.is_empty(),
    );
    if !data.is_live {
        return Ok(data);
    }
    data.title = Some(field(live, "/introduction")?);
    let stream = &streams[0];
    let name = field(stream, "/sStreamName")?;
    let anti = field(stream, "/sFlvAntiCode")?;
    let millis = chrono::Utc::now().timestamp() as u64 * 1000;
    let uid = rand::thread_rng().gen_range(1400000000000u64..=1400009999999);
    let random = rand::thread_rng().gen_range(0u64..1000);
    let suffix = anti_code(&anti, &name, millis, uid, random)?;
    let mut ratio = String::new();
    if !matches!(input.quality.to_ascii_uppercase().as_str(), "OD" | "BD") {
        if let Some(qualities) = anti.split("&exsphd=").nth(1) {
            let mut values = captures(r"264_(\d+)", qualities)?;
            values.reverse();
            let index = match input.quality.to_ascii_uppercase().as_str() {
                "UHD" => 0,
                "HD" => 1,
                "SD" => 2,
                "LD" => 3,
                _ => return Err("虎牙画质无效".into()),
            };
            ratio = values
                .get(index.min(values.len().saturating_sub(1)))
                .cloned()
                .ok_or("虎牙没有可用画质")?;
        }
    }
    let flv = format!(
        "{}/{}.{}?{suffix}&ratio={ratio}",
        field(stream, "/sFlvUrl")?,
        name,
        field(stream, "/sFlvUrlSuffix")?
    );
    let hls = format!(
        "{}/{}.{}?{suffix}&ratio={ratio}",
        field(stream, "/sHlsUrl")?,
        name,
        field(stream, "/sHlsUrlSuffix")?
    );
    data.urls(hls, flv, true);
    Ok(data)
}
pub(super) fn anti_code(
    old: &str,
    name: &str,
    millis: u64,
    uid: u64,
    random: u64,
) -> Result<String, String> {
    let parameters: url::Url = url::Url::parse(&format!("https://huya.test/?{old}"))
        .map_err(|_| "虎牙防盗链参数无效".to_string())?;
    let get = |key: &str| {
        parameters
            .query_pairs()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
            .ok_or_else(|| format!("虎牙防盗链缺少 {key}"))
    };
    let fm = String::from_utf8(
        STANDARD
            .decode(unquote(&get("fm")?))
            .map_err(|_| "虎牙防盗链 Base64 无效".to_string())?,
    )
    .map_err(|_| "虎牙防盗链编码无效".to_string())?;
    let prefix = fm.split('_').next().ok_or("虎牙签名前缀缺失")?;
    let ctype = get("ctype")?;
    let fs = get("fs")?;
    let sequence = uid + millis;
    let time = format!("{:x}", (millis + 110624) / 1000);
    let hash = signing::md5(format!("{sequence}|{ctype}|100"));
    let secret = signing::md5(format!("{prefix}_{uid}_{name}_{hash}_{time}"));
    let uuid = ((millis % 10000000000) * 1000 + random) % 4294967295;
    Ok(format!("wsSecret={secret}&wsTime={time}&seqid={sequence}&ctype={ctype}&ver=1&fs={fs}&uuid={uuid}&u={uid}&t=100&sv=2403051612&sdk_sid={millis}&codec=264"))
}
async fn app(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers = [
        (
            "user-agent",
            "ios/7.830 (ios 17.0; ; iPhone 15 (A2846/A3089/A3090/A3092))",
        ),
        ("xweb_xhr", "1"),
        (
            "referer",
            "https://servicewechat.com/wx74767bf0b684f7d3/301/page-frame.html",
        ),
    ];
    let mut id = room(&input.live_url)?;
    if id.chars().any(char::is_alphabetic) {
        let html = ctx.get(&input.live_url, &headers).await?;
        id = capture(r#"ProfileRoom":(.*?),"sPrivateHost"#, &html)
            .map_err(|_| "请使用虎牙数字房间地址".to_string())?;
    }
    let info = ctx
        .json(
            &format!(
                "https://mp.huya.com/cache.php?m=Live&do=profileRoom&roomid={}&showSecret=1",
                encode(&id)
            ),
            &[],
            Body::None,
        )
        .await?;
    let room = at(&info, "/data")?;
    let mut data = StreamData::new(
        input,
        "虎牙直播",
        field(room, "/profileInfo/nick")?,
        field(room, "/realLiveStatus")? == "ON",
    );
    data.live_url = Some(format!("https://www.huya.com/{id}"));
    if !data.is_live {
        return Ok(data);
    }
    if optional(room, "/liveData/gameHostName") == "lol" {
        let mut canonical = input.clone();
        canonical.live_url = data.live_url.unwrap();
        return web(ctx, &canonical).await;
    }
    data.title = Some(field(room, "/liveData/introduction")?);
    let streams = array(at(room, "/stream/baseSteamInfoList")?)?;
    let stream = streams
        .iter()
        .rev()
        .find(|v| v.get("sCdnType").and_then(serde_json::Value::as_str) == Some("TX"))
        .or_else(|| streams.first())
        .ok_or("虎牙没有可用 CDN")?;
    let name = field(stream, "/sStreamName")?;
    let mut hls = format!(
        "{}/{name}.m3u8?{}",
        field(stream, "/sHlsUrl")?,
        field(stream, "/sHlsAntiCode")?
    );
    let mut flv = format!(
        "{}/{name}.flv?{}",
        field(stream, "/sFlvUrl")?,
        field(stream, "/sFlvAntiCode")?
    );
    if matches!(optional(stream, "/sCdnType").as_str(), "TX" | "HW") {
        hls = hls
            .replace("&ctype=tars_mp", "&ctype=huya_webh5")
            .replace("&fs=bhct", "&fs=bgct");
        flv = flv
            .replace("&ctype=tars_mp", "&ctype=huya_webh5")
            .replace("&fs=bhct", "&fs=bgct");
    }
    data.urls(hls, flv, true);
    Ok(data)
}
