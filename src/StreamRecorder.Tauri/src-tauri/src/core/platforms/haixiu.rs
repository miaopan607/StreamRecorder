use crate::core::probe::*;
use crate::core::signing;
use serde_json::json;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    probe_site(
        ctx,
        input,
        if input.live_url.contains("haixiutv") {
            "haixiutv"
        } else {
            "lehaitv"
        },
        "嗨秀直播",
    )
    .await
}
pub(super) async fn probe_site(
    ctx: &ProbeContext,
    input: &ProbeInput,
    site: &str,
    platform: &str,
) -> Result<StreamData, String> {
    let id = room(&input.live_url)?;
    let token = if site == "haixiutv" {
        "pLXSC%252FXJ0asc1I21tVL5FYZhNJn2Zg6d7m94umCnpgL%252BuVm31GQvyw%253D%253D"
    } else {
        "s7FUbTJ%252BjILrR7kicJUg8qr025ZVjd07DAnUQd8c7g%252Fo4OH9pdSX6w%253D%253D"
    };
    let time = chrono::Utc::now().timestamp_millis();
    let params = json!({"accessToken":token,"tku":"3000006","c":"10138100100000","_st1":time});
    let signature = signing::sign(
        "haixiu",
        vec![params, json!("crypto-js.min.js")],
        ctx.cancel.clone(),
    )
    .await?;
    let origin = format!("https://www.{site}.com");
    let referer = if site == "haixiutv" {
        format!("{origin}/")
    } else {
        origin.clone()
    };
    let headers = [
        ("origin", origin.as_str()),
        ("referer", referer.as_str()),
        (
            "user-agent",
            "ios/7.830 (ios 17.0; ; iPhone 15 (A2846/A3089/A3090/A3092))",
        ),
    ];
    let api=format!("https://service.{site}.com/v2/room/{}/media/advanceInfoRoom?accessToken={}&tku=3000006&c=10138100100000&_st1={time}&_ajaxData1={}&_={}",encode(&id),encode(&unquote(&unquote(token))),encode(&text(&signature)),chrono::Utc::now().timestamp_millis());
    let info = ctx.json(&api, &headers, Body::None).await?;
    let room = at(&info, "/data")?;
    let mut data = StreamData::new(
        input,
        platform,
        field(room, "/nickname")?,
        at(room, "/live_status")?.as_i64() == Some(1),
    );
    if data.is_live {
        data.urls(String::new(), field(room, "/media_url_web")?, true);
    }
    Ok(data)
}
