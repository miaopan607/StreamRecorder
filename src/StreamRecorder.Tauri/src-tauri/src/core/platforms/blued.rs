use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let html = ctx
        .get(&input.live_url, &[("user-agent", MOBILE_UA)])
        .await?;
    let raw = capture(
        r#"(?s)decodeURIComponent\("(.*?)"\)\),window\.Promise"#,
        &html,
    )?;
    let json = parse_json(&unquote(&raw))?;
    let mut data = StreamData::new(
        input,
        "Blued",
        field(&json, "/userInfo/name")?,
        truth(at(&json, "/userInfo/onLive")?),
    );
    if data.is_live {
        data.urls(field(&json, "/liveInfo/liveUrl")?, String::new(), false);
    }
    Ok(data)
}
