use crate::core::probe::*;
use crate::core::signing;
use serde_json::{json, Value};
const DID: &str = "10000000000000000000000000001501";
const UA:&str="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    match web(ctx, input).await {
        Ok(data) => Ok(data),
        Err(error) => app(ctx, input)
            .await
            .map_err(|app| format!("web: {error}; app: {app}")),
    }
}
async fn room_id(ctx: &ProbeContext, input: &ProbeInput) -> Result<String, String> {
    if let Ok(id) = capture(r"douyu.com/(\d+)", &input.live_url)
        .or_else(|_| capture(r"rid=(\d+)", &input.live_url))
    {
        return Ok(id);
    }
    let id = input
        .live_url
        .split("douyu.com/")
        .nth(1)
        .and_then(|s| s.split(['?', '/']).next())
        .ok_or("斗鱼房间 ID 缺失")?;
    let html = ctx
        .get(&format!("https://m.douyu.com/{id}"), &headers())
        .await?;
    capture(r#""rid":(\d+)"#, &html)
}
fn headers() -> [(&'static str, &'static str); 2] {
    [("user-agent", UA), ("referer", "https://www.douyu.com/")]
}
async fn web(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = room_id(ctx, input).await?;
    let info = ctx
        .json(
            &format!("https://www.douyu.com/betard/{id}"),
            &headers(),
            Body::None,
        )
        .await?;
    let room = at(&info, "/room")?;
    let mut data = StreamData::new(
        input,
        "斗鱼直播",
        field(room, "/nickname")?,
        at(room, "/show_status")?.as_i64() == Some(1),
    );
    let title = field(room, "/room_name")?
        .replace("&nbsp;", " ")
        .trim()
        .to_owned();
    data.title = Some(if at(room, "/videoLoop")?.as_i64() == Some(1) {
        format!("【轮播】{title}")
    } else {
        title
    });
    data.extra = Some(json!({"backup_url_list":[]}));
    if !data.is_live {
        return Ok(data);
    }
    let rid = field(room, "/room_id")?;
    let rate = match input.quality.to_ascii_uppercase().as_str() {
        "UHD" => "3",
        "HD" => "2",
        "SD" | "LD" => "1",
        _ => "0",
    };
    let play = fetch_play(ctx, &rid, rate, None).await?;
    if at(&play, "/error")?.as_i64() != Some(0) {
        return Err(format!("斗鱼播放请求失败：{}", optional(&play, "/error")));
    }
    let stream = at(&play, "/data")?;
    let primary = format!(
        "{}/{}",
        field(stream, "/rtmp_url")?,
        field(stream, "/rtmp_live")?
    );
    let current = optional(stream, "/rtmp_cdn");
    let mut backups = vec![];
    if let Some(cdns) = stream.get("cdnsWithName").and_then(Value::as_array) {
        for cdn in cdns {
            let cdn = field(cdn, "/cdn")?;
            if cdn != current {
                let alternate = fetch_play(ctx, &rid, rate, Some(&cdn)).await?;
                if alternate.get("error").and_then(Value::as_i64) == Some(0) {
                    let stream = at(&alternate, "/data")?;
                    let url = format!(
                        "{}/{}",
                        field(stream, "/rtmp_url")?,
                        field(stream, "/rtmp_live")?
                    );
                    if url != primary && !backups.contains(&url) {
                        backups.push(url);
                    }
                }
            }
        }
    }
    data.urls(String::new(), primary, true);
    data.extra = Some(json!({"backup_url_list":backups}));
    Ok(data)
}
async fn fetch_play(
    ctx: &ProbeContext,
    id: &str,
    rate: &str,
    cdn: Option<&str>,
) -> Result<Value, String> {
    let white = ctx
        .json(
            &format!("https://www.douyu.com/wgapi/livenc/liveweb/websec/getEncryption?did={DID}"),
            &[("user-agent", UA)],
            Body::None,
        )
        .await?;
    if at(&white, "/error")?.as_i64() != Some(0) {
        return Err("获取斗鱼白名单密钥失败".into());
    }
    let white = at(&white, "/data")?;
    let key = field(white, "/key")?;
    let mut secret = field(white, "/rand_str")?;
    let count = at(white, "/enc_time")?
        .as_u64()
        .filter(|n| *n <= 10000)
        .ok_or("斗鱼加密轮次超出限制")?;
    for _ in 0..count {
        secret = signing::md5(format!("{secret}{key}"));
    }
    let time = chrono::Utc::now().timestamp();
    let salt = if truth(at(white, "/is_special")?) {
        String::new()
    } else {
        format!("{id}{time}")
    };
    let auth = signing::md5(format!("{secret}{key}{salt}"));
    let mut fields = vec![
        ("rate".into(), rate.into()),
        ("ver".into(), "219032101".into()),
        ("iar".into(), "0".into()),
        ("ive".into(), "0".into()),
        ("rid".into(), id.into()),
        ("hevc".into(), "0".into()),
        ("fa".into(), "0".into()),
        ("sov".into(), "0".into()),
        ("enc_data".into(), field(white, "/enc_data")?),
        ("tt".into(), time.to_string()),
        ("did".into(), DID.into()),
        ("auth".into(), auth),
    ];
    if let Some(cdn) = cdn {
        fields.push(("cdn".into(), cdn.into()));
    }
    let mut head = headers().to_vec();
    head.push(("origin", "https://www.douyu.com"));
    head.push(("content-type", "application/x-www-form-urlencoded"));
    ctx.json(
        &format!("https://playweb.douyucdn.cn/lapi/live/getH5PlayV1/{id}"),
        &head,
        Body::Form(fields),
    )
    .await
}
async fn app(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = room_id(ctx, input).await?;
    let info = ctx
        .json(
            "https://wxapp.douyucdn.cn/api/wechatsearch/nc/search/multiv2",
            &headers(),
            form(&[
                ("sk", &id),
                ("log_token", ""),
                ("ct_code", "26"),
                ("token", ""),
            ]),
        )
        .await?;
    let room = at(&info, "/data/recom")?;
    let mut data = StreamData::new(
        input,
        "斗鱼直播",
        optional(room, "/nickname"),
        room.get("isLive").and_then(Value::as_i64) == Some(1),
    );
    data.extra = Some(json!({"backup_url_list":[]}));
    if data.is_live {
        data.title = Some(optional(room, "/roomName"));
        data.quality = Some("OD".into());
        data.urls(String::new(), field(room, "/stream")?, true);
    }
    Ok(data)
}
