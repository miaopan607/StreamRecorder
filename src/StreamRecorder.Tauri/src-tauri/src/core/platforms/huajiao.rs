use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    match web(ctx, input).await {
        Ok(data) => Ok(data),
        Err(web_error) => app(ctx, input)
            .await
            .map_err(|app_error| format!("web: {web_error}; app: {app_error}")),
    }
}
async fn web(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = query(&input.live_url, "liveid")?;
    let headers = [
        (
            "user-agent",
            "living/9.4.0 (com.huajiao.seeding; build:2410231746; iOS 17.0.0) Alamofire/9.4.0",
        ),
        ("accept-language", "zh-Hans-US;q=1.0"),
        ("sdk_version", "1"),
    ];
    let info = ctx
        .json(
            &format!(
                "https://live.huajiao.com/feed/getFeedInfo?relateid={}",
                encode(&id)
            ),
            &headers,
            Body::None,
        )
        .await?;
    if truth(at(&info, "/errmsg")?) || !info.pointer("/data/creatime").is_some_and(truth) {
        return Err("花椒直播地址已失效，请使用新的分享地址".into());
    }
    let mut data = StreamData::new(
        input,
        "花椒直播",
        field(&info, "/data/author/nickname")?,
        true,
    );
    data.title = Some(field(&info, "/data/feed/title")?);
    let api=format!("https://live.huajiao.com/live/substream?time={}&version=1.0.0&sn={}&liveid={}&uid={}&encode=h265",chrono::Utc::now().timestamp_millis(),encode(&field(&info,"/data/feed/sn")?),encode(&field(&info,"/data/feed/relateid")?),encode(&field(&info,"/data/author/uid")?));
    let info=ctx.json(&api,&[("referer","https://www.huajiao.com/"),("user-agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0")],Body::None).await?;
    data.urls(
        field(&info, "/data/pull_m3u8")?,
        field(&info, "/data/h264_url")?,
        true,
    );
    Ok(data)
}
async fn app(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = query(&input.live_url, "author")?;
    let info = ctx
        .json(
            &format!(
                "https://live.huajiao.com/feed/getUserFeeds?channel=Apple&userid={}&uid={}",
                encode(&id),
                encode(&id)
            ),
            &[("user-agent", MOBILE_UA)],
            Body::None,
        )
        .await?;
    let feed = at(&info, "/data/feeds/0")?;
    let mut data = StreamData::new(
        input,
        "花椒直播",
        field(feed, "/author/nickname")?,
        field(feed, "/feed/rtop")? == "直播中",
    );
    if data.is_live {
        data.urls(
            String::new(),
            format!(
                "{}&codec={}",
                field(feed, "/feed/pull_url")?,
                field(feed, "/feed/encode")?
            ),
            true,
        );
    }
    Ok(data)
}
