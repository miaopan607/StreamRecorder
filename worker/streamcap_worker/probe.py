from __future__ import annotations

import inspect
from dataclasses import dataclass
from typing import Any

import streamget
from streamget import StreamData


@dataclass(frozen=True, slots=True)
class PlatformProbeSpec:
    stream_class_name: str


PLATFORM_SPECS: dict[str, PlatformProbeSpec] = {
    "douyin": PlatformProbeSpec("DouyinLiveStream"),
    "tiktok": PlatformProbeSpec("TikTokLiveStream"),
    "kuaishou": PlatformProbeSpec("KwaiLiveStream"),
    "huya": PlatformProbeSpec("HuyaLiveStream"),
    "douyu": PlatformProbeSpec("DouyuLiveStream"),
    "yy": PlatformProbeSpec("YYLiveStream"),
    "bilibili": PlatformProbeSpec("BilibiliLiveStream"),
    "xhs": PlatformProbeSpec("RedNoteLiveStream"),
    "xiaohongshu": PlatformProbeSpec("RedNoteLiveStream"),
    "bigo": PlatformProbeSpec("BigoLiveStream"),
    "blued": PlatformProbeSpec("BluedLiveStream"),
    "soop": PlatformProbeSpec("SoopLiveStream"),
    "netease": PlatformProbeSpec("NeteaseLiveStream"),
    "qiandurebo": PlatformProbeSpec("QiandureboLiveStream"),
    "pandalive": PlatformProbeSpec("PandaLiveStream"),
    "maoerfm": PlatformProbeSpec("MaoerLiveStream"),
    "winktv": PlatformProbeSpec("WinkTVLiveStream"),
    "flextv": PlatformProbeSpec("FlexTVLiveStream"),
    "look": PlatformProbeSpec("LookLiveStream"),
    "popkontv": PlatformProbeSpec("PopkonTVLiveStream"),
    "twitcasting": PlatformProbeSpec("TwitCastingLiveStream"),
    "baidu": PlatformProbeSpec("BaiduLiveStream"),
    "weibo": PlatformProbeSpec("WeiboLiveStream"),
    "kugou": PlatformProbeSpec("KugouLiveStream"),
    "twitch": PlatformProbeSpec("TwitchLiveStream"),
    "liveme": PlatformProbeSpec("LiveMeLiveStream"),
    "huajiao": PlatformProbeSpec("HuajiaoLiveStream"),
    "showroom": PlatformProbeSpec("ShowRoomLiveStream"),
    "acfun": PlatformProbeSpec("AcfunLiveStream"),
    "changliao": PlatformProbeSpec("ChangliaoLiveStream"),
    "yinbo": PlatformProbeSpec("YinboLiveStream"),
    "inke": PlatformProbeSpec("InkeLiveStream"),
    "zhihu": PlatformProbeSpec("ZhihuLiveStream"),
    "chzzk": PlatformProbeSpec("ChzzkLiveStream"),
    "haixiu": PlatformProbeSpec("HaixiuLiveStream"),
    "vvxq": PlatformProbeSpec("VVXQLiveStream"),
    "17live": PlatformProbeSpec("XindongreboLiveStream"),
    "lang": PlatformProbeSpec("LangLiveStream"),
    "piaopiao": PlatformProbeSpec("PiaopaioLiveStream"),
    "6room": PlatformProbeSpec("SixRoomLiveStream"),
    "lehai": PlatformProbeSpec("LehaiLiveStream"),
    "catshow": PlatformProbeSpec("HuamaoLiveStream"),
    "shopee": PlatformProbeSpec("ShopeeLiveStream"),
    "youtube": PlatformProbeSpec("YoutubeLiveStream"),
    "taobao": PlatformProbeSpec("TaobaoLiveStream"),
    "jd": PlatformProbeSpec("JDLiveStream"),
    "faceit": PlatformProbeSpec("FaceitLiveStream"),
    "lianjie": PlatformProbeSpec("LianJieLiveStream"),
    "migu": PlatformProbeSpec("MiguLiveStream"),
    "laixiu": PlatformProbeSpec("LaixiuLiveStream"),
    "picarto": PlatformProbeSpec("PicartoLiveStream"),
}


class StreamProbeService:
    def __init__(self) -> None:
        self._instances: dict[tuple[str, str | None, str | None], Any] = {}

    async def probe(
        self,
        *,
        platform_key: str,
        live_url: str,
        quality: str,
        proxy: str | None,
        cookies: str | None = None,
        username: str | None = None,
        password: str | None = None,
        account_type: str | None = None,
    ) -> StreamData:
        if platform_key == "custom":
            return self._probe_custom(live_url)

        spec = PLATFORM_SPECS.get(platform_key)
        if spec is None:
            raise ValueError(f"暂不支持的平台: {platform_key}")

        instance = self._get_or_create_instance(
            spec.stream_class_name,
            proxy=proxy,
            cookies=cookies,
            username=username,
            password=password,
            account_type=account_type,
        )

        fetch_errors: list[str] = []
        fetchers: list[str] = []
        if hasattr(instance, "fetch_web_stream_data"):
            fetchers.append("fetch_web_stream_data")
        if hasattr(instance, "fetch_app_stream_data"):
            fetchers.append("fetch_app_stream_data")

        if not fetchers:
            raise ValueError(f"平台 {platform_key} 没有可用的探测方法")

        for fetcher_name in fetchers:
            try:
                fetcher = getattr(instance, fetcher_name)
                raw_data = await fetcher(url=live_url)
                return await instance.fetch_stream_url(raw_data, quality)
            except Exception as exc:  # noqa: BLE001
                fetch_errors.append(f"{fetcher_name}: {exc}")

        raise RuntimeError("; ".join(fetch_errors))

    def _get_or_create_instance(
        self,
        stream_class_name: str,
        *,
        proxy: str | None,
        cookies: str | None,
        username: str | None,
        password: str | None,
        account_type: str | None,
    ) -> Any:
        key = (stream_class_name, proxy, cookies)
        if key in self._instances:
            return self._instances[key]

        stream_class = getattr(streamget, stream_class_name)
        signature = inspect.signature(stream_class.__init__)
        kwargs: dict[str, Any] = {
            "proxy_addr": proxy,
            "cookies": cookies,
            "username": username,
            "password": password,
            "account_type": account_type,
        }
        filtered_kwargs = {
            name: value
            for name, value in kwargs.items()
            if value is not None and name in signature.parameters
        }
        instance = stream_class(**filtered_kwargs)
        self._instances[key] = instance
        return instance

    @staticmethod
    def _probe_custom(live_url: str) -> StreamData:
        stream_data = StreamData(
            platform="自定义直播流",
            anchor_name="直播间",
            is_live=True,
            title="自定义直播流",
            record_url=live_url,
            live_url=live_url,
        )
        if ".flv" in live_url:
            stream_data.flv_url = live_url
        if ".m3u8" in live_url:
            stream_data.m3u8_url = live_url
        return stream_data
