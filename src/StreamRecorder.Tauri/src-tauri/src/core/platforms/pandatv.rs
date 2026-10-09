use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    probe_site(ctx, input, "pandalive", "PandaTV", "media fanGrade").await
}
pub(super) async fn probe_site(
    ctx: &ProbeContext,
    input: &ProbeInput,
    site: &str,
    platform: &str,
    info: &str,
) -> Result<StreamData, String> {
    let id = room(&input.live_url)?;
    let origin = format!("https://www.{site}.co.kr");
    let referer = format!("{origin}/");
    let headers = [
        ("origin", origin.as_str()),
        ("referer", referer.as_str()),
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:124.0) Gecko/20100101 Firefox/124.0",
        ),
    ];
    let details = ctx
        .json(
            &format!("https://api.{site}.co.kr/v1/member/bj"),
            &headers,
            form(&[("userId", &id), ("info", info)]),
        )
        .await?;
    let name = format!(
        "{}-{}",
        field(&details, "/bjInfo/nick")?,
        field(&details, "/bjInfo/id")?
    );
    let mut data = StreamData::new(input, platform, name, details.get("media").is_some());
    if !data.is_live {
        return Ok(data);
    }
    let password = query(&input.live_url, "pwd").unwrap_or_default();
    let play = ctx
        .json(
            &format!("https://api.{site}.co.kr/v1/live/play"),
            &headers,
            form(&[
                ("action", "watch"),
                ("userId", &id),
                ("password", &password),
                ("shareLinkType", ""),
            ]),
        )
        .await?;
    if play.get("errorData").is_some() {
        let code = optional(&play, "/errorData/code");
        return Err(if code == "needAdult" {
            format!("{platform} 房间需要已成年登录账号的有效 Cookie")
        } else {
            format!(
                "{platform} 播放失败：{code} {}",
                optional(&play, "/message")
            )
        });
    }
    let hls = field(&play, "/PlayList/hls/0/url")?;
    let streams = ctx.variants(&hls, &headers).await?;
    data.urls(
        pick(&streams, &input.quality)?.clone(),
        String::new(),
        false,
    );
    data.m3u8_url = Some(hls);
    Ok(data)
}
