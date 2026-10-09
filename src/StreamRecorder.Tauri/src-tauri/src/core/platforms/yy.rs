use crate::core::probe::*;
use serde_json::json;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers = [(
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0",
    )];
    let html = ctx.get(&input.live_url, &headers).await?;
    let name = capture(r#"nick: "(.*?)",\n\s+logo"#, &html)?;
    let id = capture(r#"(?s)sid : "(.*?)",\n\s+ssid"#, &html)?;
    let request = json!({"head":{"seq":1701869217590u64,"appidstr":"0","bidstr":"121","cidstr":id,"sidstr":id,"uid64":0,"client_type":108,"client_ver":"5.17.0","stream_sys_ver":1,"app":"yylive_web","playersdk_ver":"5.17.0","thundersdk_ver":"0","streamsdk_ver":"5.17.0"},"client_attribute":{"client":"web","model":"web0","cpu":"","graphics_card":"","os":"chrome","osversion":"0","vsdk_version":"","app_identify":"","app_version":"","business":"","width":"1920","height":"1080","scale":"","client_type":8,"h265":0},"avp_parameter":{"version":1,"client_type":8,"service_type":0,"imsi":0,"send_time":1701869217,"line_seq":-1,"gear":4,"ssl":1,"stream_format":0}});
    let info=ctx.json(&format!("https://stream-manager.yy.com/v3/channel/streams?uid=0&cid={}&sid={}&appid=0&sequence=1701869217590&encode=json",encode(&id),encode(&id)),&headers,Body::Bytes(request.to_string().into_bytes())).await?;
    let title = ctx
        .json(
            &format!(
                "https://www.yy.com/live/detail?uid=&sid={}&ssid={}&_={}",
                encode(&id),
                encode(&id),
                chrono::Utc::now().timestamp_millis()
            ),
            &headers,
            Body::None,
        )
        .await?;
    let mut data = StreamData::new(input, "YY直播", name, info.get("avp_info_res").is_some());
    if data.is_live {
        let lines = object(at(&info, "/avp_info_res/stream_line_addr")?)?;
        let selected = lines.values().next().ok_or("YY 没有可用 CDN")?;
        data.title = Some(field(&title, "/data/roomName")?);
        data.quality = Some("OD".into());
        data.urls(String::new(), field(selected, "/cdn_info/url")?, true);
    }
    Ok(data)
}
