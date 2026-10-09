use crate::core::probe::*;
use crate::core::signing;
use serde_json::json;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers = [
        ("origin", "https://www.liveme.com"),
        ("referer", "https://www.liveme.com"),
        (
            "user-agent",
            "ios/7.830 (ios 17.0; ; iPhone 15 (A2846/A3089/A3090/A3092))",
        ),
    ];
    let mut url = input.live_url.clone();
    if !url.contains("index.html") {
        let html = ctx.get(&url, &headers).await?;
        if let Ok(redirect) = capture(r#"<meta property="og:url" content="(.*?)">"#, &html) {
            url = redirect;
        }
    }
    let id = url
        .split("/index.html")
        .next()
        .unwrap()
        .rsplit('/')
        .next()
        .ok_or("LiveMe 房间 ID 缺失")?;
    let signed = signing::sign(
        "liveme",
        vec![json!(id), json!("crypto-js.min.js")],
        ctx.cancel.clone(),
    )
    .await?;
    let signature = field(&signed, "/lm_s_sign")?;
    let black = field(&signed, "/tongdun_black_box")?;
    let os = field(&signed, "/os")?;
    let fields = object(&signed)?
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "lm_s_sign" | "tongdun_black_box" | "os"))
        .map(|(k, v)| (k.clone(), text(v)))
        .collect();
    let mut headers = headers.to_vec();
    headers.push(("lm-s-sign", &signature));
    let info=ctx.json(&format!("https://live.liveme.com/live/queryinfosimple?alias=liveme&tongdun_black_box={}&os={}",encode(&black),encode(&os)),&headers,Body::Form(fields)).await?;
    let info = at(&info, "/data/video_info")?;
    let mut data = StreamData::new(
        input,
        "Liveme",
        field(info, "/uname")?,
        field(info, "/status")? == "0",
    );
    data.live_url = Some(url);
    if data.is_live {
        data.urls(
            field(info, "/hlsvideosource")?,
            field(info, "/videosource")?,
            false,
        );
    }
    Ok(data)
}
