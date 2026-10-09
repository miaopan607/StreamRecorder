use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = room(&input.live_url)?;
    let cookie = input
        .cookies
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("__ac_nonce=064caded4009deafd8b89;");
    let headers = [
        ("cookie", cookie),
        ("origin", "https://live.bilibili.com"),
        ("referer", "https://live.bilibili.com/26066074"),
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0",
        ),
    ];
    let room = ctx
        .json(
            &format!(
                "https://api.live.bilibili.com/room/v1/Room/room_init?id={}",
                encode(&id)
            ),
            &headers,
            Body::None,
        )
        .await?;
    if room
        .get("code")
        .and_then(serde_json::Value::as_i64)
        .is_some_and(|c| c != 0)
    {
        return Err(format!("B站房间查询失败：{}", optional(&room, "/message")));
    }
    let uid = field(&room, "/data/uid")?;
    let profile = ctx
        .json(
            &format!(
                "https://api.live.bilibili.com/live_user/v1/Master/info?uid={}",
                encode(&uid)
            ),
            &headers,
            Body::None,
        )
        .await?;
    let mut data = StreamData::new(
        input,
        "哔哩哔哩",
        field(&profile, "/data/info/uname")?,
        at(&room, "/data/live_status")?.as_i64() == Some(1),
    );
    data.live_url = Some(format!("https://live.bilibili.com/{id}"));
    let title = ctx
        .json(
            &format!(
                "https://api.live.bilibili.com/xlive/web-room/v1/index/getH5InfoByRoom?room_id={}",
                encode(&id)
            ),
            &headers,
            Body::None,
        )
        .await?;
    data.title = Some(optional(&title, "/data/room_info/title"));
    if !data.is_live {
        return Ok(data);
    }
    let qn = match input.quality.to_ascii_uppercase().as_str() {
        "BD" => "400",
        "UHD" => "250",
        "HD" => "150",
        "SD" | "LD" => "80",
        _ => "10000",
    };
    let old = ctx
        .json(
            &format!(
                "https://api.live.bilibili.com/room/v1/Room/playUrl?cid={}&qn={qn}&platform=web",
                encode(&id)
            ),
            &headers,
            Body::None,
        )
        .await?;
    let play = if old.get("code").and_then(serde_json::Value::as_i64) == Some(0) {
        let urls = array(at(&old, "/data/durl")?)?;
        let selected = urls
            .iter()
            .find(|v| optional(v, "/url").contains("d1--cn-gotcha"))
            .or_else(|| urls.last())
            .ok_or("B站没有可用流地址")?;
        field(selected, "/url")?
    } else {
        let info=ctx.json(&format!("https://api.live.bilibili.com/xlive/web-room/v2/index/getRoomPlayInfo?room_id={}&protocol=0%2C1&format=0%2C1%2C2&codec=0%2C1%2C2&qn={qn}&platform=web&ptype=8&dolby=5&panorama=1&hdr_type=0%2C1",encode(&id)),&headers,Body::None).await?;
        if at(&info, "/data/live_status")?.as_i64() == Some(0) {
            data.is_live = false;
            return Ok(data);
        }
        let mut codecs = array(at(
            &info,
            "/data/playurl_info/playurl/stream/0/format/0/codec",
        )?)?
        .iter()
        .collect::<Vec<_>>();
        codecs.sort_by_key(|v| {
            std::cmp::Reverse(
                v.get("current_qn")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
            )
        });
        let index = match qn {
            "400" => 1,
            "250" => 2,
            "150" => 3,
            "80" => 4,
            _ => 0,
        };
        let codec = codecs
            .get(index.min(codecs.len().saturating_sub(1)))
            .ok_or("B站没有可用画质")?;
        format!(
            "{}{}{}",
            field(codec, "/url_info/0/host")?,
            field(codec, "/base_url")?,
            field(codec, "/url_info/0/extra")?
        )
    };
    data.record_url = Some(play);
    Ok(data)
}
