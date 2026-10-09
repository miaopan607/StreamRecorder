use crate::core::probe::*;
use rand::Rng;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let id = room(&input.live_url)?;
    let cookie = input
        .cookies
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("__ac_nonce=064caded4009deafd8b89;");
    let headers=[("origin","https://live.acfun.cn"),("referer","https://live.acfun.cn/"),("cookie",cookie),("user-agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36 Edg/136.0.0.0")];
    let info = ctx
        .json(
            &format!(
                "https://live.acfun.cn/rest/pc-direct/user/userInfo?userId={}",
                encode(&id)
            ),
            &headers,
            Body::None,
        )
        .await?;
    let profile = at(&info, "/profile")?;
    let mut data = StreamData::new(
        input,
        "Acfun",
        field(profile, "/name")?,
        profile.get("liveId").is_some(),
    );
    if !data.is_live {
        return Ok(data);
    }
    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let random: String = (0..16)
        .map(|_| chars[rand::thread_rng().gen_range(0..chars.len())] as char)
        .collect();
    let did = format!("web_{random}");
    let visitor_cookie = format!("_did={did}");
    let visitor=ctx.json("https://id.app.acfun.cn/rest/app/visitor/login",&[("referer","https://live.acfun.cn/"),("user-agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0"),("cookie",&visitor_cookie)],form(&[("sid","acfun.api.visitor")])).await?;
    let play=ctx.json(&format!("https://api.kuaishouzt.com/rest/zt/live/web/startPlay?subBiz=mainApp&kpn=ACFUN_APP&kpf=PC_WEB&userId={}&did={}&acfun.api.visitor_st={}",encode(&field(&visitor,"/userId")?),encode(&did),encode(&field(&visitor,"/acfun.api.visitor_st")?)),&headers,form(&[("authorId",&id),("pullStreamType","FLV")])).await?;
    data.title = Some(field(&play, "/data/caption")?);
    let manifest = parse_json(&field(&play, "/data/videoPlayRes")?)?;
    let mut streams = array(at(
        &manifest,
        "/liveAdaptiveManifest/0/adaptationSet/representation",
    )?)?
    .iter()
    .collect::<Vec<_>>();
    streams.sort_by_key(|v| {
        std::cmp::Reverse(
            v.get("bitrate")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        )
    });
    data.urls(
        String::new(),
        field(pick(&streams, &input.quality)?, "/url")?,
        true,
    );
    Ok(data)
}
