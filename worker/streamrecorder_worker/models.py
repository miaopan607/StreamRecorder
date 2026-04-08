from __future__ import annotations

from dataclasses import asdict, dataclass, field
from datetime import UTC, datetime, timedelta
from typing import Any
from urllib.parse import urlparse
from uuid import uuid4

from .platforms import get_platform_info

DEFAULT_STREAMER_NAME = "直播间"
STATUS_STOPPED = "已停止"
STATUS_MONITORING = "监控中"
STATUS_CHECKING = "检测中"
STATUS_RECORDING = "录制中"
STATUS_ERROR = "错误"
STATUS_WAITING = "未开播"

QUALITY_TEXT_MAP: dict[str, str] = {
    "OD": "原画",
    "UHD": "超清",
    "HD": "高清",
    "SD": "标清",
    "LD": "流畅",
}


def get_quality_text(quality: str) -> str:
    return QUALITY_TEXT_MAP.get((quality or "").upper(), quality or "原画")


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def is_valid_url(url: str) -> bool:
    try:
        parsed = urlparse(url)
    except ValueError:
        return False
    return bool(parsed.scheme and parsed.netloc)


@dataclass(slots=True)
class RecordingJob:
    id: str = field(default_factory=lambda: uuid4().hex)
    url: str = ""
    streamer_name: str = DEFAULT_STREAMER_NAME
    quality: str = "OD"
    record_format: str = "TS"
    segment_record: bool = False
    segment_time: str = "1800"
    monitor_status: bool = True
    scheduled_recording: bool = False
    scheduled_start_time: str = ""
    monitor_hours: str = "5"
    recording_dir: str = ""
    enabled_message_push: bool = True
    only_notify_no_record: bool = False
    flv_use_direct_download: bool = False
    platform: str = "未知平台"
    platform_key: str = "unknown"
    status_info: str = STATUS_MONITORING
    display_title: str = ""
    title: str = ""
    error_message: str = ""
    created_at: str = field(default_factory=utc_now)
    updated_at: str = field(default_factory=utc_now)
    last_checked_at: str = ""
    live_title: str = ""
    record_url: str = ""
    latest_output_path: str = ""
    duration_text: str = "00:00:00"
    speed_text: str = "X KB/s"
    recording_started_at: str = ""

    @classmethod
    def from_dict(cls, payload: dict[str, Any]) -> "RecordingJob":
        job = cls(
            id=str(payload.get("id") or payload.get("rec_id") or uuid4().hex),
            url=str(payload.get("url") or "").strip(),
            streamer_name=str(
                payload.get("streamer_name") or DEFAULT_STREAMER_NAME
            ).strip()
            or DEFAULT_STREAMER_NAME,
            quality=str(payload.get("quality") or "OD").upper(),
            record_format=str(
                payload.get("record_format") or payload.get("recordFormat") or "TS"
            ).upper(),
            segment_record=bool(
                payload.get("segment_record", payload.get("segmentRecord", False))
            ),
            segment_time=str(
                payload.get("segment_time") or payload.get("segmentTime") or "1800"
            ),
            monitor_status=bool(
                payload.get("monitor_status", payload.get("monitorStatus", True))
            ),
            scheduled_recording=bool(
                payload.get(
                    "scheduled_recording", payload.get("scheduledRecording", False)
                )
            ),
            scheduled_start_time=str(
                payload.get("scheduled_start_time")
                or payload.get("scheduledStartTime")
                or ""
            ),
            monitor_hours=str(
                payload.get("monitor_hours") or payload.get("monitorHours") or "5"
            ),
            recording_dir=str(
                payload.get("recording_dir") or payload.get("recordingDir") or ""
            ),
            enabled_message_push=bool(
                payload.get(
                    "enabled_message_push", payload.get("enabledMessagePush", True)
                )
            ),
            only_notify_no_record=bool(
                payload.get(
                    "only_notify_no_record", payload.get("onlyNotifyNoRecord", False)
                )
            ),
            flv_use_direct_download=bool(
                payload.get(
                    "flv_use_direct_download",
                    payload.get("flvUseDirectDownload", False),
                )
            ),
            created_at=str(
                payload.get("created_at") or payload.get("createdAt") or utc_now()
            ),
            updated_at=str(
                payload.get("updated_at") or payload.get("updatedAt") or utc_now()
            ),
            last_checked_at=str(
                payload.get("last_checked_at") or payload.get("lastCheckedAt") or ""
            ),
        )
        job.refresh_derived_fields(
            existing_status=str(
                payload.get("status_info") or payload.get("statusInfo") or ""
            )
        )
        job.error_message = str(
            payload.get("error_message") or payload.get("errorMessage") or ""
        )
        job.live_title = str(
            payload.get("live_title") or payload.get("liveTitle") or ""
        )
        job.record_url = str(
            payload.get("record_url") or payload.get("recordUrl") or ""
        )
        job.latest_output_path = str(
            payload.get("latest_output_path") or payload.get("latestOutputPath") or ""
        )
        job.duration_text = str(
            payload.get("duration_text") or payload.get("durationText") or "00:00:00"
        )
        job.speed_text = str(
            payload.get("speed_text") or payload.get("speedText") or "X KB/s"
        )
        job.recording_started_at = str(
            payload.get("recording_started_at")
            or payload.get("recordingStartedAt")
            or ""
        )
        return job

    def refresh_derived_fields(self, existing_status: str = "") -> None:
        platform, platform_key = get_platform_info(self.url)
        if platform and platform_key:
            self.platform = platform
            self.platform_key = platform_key
        self.title = f"{self.streamer_name} - {get_quality_text(self.quality)}"
        self.display_title = self.title
        self.updated_at = utc_now()
        if existing_status:
            self.status_info = existing_status
        else:
            self.status_info = (
                STATUS_MONITORING if self.monitor_status else STATUS_STOPPED
            )

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


