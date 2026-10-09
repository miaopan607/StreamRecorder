use crate::core::probe::*;
use rand::Rng;
use serde_json::json;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = query(&input.live_url, "room_id")?;
    let uid = [
        "h5-683e85bdf741bf2492586f7ca39bf465",
        "h5-c7c6dc14064a136be4215b452fab9eea",
        "h5-4581281f80bb8968bd9a9dfba6050d3a",
    ][rand::thread_rng().gen_range(0..3)];
    let request=json!({"data":{"room_id":id,"device_id":"h5-683e85bdf741bf2492586f7ca39bf465","source_type":0,"osname":"baiduboxapp"},"replay_slice":0,"nid":"","schemeParams":{"src_pre":"pc","src_suf":"other","bd_vid":"","share_uid":"","share_cuk":"","share_ecid":"","zb_tag":"","shareTaskInfo":"{\"room_id\":\"9175031377\"}","share_from":"","ext_params":"","nid":""}}).to_string();
    let info=ctx.json(&format!("https://mbd.baidu.com/searchbox?cmd=371&action=star&service=bdbox&osname=baiduboxapp&data={}&ua=360_740_ANDROID_0&bd_vid=&uid={}&_={}",encode(&request),encode(uid),chrono::Utc::now().timestamp_millis()),&[],Body::None).await?;
    let info = object(at(&info, "/data")?)?
        .values()
        .next()
        .ok_or("百度直播数据为空")?;
    let mut data = StreamData::new(
        input,
        "百度",
        field(info, "/host/name")?,
        field(info, "/status")? == "0",
    );
    if data.is_live {
        data.title = Some(field(info, "/video/title")?);
        let mut urls = vec![];
        let clarity = array(at(info, "/video/url_clarity_list")?)?;
        if !clarity.is_empty() {
            for stream in clarity {
                let flv = field(stream, "/urls/flv")?;
                let stem = flv
                    .rsplit('/')
                    .next()
                    .unwrap()
                    .rsplit_once('.')
                    .map(|(s, _)| s)
                    .ok_or("百度流地址无效")?;
                urls.push(format!(
                    "https://hls.liveshow.bdstatic.com/live/{stem}.m3u8"
                ));
            }
        } else {
            for stream in array(at(info, "/video/url_list")?)? {
                let hls = field(stream, "/urls/0/hls")?;
                let name = hls.split('?').next().unwrap().rsplit('/').next().unwrap();
                urls.push(format!("https://hls.liveshow.bdstatic.com/live/{name}"));
            }
        }
        data.urls(pick(&urls, &input.quality)?.clone(), String::new(), false);
    }
    Ok(data)
}
