// 顺序与旧数据的平台识别规则一致，先识别房间域名，再识别直链。
const MARKERS: &[(&str, &str, &str)] = &[
    ("douyin.com/", "抖音直播", "douyin"),
    ("tiktok.com/", "TikTok直播", "tiktok"),
    ("live.kuaishou.com/", "快手直播", "kuaishou"),
    ("huya.com/", "虎牙直播", "huya"),
    ("douyu.com/", "斗鱼直播", "douyu"),
    ("yy.com/", "YY直播", "yy"),
    ("live.bilibili.com/", "B站直播", "bilibili"),
    ("xiaohongshu.com/", "小红书直播", "xiaohongshu"),
    ("xhslink.com/", "小红书直播", "xhs"),
    ("bigo.tv/", "Bigo直播", "bigo"),
    ("app.blued.cn/", "Blued直播", "blued"),
    ("sooplive.co.kr/", "SOOP直播", "soop"),
    ("cc.163.com/", "网易CC直播", "netease"),
    ("qiandurebo.com/", "千度热播", "qiandurebo"),
    ("pandalive.co.kr/", "PandaTV直播", "pandalive"),
    ("fm.missevan.com/", "猫耳FM直播", "maoerfm"),
    ("winktv.co.kr/", "WinkTV直播", "winktv"),
    ("flextv.co.kr/", "FlexTV直播", "flextv"),
    ("ttinglive.com/", "FlexTV直播", "flextv"),
    ("look.163.com/", "Look直播", "look"),
    ("popkontv.com/", "PopkonTV直播", "popkontv"),
    ("twitcasting.tv/", "TwitCasting直播", "twitcasting"),
    ("live.baidu.com/", "百度直播", "baidu"),
    ("weibo.com/", "微博直播", "weibo"),
    ("kugou.com/", "酷狗直播", "kugou"),
    ("twitch.tv/", "Twitch直播", "twitch"),
    ("liveme.com/", "LiveMe直播", "liveme"),
    ("huajiao.com/", "花椒直播", "huajiao"),
    ("showroom-live.com/", "ShowRoom直播", "showroom"),
    ("live.acfun.cn/", "AcFun直播", "acfun"),
    ("tlclw.com/", "畅聊直播", "changliao"),
    ("ybw1666.com/", "音播直播", "yinbo"),
    ("inke.cn/", "映客直播", "inke"),
    ("zhihu.com/", "知乎直播", "zhihu"),
    ("chzzk.naver.com/", "CHZZK直播", "chzzk"),
    ("haixiutv.com/", "嗨秀直播", "haixiu"),
    ("vvxqiu.com/", "VVXQ直播", "vvxq"),
    ("17.live/", "17Live直播", "17live"),
    ("lang.live/", "浪Live", "lang"),
    ("weimipopo.com/", "漂漂直播", "piaopiao"),
    (".6.cn/", "六间房直播", "6room"),
    ("lehaitv.com/", "乐嗨直播", "lehai"),
    ("catshow168.com/", "花猫直播", "catshow"),
    ("live.shopee", "Shopee直播", "shopee"),
    (".shp.", "Shopee直播", "shopee"),
    ("youtube.com/", "YouTube直播", "youtube"),
    ("tb.cn", "淘宝直播", "taobao"),
    ("3.cn", "京东直播", "jd"),
    ("faceit.com", "Faceit直播", "faceit"),
    ("lailianjie.com", "连接直播", "lianjie"),
    ("miguvideo.com", "咪咕直播", "migu"),
    ("imkktv.com", "来秀直播", "laixiu"),
    ("picarto.tv", "Picarto直播", "picarto"),
    (".m3u8", "自定义录制直播", "custom"),
    (".flv", "自定义录制直播", "custom"),
];
pub fn identify(url: &str) -> (&'static str, &'static str) {
    let lowered = url.to_ascii_lowercase();
    MARKERS
        .iter()
        .find(|(marker, _, _)| lowered.contains(marker))
        .map(|(_, name, key)| (*name, *key))
        .unwrap_or(("未知平台", "unknown"))
}

