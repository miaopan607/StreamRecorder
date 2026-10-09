use crate::core::probe::*;
use serde_json::json;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    match web(ctx, input).await {
        Ok(data) => Ok(data),
        Err(web_error) => app(ctx, input)
            .await
            .map_err(|error| format!("web: {web_error}; app: {error}")),
    }
}
pub(super) async fn web(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = query(&input.live_url, "anchorUid")?;
    let cat = input.live_url.contains("catshow");
    let api = if cat {
        "https://api.catshow168.com/live/preview"
    } else {
        "https://api.pp.weimipopo.com/live/preview"
    };
    let origin = if cat {
        "https://h.catshow168.com"
    } else {
        "https://m.pp.weimipopo.com"
    };
    let headers = [
        ("origin", origin),
        ("referer", origin),
        (
            "user-agent",
            "ios/7.830 (ios 17.0; ; iPhone 15 (A2846/A3089/A3090/A3092))",
        ),
    ];
    let info = ctx
        .json(
            api,
            &headers,
            Body::Json(json!({"inviteUuid":"","anchorUuid":id})),
        )
        .await?;
    let room = at(&info, "/data")?;
    let mut data = StreamData::new(
        input,
        "飘飘直播",
        field(room, "/name")?,
        truth(at(room, "/living")?),
    );
    if data.is_live {
        data.urls(field(room, "/pullUrl")?, String::new(), false);
    }
    Ok(data)
}
async fn app(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = room(&input.live_url)?;
    let request = json!({"platform":"iOS","device":"iPhone","token":"Iq6iKDovSwvmmMtJo8f3bqcThX573ndM","channel":"AppStorePLIM","subChannel":"","version":"1.7.27","meid":"","uid":92128122,"params":{"keyword":id},"build":"183","app":"plpl","imei":""});
    let info = ctx
        .json(
            "https://api.pp.weimipopo.com/plpl/pms/search/user/v2",
            &[("user-agent", MOBILE_UA)],
            Body::Json(request),
        )
        .await?;
    let users = array(at(&info, "/data/userList")?)?;
    let mut data = StreamData::new(input, "飘飘直播", String::new(), false);
    if let Some(user) = users.iter().find(|u| optional(u, "/user/shortId") == id) {
        data.anchor_name = Some(field(user, "/user/name")?);
        if users.first().and_then(|u| u.get("live")).is_some_and(truth) {
            data.is_live = true;
            data.title = Some(field(&info, "/data/livingUsers/0/live/title")?);
            data.urls(
                field(&info, "/data/livingUsers/0/live/pullUrl")?,
                String::new(),
                false,
            );
        }
    }
    Ok(data)
}
