use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let nickname = capture(r"/players/(.*?)/stream", &input.live_url)?;
    let user = ctx
        .json(
            &format!(
                "https://www.faceit.com/api/users/v1/nicknames/{}",
                encode(&nickname)
            ),
            &[],
            Body::None,
        )
        .await?;
    let streams = ctx
        .json(
            &format!(
                "https://www.faceit.com/api/stream/v1/streamings?userId={}",
                encode(&field(&user, "/payload/id")?)
            ),
            &[],
            Body::None,
        )
        .await?;
    let list = streams.get("payload").and_then(serde_json::Value::as_array);
    let Some(stream) = list.and_then(|l| l.first()) else {
        return Ok(StreamData::new(input, "Faceit", nickname, false));
    };
    let name = optional(stream, "/userNickname");
    if stream.get("platform").and_then(serde_json::Value::as_str) != Some("twitch") {
        return Ok(StreamData::new(input, "Faceit", name, false));
    }
    let mut twitch = input.clone();
    twitch.live_url = format!("https://www.twitch.tv/{}", field(stream, "/platformId")?);
    twitch.platform_key = "twitch".into();
    twitch.cookies = None;
    let mut data = super::twitch::probe(ctx, &twitch).await?;
    data.platform = Some("Faceit".into());
    data.anchor_name = Some(name);
    data.live_url = Some(input.live_url.clone());
    Ok(data)
}
