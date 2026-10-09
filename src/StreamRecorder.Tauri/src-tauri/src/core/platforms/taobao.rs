use crate::core::probe::*;
use crate::core::signing;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let base_headers = [
        ("referer", "https://huodong.m.taobao.com/"),
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0",
        ),
    ];
    let id = match query(&input.live_url, "id").or_else(|_| query(&input.live_url, "liveId")) {
        Ok(id) => id,
        Err(_) => {
            let html = ctx.get(&input.live_url, &base_headers).await?;
            let url = capture(r"var url = '(.*?)';", &html)?;
            query(&url, "id").or_else(|_| query(&url, "liveId"))?
        }
    };
    let payload = format!(
        "{{\"liveId\":{},\"creatorId\":null}}",
        serde_json::to_string(&id).map_err(|e| e.to_string())?
    );
    let mut session = ctx.session.fields.lock().await;
    let mut cookie = session
        .get("cookie")
        .cloned()
        .or_else(|| input.cookies.clone())
        .unwrap_or_default();
    let mut refreshed = None;
    for _ in 0..2 {
        let time = chrono::Utc::now().timestamp_millis();
        let token = cookie
            .split(';')
            .filter_map(|s| s.trim().split_once('='))
            .find(|(k, _)| *k == "_m_h5_tk")
            .map(|(_, v)| v.split('_').next().unwrap());
        let signature = if cookie.contains("_m_h5_tk_enc=") {
            token
                .map(|token| signing::md5(format!("{token}&{time}&12574478&{payload}")))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let api=format!("https://h5api.m.taobao.com/h5/mtop.mediaplatform.live.livedetail/4.0/?jsv=2.7.0&appKey=12574478&t={time}&sign={signature}&AntiFlood=true&AntiCreep=true&api=mtop.mediaplatform.live.livedetail&v=4.0&preventFallback=true&type=jsonp&dataType=jsonp&callback=mtopjsonp1&data={}",encode(&payload));
        let mut headers = base_headers.to_vec();
        headers.push(("cookie", &cookie));
        let response = ctx
            .request(reqwest::Method::GET, &api, &headers, Body::None)
            .await?;
        let raw = response.text.trim().trim_end_matches(';');
        let inner = raw
            .split_once('(')
            .and_then(|(_, s)| s.rsplit_once(')').map(|(v, _)| v))
            .ok_or("淘宝 JSONP 格式错误")?;
        let info = parse_json(inner)?;
        let result = array(at(&info, "/ret")?)?;
        if result
            .first()
            .is_some_and(|v| text(v).contains("哎哟喂,被挤爆啦,请稍后重试"))
        {
            return Err("淘宝 Cookie 已失效或触发风控，请更新 Cookie".into());
        }
        if result.len() == 1 && result[0] == "SUCCESS::调用成功" {
            let room = at(&info, "/data")?;
            let mut data = StreamData::new(
                input,
                "淘宝直播",
                field(room, "/broadCaster/accountName")?,
                field(room, "/streamStatus")? == "1",
            );
            data.live_url = Some(format!("https://tbzb.taobao.com/live?liveId={id}"));
            data.new_cookies = refreshed;
            if data.is_live {
                data.title = Some(field(room, "/title")?);
                let mut streams = array(at(room, "/liveUrlList")?)?.iter().collect::<Vec<_>>();
                streams.sort_by_key(|v| {
                    std::cmp::Reverse(
                        match v
                            .get("definition")
                            .filter(|v| truth(v))
                            .or_else(|| v.get("newDefinition"))
                            .and_then(serde_json::Value::as_str)
                        {
                            Some("lld") => 0,
                            Some("ld") => 1,
                            Some("md") => 2,
                            Some("hd") => 3,
                            Some("ud") => 4,
                            _ => -1,
                        },
                    )
                });
                let selected = pick(&streams, &input.quality)?;
                data.urls(
                    field(selected, "/hlsUrl")?,
                    field(selected, "/flvUrl")?,
                    false,
                );
            }
            return Ok(data);
        }
        if !response.cookies.contains_key("_m_h5_tk")
            || !response.cookies.contains_key("_m_h5_tk_enc")
        {
            return Err("淘宝 Cookie 刷新失败，请更新登录 Cookie".into());
        }
        cookie = cookie_string(&response.cookies);
        session.insert("cookie".into(), cookie.clone());
        refreshed = Some(cookie.clone());
    }
    Err("淘宝登录已失效，刷新 Cookie 后仍无法读取直播间".into())
}
