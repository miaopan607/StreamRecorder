use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let fallback="SUB=_2AkMRNMCwf8NxqwFRmfwWymPrbI9-zgzEieKnaDFrJRMxHRl-yT9kqmkhtRB6OrTuX5z9N_7qk9C3xxEmNR-8WLcyo2PM; SUBP=0033WrSXqPxfM72-Ws9jqgMF55529P9D9WWemwcqkukCduUO11o9sBqA;";
    let cookie = input
        .cookies
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback);
    let headers = [
        ("cookie", cookie),
        ("referer", "https://weibo.com/u/5885340893"),
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:124.0) Gecko/20100101 Firefox/124.0",
        ),
    ];
    let mut data = StreamData::new(input, "微博直播", String::new(), false);
    let id = if let Some((_, id)) = input
        .live_url
        .split('?')
        .next()
        .unwrap()
        .split_once("show/")
    {
        id.to_owned()
    } else {
        let uid = input
            .live_url
            .split('?')
            .next()
            .unwrap()
            .rsplit("/u/")
            .next()
            .ok_or("微博用户 ID 缺失")?;
        let json = ctx
            .json(
                &format!(
                    "https://weibo.com/ajax/statuses/mymblog?uid={}&page=1&feature=0",
                    encode(uid)
                ),
                &headers,
                Body::None,
            )
            .await?;
        let posts = array(at(&json, "/data/list")?)?;
        data.anchor_name = posts.first().map(|p| optional(p, "/user/screen_name"));
        posts
            .iter()
            .find(|p| {
                p.pointer("/page_info/object_type")
                    .and_then(serde_json::Value::as_str)
                    == Some("live")
            })
            .map(|p| optional(p, "/page_info/object_id"))
            .unwrap_or_default()
    };
    if id.is_empty() {
        return Ok(data);
    }
    let json = ctx
        .json(
            &format!("https://weibo.com/l/pc/anchor/live?live_id={}", encode(&id)),
            &headers,
            Body::None,
        )
        .await?;
    let info = at(&json, "/data")?;
    data.anchor_name = Some(field(info, "/user_info/name")?);
    data.is_live = at(info, "/item/status")?.as_i64() == Some(1);
    if data.is_live {
        data.title = Some(field(info, "/item/desc")?);
        let hls = field(info, "/item/stream_info/pull/live_origin_hls_url")?;
        let flv = field(info, "/item/stream_info/pull/live_origin_flv_url")?;
        if quality_index(&input.quality) == 0 {
            data.urls(hls, flv, false)
        } else {
            data.urls(
                format!("{}.m3u8", hls.split('_').next().unwrap()),
                format!("{}.flv", flv.split('_').next().unwrap()),
                false,
            )
        }
    }
    Ok(data)
}
