use crate::core::probe::*;
use serde_json::json;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers=[("user-agent","ShopeeSG/3.68.33 (com.beeasy.shopee.sg; build:3.68.33; iOS 17.0.0) Alamofire/5.0.5 appver=36833 language=en app_type=1 platform=native_ios os_ver=17.0.0 Cronet/102.0.5005.61"),("content-type","application/json")];
    let url = ctx
        .request(reqwest::Method::GET, &input.live_url, &headers, Body::None)
        .await?
        .final_url;
    let session = query(&url, "session")?
        .parse::<u64>()
        .map_err(|_| "Shopee session ID 无效".to_string())?;
    let extra=json!({"url_history":{"first_frame_succ_cost":[],"connection_succ_cost":[]},"client_speed_version":"1.0","client_speed_result":{"bitrates":[],"status_times":[]},"is_first_session":true}).to_string();
    let info = ctx
        .json(
            "https://live.shopee.sg/api/v1/play_param/session",
            &headers,
            Body::Json(json!({"extra":extra,"quality_level_id":0,"session_ids":[session]})),
        )
        .await?;
    let rooms = array(at(&info, "/data/play_param_list")?)?;
    let Some(room) = rooms.first() else {
        return Ok(StreamData::new(input, "Shopee", String::new(), false));
    };
    let name = format!(
        "{}_{}",
        field(room, "/session/nickname")?,
        field(room, "/session/username")?
    );
    let mut data = StreamData::new(input, "Shopee", name, true);
    let mpd = optional(room, "/play_param/las_param/mpd");
    let (flv, backups) = if mpd.is_empty() {
        let urls = array(at(room, "/play_param/play_url_list")?)?;
        (
            urls.first().map(text).ok_or("Shopee 没有直播流")?,
            json!(urls.iter().skip(1).collect::<Vec<_>>()),
        )
    } else {
        let info = parse_json(&mpd)?;
        let mut streams = array(at(&info, "/adaptationSet/0/representation")?)?
            .iter()
            .collect::<Vec<_>>();
        streams.sort_by_key(|s| {
            s.get("maxBitrate")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
        });
        let selected = pick(&streams, &input.quality)?;
        (
            field(selected, "/url")?,
            at(selected, "/backupUrl")?.clone(),
        )
    };
    data.urls(String::new(), flv, true);
    data.extra = Some(json!({"backup_url_list":backups}));
    Ok(data)
}
