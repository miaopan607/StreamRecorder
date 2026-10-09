use crate::core::probe::*;
use crate::core::signing;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let time = chrono::Utc::now().timestamp_millis().to_string();
    let imei = uuid::Uuid::new_v4().simple().to_string();
    let signature = signing::md5(format!(
        "web{imei}{time}kk792f28d6ff1f34ec702c08626d454b39pro"
    ));
    let headers=[("user-agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36 Edg/138.0.0.0"),("mobileModel","web"),("timestamp",time.as_str()),("loginType","2"),("versionCode","10003"),("imei",imei.as_str()),("requestId",signature.as_str()),("channel","9"),("version","1.0.0"),("os","web"),("platform","WEB"),("origin","https://www.imkktv.com"),("referer","https://www.imkktv.com/")];
    let id = query(&input.live_url, "roomId").or_else(|_| query(&input.live_url, "anchorId"))?;
    let info = ctx
        .json(
            &format!(
                "https://api.imkktv.com/liveroom/getShareLiveVideo?roomId={}",
                encode(&id)
            ),
            &headers,
            Body::None,
        )
        .await?;
    let room = at(&info, "/data")?;
    let mut data = StreamData::new(
        input,
        "来秀直播",
        field(room, "/nickname")?,
        at(room, "/playStatus")?.as_i64() == Some(0),
    );
    if data.is_live {
        data.urls(String::new(), field(room, "/playUrl")?, true);
    }
    Ok(data)
}
