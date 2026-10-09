use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let headers = [
        ("referer", "https://ios.6.cn/?ver=8.0.3&build=4"),
        (
            "user-agent",
            "ios/7.830 (ios 17.0; ; iPhone 15 (A2846/A3089/A3090/A3092))",
        ),
    ];
    let html = ctx
        .get(
            &format!("https://v.6.cn/{}", encode(&room(&input.live_url)?)),
            &headers,
        )
        .await?;
    let id = capture(r"rid: '(.*?)',\n\s+roomid", &html)?;
    let info = ctx
        .json(
            "https://v.6.cn/coop/mobile/index.php?padapi=coop-mobile-inroom.php",
            &headers,
            form(&[
                ("av", "3.1"),
                ("encpass", ""),
                ("logiuid", ""),
                ("project", "v6iphone"),
                ("rate", "1"),
                ("rid", ""),
                ("ruid", &id),
            ]),
        )
        .await?;
    // 显式 null 表示没有直播流；字段缺失仍是响应错误。
    let title = text(
        info.pointer("/content/liveinfo/flvtitle")
            .ok_or("平台响应缺少字段 /content/liveinfo/flvtitle")?,
    );
    let mut data = StreamData::new(
        input,
        "六间房直播",
        field(&info, "/content/roominfo/alias")?,
        !title.is_empty(),
    );
    if data.is_live {
        data.urls(
            String::new(),
            format!("https://wlive.6rooms.com/httpflv/{title}.flv"),
            true,
        );
    }
    Ok(data)
}
