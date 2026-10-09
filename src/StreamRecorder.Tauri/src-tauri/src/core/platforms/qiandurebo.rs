use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    probe_with_platform(ctx, input, "千度热播直播").await
}
pub(super) async fn probe_with_platform(
    ctx: &ProbeContext,
    input: &ProbeInput,
    platform: &str,
) -> Result<StreamData, String> {
    let html = ctx.get(&input.live_url, &[]).await?;
    let raw = capture(r"(?s)var user = (.*?)\r\n\s+user\.play_url", &html)?;
    let names = captures(r#""zb_nickname": "(.*?)",\r\n"#, &raw)?;
    let urls = captures(r#""play_url": "(.*?)",\r\n"#, &raw)?;
    let live = !names.is_empty()
        && !urls.is_empty()
        && !html.contains("common-text-center\" style=\"display:block");
    let mut data = StreamData::new(
        input,
        platform,
        names.first().cloned().unwrap_or_default(),
        live,
    );
    if live {
        data.urls(String::new(), urls[0].clone(), true);
    }
    Ok(data)
}
