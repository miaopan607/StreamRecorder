use crate::core::probe::*;
use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::sync::Semaphore;
static EXECUTORS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(4)));
static WASM_BYTES: LazyLock<tokio::sync::Mutex<std::collections::HashMap<String, Arc<Vec<u8>>>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(std::collections::HashMap::new()));
async fn wasm_bytes(ctx: &ProbeContext, version: &str) -> Result<Arc<Vec<u8>>, String> {
    let mut cache = tokio::select! {_=ctx.cancel.cancelled()=>return Err("探测已取消".into()),cache=WASM_BYTES.lock()=>cache};
    if let Some(bytes) = cache.get(version) {
        return Ok(bytes.clone());
    }
    let bytes = Arc::new(
        ctx.request_raw(
            reqwest::Method::GET,
            &format!(
                "https://www.miguvideo.com/mgs/player/prd/{}/dist/mgprtcl.wasm",
                encode(version)
            ),
            &[],
            Body::None,
        )
        .await?
        .bytes,
    );
    if cache.len() >= 4 {
        cache.clear();
    }
    cache.insert(version.into(), bytes.clone());
    Ok(bytes)
}
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers = [
        ("origin", "https://www.miguvideo.com"),
        ("referer", "https://www.miguvideo.com/"),
        ("appCode", "miguvideo_default_www"),
        ("appId", "miguvideo"),
        ("channel", "H5"),
    ];
    let id = room(&input.live_url)?;
    let info=ctx.json(&format!("https://vms-sc.miguvideo.com/vms-match/v6/staticcache/basic/basic-data/{}/miguvideo",encode(&id)),&headers,Body::None).await?;
    let name = field(&info, "/body/title")?;
    let mut data = StreamData::new(input, "咪咕直播", name.clone(), false);
    let room_id = optional(&info, "/body/pId");
    if room_id.is_empty() {
        return Ok(data);
    }
    let title = format!("{name}-{}", optional(&info, "/body/detailPageTitle"));
    let api=format!("https://webapi.miguvideo.com/gateway/playurl/v3/play/playurl?contId={}&rateType=3&clientId={}&timestamp={}&flvEnable=true&xh265=false&chip=mgwww&channelId=",encode(&room_id),uuid::Uuid::new_v4(),chrono::Utc::now().timestamp_millis());
    let info = ctx.json(&api, &headers, Body::None).await?;
    if field(&info, "/body/content/currentLive")? != "1" {
        return Ok(data);
    }
    let mut url = field(&info, "/body/urlInfo/url")?;
    // 当前 SDK 对没有 puData 的普通直链不执行 WASM；不制造空 ddCalcu。
    if query(&url, "puData").is_ok() {
        let settings = ctx
            .json(
                "https://app-sc.miguvideo.com/common/v1/settings/H5_DetailPage",
                &[],
                Body::None,
            )
            .await?;
        let settings = parse_json(&field(&settings, "/body/paramValue")?)?;
        let version = field(&settings, "/playerVersion")?;
        let bytes = wasm_bytes(ctx, &version).await?;
        // 已核实当前 SDK 的公开缺省因子；旧 ABI 沿用固定 streamget 因子。
        let modern = super::migu_wasm::is_modern(&bytes)?;
        let (factor, sv) = if modern {
            let factor=ctx.get("https://webapi.miguvideo.com/gateway/app-management/videox/staticcache/v2/factor/miguvideo/www",&headers).await;
            match factor
                .and_then(|body| parse_json(&body))
                .and_then(|info| Ok((field(&info, "/body/factor")?, field(&info, "/body/sv")?)))
            {
                Ok(value) => value,
                Err(_) => ("BjfS7eNf3OIROs2T1E8hHQ==".into(), "119".into()),
            }
        } else {
            ("PBTxuWiTEbUPPFcpyxs0ww==".into(), "10010".into())
        };
        let permit = tokio::select! {_=ctx.cancel.cancelled()=>return Err("探测已取消".into()),permit=EXECUTORS.clone().acquire_owned()=>permit.map_err(|e|e.to_string())?};
        let source = url.clone();
        let origin = input.live_url.clone();
        let cancel = ctx.cancel.child_token();
        let interrupted = cancel.clone();
        let signing = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            if interrupted.is_cancelled() {
                return Err("探测已取消".into());
            }
            super::migu_wasm::calculate(&version, &bytes, &source, &factor, &origin, interrupted)
        });
        let signature = tokio::select! {
            _=ctx.cancel.cancelled()=>{cancel.cancel();return Err("探测已取消".into())},
            result=tokio::time::timeout(Duration::from_secs(5),signing)=>match result{
                Ok(result)=>result.map_err(|e|format!("咪咕签名任务失败：{e}"))??,
                Err(_)=>{cancel.cancel();return Err("咪咕签名计算超时".into())}
            }
        };
        url = format!("{url}&ddCalcu={}&sv={}", encode(&signature), encode(&sv));
    }
    data.is_live = true;
    data.title = Some(title);
    if url.contains(".m3u8") {
        url = ctx
            .request(reqwest::Method::GET, &url, &headers, Body::None)
            .await?
            .final_url;
        data.urls(url, String::new(), false);
    } else {
        data.urls(String::new(), url, true);
    }
    Ok(data)
}
