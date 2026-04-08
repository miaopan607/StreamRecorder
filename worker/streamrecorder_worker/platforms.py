from __future__ import annotations

from typing import Final


PlatformInfo = tuple[str, str]


PLATFORM_MAP: Final[tuple[tuple[str, PlatformInfo], ...]] = (
    ("douyin.com/", ("抖音直播", "douyin")),
    ("tiktok.com/", ("TikTok直播", "tiktok")),
    ("live.kuaishou.com/", ("快手直播", "kuaishou")),
    ("huya.com/", ("虎牙直播", "huya")),
    ("douyu.com/", ("斗鱼直播", "douyu")),
    ("yy.com/", ("YY直播", "yy")),
    ("live.bilibili.com/", ("B站直播", "bilibili")),
    ("xiaohongshu.com/", ("小红书直播", "xiaohongshu")),
    ("xhslink.com/", ("小红书直播", "xhs")),
    ("bigo.tv/", ("Bigo直播", "bigo")),
    ("app.blued.cn/", ("Blued直播", "blued")),
    ("sooplive.co.kr/", ("SOOP直播", "soop")),
    ("cc.163.com/", ("网易CC直播", "netease")),
    ("qiandurebo.com/", ("千度热播", "qiandurebo")),
    ("pandalive.co.kr/", ("PandaTV直播", "pandalive")),
    ("fm.missevan.com/", ("猫耳FM直播", "maoerfm")),
    ("winktv.co.kr/", ("WinkTV直播", "winktv")),
    ("flextv.co.kr/", ("FlexTV直播", "flextv")),
    ("ttinglive.com/", ("FlexTV直播", "flextv")),
    ("look.163.com/", ("Look直播", "look")),
    ("popkontv.com/", ("PopkonTV直播", "popkontv")),
    ("twitcasting.tv/", ("TwitCasting直播", "twitcasting")),
    ("live.baidu.com/", ("百度直播", "baidu")),
    ("weibo.com/", ("微博直播", "weibo")),
    ("kugou.com/", ("酷狗直播", "kugou")),
    ("twitch.tv/", ("Twitch直播", "twitch")),
    ("liveme.com/", ("LiveMe直播", "liveme")),
    ("huajiao.com/", ("花椒直播", "huajiao")),
    ("showroom-live.com/", ("ShowRoom直播", "showroom")),
    ("live.acfun.cn/", ("AcFun直播", "acfun")),
    ("tlclw.com/", ("畅聊直播", "changliao")),
    ("ybw1666.com/", ("音播直播", "yinbo")),
    ("inke.cn/", ("映客直播", "inke")),
    ("zhihu.com/", ("知乎直播", "zhihu")),
    ("chzzk.naver.com/", ("CHZZK直播", "chzzk")),
    ("haixiutv.com/", ("嗨秀直播", "haixiu")),
    ("vvxqiu.com/", ("VVXQ直播", "vvxq")),
    ("17.live/", ("17Live直播", "17live")),
    ("lang.live/", ("浪Live", "lang")),
    ("weimipopo.com/", ("漂漂直播", "piaopiao")),
    (".6.cn/", ("六间房直播", "6room")),
    ("lehaitv.com/", ("乐嗨直播", "lehai")),
    ("catshow168.com/", ("花猫直播", "catshow")),
    ("live.shopee", ("Shopee直播", "shopee")),
    (".shp.", ("Shopee直播", "shopee")),
    ("youtube.com/", ("YouTube直播", "youtube")),
    ("tb.cn", ("淘宝直播", "taobao")),
    ("3.cn", ("京东直播", "jd")),
    ("faceit.com", ("Faceit直播", "faceit")),
    ("lailianjie.com", ("连接直播", "lianjie")),
    ("miguvideo.com", ("咪咕直播", "migu")),
    ("imkktv.com", ("来秀直播", "laixiu")),
    ("picarto.tv", ("Picarto直播", "picarto")),
    (".m3u8", ("自定义录制直播", "custom")),
    (".flv", ("自定义录制直播", "custom")),
)


def get_platform_info(url: str) -> PlatformInfo | tuple[None, None]:
    lowered_url = (url or "").lower()
    for marker, platform_info in PLATFORM_MAP:
        if marker in lowered_url:
            return platform_info
    return None, None