def format_duration(elapsed: timedelta | None) -> str:
    if elapsed is None:
        return "00:00:00"
    total_seconds = max(0, int(elapsed.total_seconds()))
    hours = total_seconds // 3600
    minutes = (total_seconds % 3600) // 60
    seconds = total_seconds % 60
    return f"{hours:02d}:{minutes:02d}:{seconds:02d}"


DEFAULT_SETTINGS: dict[str, Any] = {
    "language": "Chinese",
    "live_save_path": "",
    "filename_includes_title": False,
    "custom_filename_template": "{anchor_name}_{title}_{time}",
    "remove_emojis": False,
    "folder_name_platform": True,
    "folder_name_author": True,
    "folder_name_time": False,
    "folder_name_title": False,
    "enable_proxy": False,
    "proxy_address": "",
    "video_format": "TS",
    "record_quality": "OD",
    "loop_time_seconds": "180",
    "segmented_recording_enabled": True,
    "force_https_recording": True,
    "default_live_source": "FLV",
    "flv_use_direct_download": False,
    "recording_space_threshold": "2.0",
    "video_segment_time": "1800",
    "convert_to_mp4": True,
    "delete_original": False,
    "generate_time_subtitle_file": False,
    "execute_custom_script": False,
    "custom_script_command": "",
    "default_platform_with_proxy": "",
    "system_notification_enabled": True,
    "system_stream_start_notification_enabled": True,
    "system_stream_end_notification_enabled": False,
    "system_error_notification_enabled": False,
    "system_minimize_to_tray_notification_enabled": False,
    "system_close_to_tray_notification_enabled": False,
    "stream_start_notification_enabled": False,
    "stream_end_notification_enabled": False,
    "stream_error_notification_enabled": False,
    "only_notify_no_record": False,
    "custom_notification_title": "",
    "custom_stream_start_content": "",
    "custom_stream_end_content": "",
    "custom_stream_error_content": "",
    "dingtalk_enabled": False,
    "wechat_enabled": False,
    "bark_enabled": False,
    "ntfy_enabled": False,
    "serverchan_enabled": False,
    "telegram_enabled": False,
    "email_enabled": False,
    "dingtalk_webhook_url": "",
    "dingtalk_at_objects": "",
    "dingtalk_at_all": False,
    "wechat_webhook_url": "",
    "bark_webhook_url": "",
    "bark_interrupt_level": "active",
    "bark_sound": "",
    "ntfy_server_url": "https://ntfy.sh/xxxxx",
    "ntfy_tags": "tada",
    "ntfy_email": "",
    "ntfy_action_url": "",
    "serverchan_sendkey": "",
    "serverchan_channel": "9",
    "serverchan_tags": "直播通知",
    "telegram_api_token": "",
    "telegram_chat_id": "",
    "smtp_server": "smtp.qq.com",
    "email_username": "",
    "email_password": "",
    "sender_email": "",
    "sender_name": "",
    "recipient_email": "",
    "theme_color": "blue",
    "is_grid_view": True,
    "theme_mode": "light",
    "platform_max_concurrent_requests": "3",
    "minimize_to_tray_on_minimize": False,
    "minimize_to_tray_on_close": True,
}
