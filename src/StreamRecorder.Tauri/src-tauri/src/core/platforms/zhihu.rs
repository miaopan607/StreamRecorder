use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    if !input.live_url.contains("/theater/") {
        return Err("请使用知乎直播页面 URL".into());
    }
    let id = room(&input.live_url)?;
    let html=ctx.get(&input.live_url,&[("user-agent","osee2unifiedRelease/21914 osee2unifiedReleaseVersion/10.39.0 Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148"),("referer","https://live.ybw1666.com/800005143?promoters=0")]).await?;
    let raw = capture(
        r#"<script id="js-initialData" type="text/json">(.*?)</script>"#,
        &html,
    )
    .map_err(|_| "知乎页面缺少直播数据".to_string())?;
    let json = parse_json(&raw)?;
    let info = at(&json, "/initialState/theater/theaters")?
        .get(&id)
        .ok_or("知乎房间数据缺失")?;
    let mut data = StreamData::new(
        input,
        "知乎直播",
        field(info, "/actor/name")?,
        at(info, "/drama/status")?.as_i64() == Some(1),
    );
    if data.is_live {
        data.title = Some(field(info, "/theme")?);
        data.urls(
            field(info, "/drama/playInfo/hlsUrl")?,
            field(info, "/drama/playInfo/playUrl")?,
            false,
        );
    }
    Ok(data)
}
