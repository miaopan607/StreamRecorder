use crate::core::probe::*;
use serde_json::json;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers = [
        (
            "user-agent",
            "ios/7.830 (ios 17.0; ; iPhone 15 (A2846/A3089/A3090/A3092))",
        ),
        ("origin", "https://lives.jd.com"),
        ("referer", "https://lives.jd.com/"),
        ("x-referer-page", "https://lives.jd.com/"),
    ];
    let redirected = ctx
        .request(reqwest::Method::GET, &input.live_url, &headers, Body::None)
        .await?
        .final_url;
    let author = query(&redirected, "authorId").ok();
    let mut data = StreamData::new(input, "京东直播", String::new(), false);
    let id = if let Some(author) = author.as_deref() {
        let body = json!({"authorId":author,"monitorSource":"1","userId":""}).to_string();
        let info = ctx
            .json(
                "https://api.m.jd.com/talent_head_findTalentMsg",
                &headers,
                form(&[
                    ("functionId", "talent_head_findTalentMsg"),
                    ("appid", "dr_detail"),
                    ("body", &body),
                ]),
            )
            .await?;
        data.anchor_name = Some(field(&info, "/result/talentName")?);
        if info.pointer("/result/livingRoomJump").is_none() {
            return Ok(data);
        }
        field(&info, "/result/livingRoomJump/params/id")?
    } else {
        let id = match capture(r"#/(.*?)\?origin", &redirected) {
            Ok(id) => id,
            Err(_) => return Ok(data),
        };
        data.anchor_name = Some(format!("jd_{id}"));
        id
    };
    let body = json!({"liveId":id}).to_string();
    let info=ctx.json(&format!("https://api.m.jd.com/client.action?body={}&functionId=getImmediatePlayToM&appid=h5-live",encode(&body)),&headers,Body::None).await?;
    data.is_live = at(&info, "/data/status")?.as_i64() == Some(1);
    if data.is_live {
        if let Some(author) = author {
            let body=json!({"authorId":author,"type":1,"userId":"","page":1,"offset":"-1","monitorSource":"1","pageSize":1}).to_string();
            let title = ctx
                .json(
                    "https://api.m.jd.com/jdTalentContentList",
                    &headers,
                    form(&[
                        ("functionId", "jdTalentContentList"),
                        ("appid", "dr_detail"),
                        ("body", &body),
                    ]),
                )
                .await?;
            data.title = Some(field(&title, "/result/content/0/title")?);
        }
        data.urls(
            field(&info, "/data/h5VideoUrl")?,
            field(&info, "/data/videoUrl")?,
            false,
        );
    }
    Ok(data)
}