use super::probe::{ProbeContext, ProbeInput, StreamData};
mod acfun;
mod baidu;
mod bigo;
mod bilibili;
mod blued;
mod changliao;
mod chzzk;
mod douyin;
mod douyu;
mod faceit;
mod flextv;
mod haixiu;
mod huajiao;
mod huamao;
mod huya;
mod inke;
mod jd;
mod kuaishou;
mod kugou;
mod laixiu;
mod langlive;
mod lehai;
mod lianjie;
mod liveme;
mod look;
mod maoer;
mod migu;
mod migu_wasm;
mod netease;
mod pandatv;
mod piaopiao;
mod picarto;
mod popkontv;
mod qiandurebo;
mod rednote;
mod shopee;
mod showroom;
mod sixroom;
mod soop;
mod taobao;
mod tiktok;
mod twitcasting;
mod twitch;
mod vvxq;
mod weibo;
mod winktv;
mod xindongrebo;
mod yinbo;
mod youtube;
mod yy;
mod zhihu;

pub async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    match input.platform_key.as_str() {
        "acfun" => acfun::probe(ctx, input).await,
        "baidu" => baidu::probe(ctx, input).await,
        "bigo" => bigo::probe(ctx, input).await,
        "bilibili" => bilibili::probe(ctx, input).await,
        "blued" => blued::probe(ctx, input).await,
        "changliao" => changliao::probe(ctx, input).await,
        "chzzk" => chzzk::probe(ctx, input).await,
        "douyin" => douyin::probe(ctx, input).await,
        "douyu" => douyu::probe(ctx, input).await,
        "faceit" => faceit::probe(ctx, input).await,
        "flextv" => flextv::probe(ctx, input).await,
        "haixiu" => haixiu::probe(ctx, input).await,
        "huajiao" => huajiao::probe(ctx, input).await,
        "catshow" => huamao::probe(ctx, input).await,
        "huya" => huya::probe(ctx, input).await,
        "inke" => inke::probe(ctx, input).await,
        "jd" => jd::probe(ctx, input).await,
        "kuaishou" => kuaishou::probe(ctx, input).await,
        "kugou" => kugou::probe(ctx, input).await,
        "laixiu" => laixiu::probe(ctx, input).await,
        "lang" => langlive::probe(ctx, input).await,
        "lehai" => lehai::probe(ctx, input).await,
        "lianjie" => lianjie::probe(ctx, input).await,
        "liveme" => liveme::probe(ctx, input).await,
        "look" => look::probe(ctx, input).await,
        "maoerfm" => maoer::probe(ctx, input).await,
        "migu" => migu::probe(ctx, input).await,
        "netease" => netease::probe(ctx, input).await,
        "pandalive" => pandatv::probe(ctx, input).await,
        "piaopiao" => piaopiao::probe(ctx, input).await,
        "picarto" => picarto::probe(ctx, input).await,
        "popkontv" => popkontv::probe(ctx, input).await,
        "qiandurebo" => qiandurebo::probe(ctx, input).await,
        "xhs" | "xiaohongshu" => rednote::probe(ctx, input).await,
        "shopee" => shopee::probe(ctx, input).await,
        "showroom" => showroom::probe(ctx, input).await,
        "6room" => sixroom::probe(ctx, input).await,
        "soop" => soop::probe(ctx, input).await,
        "taobao" => taobao::probe(ctx, input).await,
        "tiktok" => tiktok::probe(ctx, input).await,
        "twitcasting" => twitcasting::probe(ctx, input).await,
        "twitch" => twitch::probe(ctx, input).await,
        "vvxq" => vvxq::probe(ctx, input).await,
        "weibo" => weibo::probe(ctx, input).await,
        "winktv" => winktv::probe(ctx, input).await,
        "17live" => xindongrebo::probe(ctx, input).await,
        "yinbo" => yinbo::probe(ctx, input).await,
        "youtube" => youtube::probe(ctx, input).await,
        "yy" => yy::probe(ctx, input).await,
        "zhihu" => zhihu::probe(ctx, input).await,
        key => Err(format!("暂不支持的平台：{key}")),
    }
}

#[cfg(test)]
mod tests;
