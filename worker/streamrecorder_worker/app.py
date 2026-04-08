from __future__ import annotations

import asyncio
import contextlib
import os
import shutil
import signal
import subprocess
import sys
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from . import __version__
from .config import WorkerConfigStore
from .ffmpeg import build_ffmpeg_command, build_save_path
from .models import (
    RecordingJob,
    STATUS_CHECKING,
    STATUS_ERROR,
    STATUS_MONITORING,
    STATUS_RECORDING,
    STATUS_STOPPED,
    STATUS_WAITING,
    format_duration,
    is_valid_url,
    utc_now,
)
from .notification_service import NotificationService
from .probe import StreamProbeService
from .protocol import JsonLineWriter, error, event, ok


@dataclass(slots=True)
class ActiveRecording:
    job_id: str
    process: asyncio.subprocess.Process
    output_path: str
    record_url: str
    watcher: asyncio.Task[None]
    started_at: datetime
    last_measure_time: datetime
    last_measure_bytes: int


class StreamRecorderWorkerApp:
    def __init__(self, data_root: Path) -> None:
        self.store = WorkerConfigStore(data_root)
        self.writer = JsonLineWriter()
        self.settings = self.store.load_settings()
        self.cookies = self.store.load_cookies()
        self.accounts = self.store.load_accounts()
        self.jobs = [RecordingJob.from_dict(item) for item in self.store.load_jobs()]
        self.probe_service = StreamProbeService()
        self.notification_service = NotificationService()
        self.should_exit = False
        self.monitor_loop_task: asyncio.Task[None] | None = None
        self.active_recordings: dict[str, ActiveRecording] = {}
        self.checking_jobs: set[str] = set()

    def snapshot(self) -> dict[str, Any]:
        return {
            "app": {
                "name": "StreamRecorder 核心服务",
                "version": __version__,
                "python_version": sys.version.split()[0],
                "ffmpeg_available": shutil.which("ffmpeg") is not None,
                "node_available": shutil.which("node") is not None,
                "updated_at": utc_now(),
            },
            "settings": self.settings,
            "jobs": [job.to_dict() for job in self.jobs],
        }

    async def publish_snapshot(self) -> None:
        await self.writer.write(event("snapshot_changed", self.snapshot()))

    async def publish_health(self) -> None:
        await self.writer.write(event("core_health", self.snapshot()["app"]))

    def log(self, message: str) -> None:
        sys.stderr.write(message + "\n")
        sys.stderr.flush()

    def persist(self) -> None:
        self.store.save_settings(self.settings)
        self.store.save_cookies(self.cookies)
        self.store.save_accounts(self.accounts)
        self.store.save_jobs([job.to_dict() for job in self.jobs])

    async def dispatch(self, payload: dict[str, Any]) -> dict[str, Any]:
        message_id = str(payload.get("id") or "")
        method = str(payload.get("method") or "")
        body = payload.get("body") or {}

        if not message_id or payload.get("kind") != "cmd":
            raise ValueError("Invalid command envelope.")

        if method == "initialize":
            return ok(message_id, self.snapshot())
        if method == "get_snapshot":
            return ok(message_id, self.snapshot())
        if method == "health.ping":
            return ok(message_id, {"utc": utc_now()})
        if method == "settings.get":
            return ok(message_id, {"settings": self.settings})
        if method == "settings.update":
            return ok(message_id, await self._update_settings(body))
        if method == "cookies.get":
            return ok(message_id, {"cookies": self.cookies})
        if method == "cookies.update":
            return ok(message_id, await self._update_cookies(body))
        if method == "accounts.get":
            return ok(message_id, {"accounts": self.accounts})
        if method == "accounts.update":
            return ok(message_id, await self._update_accounts(body))
        if method == "dependencies.get":
            return ok(message_id, self._get_dependencies())
        if method == "jobs.upsert":
            return ok(message_id, await self._upsert_jobs(body))
        if method == "jobs.delete":
            return ok(message_id, await self._delete_jobs(body))
        if method == "jobs.start_monitoring":
            return ok(message_id, await self._set_monitoring(body, True))
        if method == "jobs.stop_monitoring":
            return ok(message_id, await self._set_monitoring(body, False))
        if method == "jobs.recheck":
            return ok(message_id, await self._recheck_jobs(body))
        if method == "core.shutdown":
            self.should_exit = True
            return ok(message_id, {"accepted": True})

        raise ValueError(f"Unsupported method: {method}")

    async def _update_settings(self, body: dict[str, Any]) -> dict[str, Any]:
        incoming_settings = body.get("settings") or {}
        for key, value in incoming_settings.items():
            if key in self.settings:
                self.settings[key] = value
        self.persist()
        await self.publish_snapshot()
        return {"settings": self.settings}

    async def _update_cookies(self, body: dict[str, Any]) -> dict[str, Any]:
        incoming_cookies = body.get("cookies") or {}
        self.cookies = {
            str(key): str(value or "") for key, value in incoming_cookies.items()
        }
        self.persist()
        return {"cookies": self.cookies}

    async def _update_accounts(self, body: dict[str, Any]) -> dict[str, Any]:
        incoming_accounts = body.get("accounts") or {}
        self.accounts = incoming_accounts if isinstance(incoming_accounts, dict) else {}
        self.persist()
        return {"accounts": self.accounts}

    def _get_dependencies(self) -> dict[str, Any]:
        return {
            "python": {
                "available": True,
                "version": sys.version.split()[0],
                "path": sys.executable,
            },
            "ffmpeg": {
                "available": shutil.which("ffmpeg") is not None,
                "path": shutil.which("ffmpeg") or "",
            },
            "node": {
                "available": shutil.which("node") is not None,
                "path": shutil.which("node") or "",
            },
        }

    async def _upsert_jobs(self, body: dict[str, Any]) -> dict[str, Any]:
        raw_jobs = body.get("jobs") or []
        changed_ids: list[str] = []

        for raw_job in raw_jobs:
            job = RecordingJob.from_dict(raw_job)
            if not is_valid_url(job.url):
                raise ValueError(f"Invalid live URL: {job.url}")

            existing = self._find_job(job.id)
            if existing is None:
                duplicate = self._find_job_by_url(job.url)
                if duplicate is not None:
                    existing = duplicate

            if existing is None:
                self.jobs.append(job)
                changed_ids.append(job.id)
                continue

            existing.url = job.url
            existing.streamer_name = job.streamer_name
            existing.quality = job.quality
            existing.record_format = job.record_format
            existing.segment_record = job.segment_record
            existing.segment_time = job.segment_time
            existing.monitor_status = job.monitor_status
            existing.scheduled_recording = job.scheduled_recording
            existing.scheduled_start_time = job.scheduled_start_time
            existing.monitor_hours = job.monitor_hours
            existing.recording_dir = job.recording_dir
            existing.enabled_message_push = job.enabled_message_push
            existing.only_notify_no_record = job.only_notify_no_record
            existing.flv_use_direct_download = job.flv_use_direct_download
            existing.refresh_derived_fields(existing.status_info or STATUS_MONITORING)
            changed_ids.append(existing.id)

        self.persist()
        await self.publish_snapshot()
        if changed_ids:
            asyncio.create_task(self._recheck_job_ids(changed_ids, publish=True))
        return {"changed_ids": changed_ids}

    async def _delete_jobs(self, body: dict[str, Any]) -> dict[str, Any]:
        ids = {str(item) for item in body.get("ids") or []}
        for job in list(self.jobs):
            if job.id in ids:
                await self._stop_recording_if_needed(job)
        before = len(self.jobs)
        self.jobs = [job for job in self.jobs if job.id not in ids]
        deleted = before - len(self.jobs)
        self.persist()
        await self.publish_snapshot()
        return {"deleted": deleted}

    async def _set_monitoring(
        self, body: dict[str, Any], enabled: bool
    ) -> dict[str, Any]:
        ids = {str(item) for item in body.get("ids") or []}
        changed_ids: list[str] = []
        target_status = STATUS_MONITORING if enabled else STATUS_STOPPED
        for job in self.jobs:
            if ids and job.id not in ids:
                continue
            job.monitor_status = enabled
            job.status_info = target_status
            job.error_message = ""
            job.last_checked_at = utc_now()
            job.refresh_derived_fields(target_status)
            changed_ids.append(job.id)
            if not enabled:
                await self._stop_recording_if_needed(job)
        self.persist()
        await self.publish_snapshot()
        if enabled:
            asyncio.create_task(self._recheck_job_ids(changed_ids, publish=True))
        return {"changed_ids": changed_ids, "monitoring": enabled}

    async def _recheck_jobs(self, body: dict[str, Any]) -> dict[str, Any]:
        ids = [str(item) for item in body.get("ids") or []]
        checked_ids = await self._recheck_job_ids(ids, publish=True)
        return {"checked_ids": checked_ids}

    def _find_job(self, job_id: str) -> RecordingJob | None:
        for job in self.jobs:
            if job.id == job_id:
                return job
        return None

    def _find_job_by_url(self, url: str) -> RecordingJob | None:
        normalized = url.strip().lower()
        for job in self.jobs:
            if job.url.strip().lower() == normalized:
                return job
        return None

    async def heartbeat_loop(self) -> None:
        while not self.should_exit:
            await asyncio.sleep(10)
            if await self._refresh_active_recording_stats():
                self.persist()
                await self.publish_snapshot()
            await self.publish_health()

    async def monitor_loop(self) -> None:
        while not self.should_exit:
            try:
                if await self._refresh_active_recording_stats():
                    self.persist()
                    await self.publish_snapshot()
                await self._monitor_tick()
            except Exception as exc:  # noqa: BLE001
                self.log(f"monitor loop error: {exc}")
            await asyncio.sleep(2)

    async def _refresh_active_recording_stats(self) -> bool:
        changed = False
        now = datetime.now(UTC)
        for job_id, active in list(self.active_recordings.items()):
            job = self._find_job(job_id)
            if job is None:
                continue

            total_bytes = self._get_output_size_bytes(active.output_path)
            elapsed = now - active.started_at
            delta_seconds = max((now - active.last_measure_time).total_seconds(), 1)
            delta_bytes = max(0, total_bytes - active.last_measure_bytes)
            speed_kb = delta_bytes / delta_seconds / 1024
            speed_text = (
                f"{speed_kb:.1f} KB/s"
                if speed_kb < 1024
                else f"{speed_kb / 1024:.2f} MB/s"
            )
            duration_text = format_duration(elapsed)

            if job.duration_text != duration_text or job.speed_text != speed_text:
                job.duration_text = duration_text
                job.speed_text = speed_text
                changed = True

            active.last_measure_time = now
            active.last_measure_bytes = total_bytes
        return changed

    @staticmethod
    def _get_output_size_bytes(output_path: str) -> int:
        if "%03d" in output_path:
            directory = Path(output_path).parent
            pattern = Path(output_path).name.replace("%03d", "*")
            return sum(
                path.stat().st_size
                for path in directory.glob(pattern)
                if path.is_file()
            )

        output_file = Path(output_path)
        return output_file.stat().st_size if output_file.exists() else 0

    async def _monitor_tick(self) -> None:
        now = datetime.now(UTC)
        interval_seconds = self._get_loop_interval_seconds()
        for job in self.jobs:
            if not job.monitor_status:
                continue
            if job.id in self.active_recordings or job.id in self.checking_jobs:
                continue
            if not self._should_check_job(job, now, interval_seconds):
                continue
            await self._check_job(job, publish=True)

    def _get_loop_interval_seconds(self) -> int:
        raw = str(self.settings.get("loop_time_seconds") or "180").strip()
        try:
            return max(10, int(float(raw)))
        except ValueError:
            return 180

    @staticmethod
    def _should_check_job(
        job: RecordingJob, now: datetime, interval_seconds: int
    ) -> bool:
        if not job.last_checked_at:
            return True
        try:
            last = datetime.fromisoformat(job.last_checked_at)
        except ValueError:
            return True
        return (now - last).total_seconds() >= interval_seconds

    async def _recheck_job_ids(self, ids: list[str], publish: bool) -> list[str]:
        checked_ids: list[str] = []
        id_set = set(ids)
        for job in self.jobs:
            if id_set and job.id not in id_set:
                continue
            if not job.monitor_status:
                continue
            await self._check_job(job, publish=publish)
            checked_ids.append(job.id)
        if publish:
            await self.publish_snapshot()
        return checked_ids

    async def _check_job(self, job: RecordingJob, publish: bool) -> None:
        if job.id in self.checking_jobs or job.id in self.active_recordings:
            return

        self.checking_jobs.add(job.id)
        try:
            job.status_info = STATUS_CHECKING
            job.error_message = ""
            job.last_checked_at = utc_now()
            job.updated_at = utc_now()
            if publish:
                self.persist()
                await self.publish_snapshot()

            self.log(f"开始检测: {job.url}")
            stream_info = await self.probe_service.probe(
                platform_key=job.platform_key,
                live_url=job.url,
                quality=job.quality,
                proxy=self._get_proxy_for_job(job),
                cookies=self.cookies.get(job.platform_key) or None,
                username=self._get_account_field(job.platform_key, "username"),
                password=self._get_account_field(job.platform_key, "password"),
                account_type=self._get_account_field(job.platform_key, "account_type"),
            )

            if getattr(stream_info, "new_cookies", None):
                self.cookies[job.platform_key] = stream_info.new_cookies

            job.streamer_name = (
                stream_info.anchor_name or job.streamer_name or "直播间"
            ).strip() or "直播间"
            job.live_title = (stream_info.title or "").strip()

            if stream_info.is_live:
                self.log(f"检测到开播: {job.url}")
                await self._start_recording(job, stream_info)
            else:
                self.log(f"未开播: {job.url}")
                job.status_info = (
                    STATUS_WAITING if job.monitor_status else STATUS_STOPPED
                )
                job.record_url = ""
                job.refresh_derived_fields(job.status_info)
        except Exception as exc:  # noqa: BLE001
            job.status_info = STATUS_ERROR
            job.error_message = str(exc)
            job.updated_at = utc_now()
            self.log(f"检测失败: {job.url} | {exc}")
        finally:
            self.checking_jobs.discard(job.id)
            self.persist()
            if publish:
                await self.publish_snapshot()

    def _get_proxy_for_job(self, job: RecordingJob) -> str | None:
        if not self.settings.get("enable_proxy"):
            return None
        proxy = str(self.settings.get("proxy_address") or "").strip()
        if not proxy:
            return None
        platforms_text = str(self.settings.get("default_platform_with_proxy") or "")
        platform_list = {
            item.strip().lower()
            for item in platforms_text.replace("，", ",").split(",")
            if item.strip()
        }
        if platform_list and job.platform_key.lower() not in platform_list:
            return None
        return proxy if proxy.startswith("http") else f"http://{proxy}"

    def _get_account_field(self, platform_key: str, field_name: str) -> str | None:
        nested = self.accounts.get(platform_key)
        if isinstance(nested, dict):
            value = nested.get(field_name)
            return str(value) if value not in (None, "") else None

        legacy_key = f"{platform_key}_{field_name}"
        value = self.accounts.get(legacy_key)
        return str(value) if value not in (None, "") else None

    async def _start_recording(self, job: RecordingJob, stream_info: Any) -> None:
        if shutil.which("ffmpeg") is None:
            raise RuntimeError("未检测到 ffmpeg，无法开始录制")

        if not self._has_enough_disk_space(job):
            raise RuntimeError("存储空间不足，已跳过录制")

        save_format = self._get_job_format(job, stream_info)
        output_dir = self._build_output_dir(job, stream_info)
        output_dir.mkdir(parents=True, exist_ok=True)
        base_name = self._build_base_name(job, stream_info)
        save_path = build_save_path(
            base_name, output_dir, save_format, job.segment_record
        )
        record_url = self._select_record_url(job, stream_info)
        ffmpeg_command = build_ffmpeg_command(
            record_url,
            save_path,
            save_format,
            segment_record=job.segment_record,
            segment_time=job.segment_time,
            proxy=self._get_proxy_for_job(job),
        )
        process = await asyncio.create_subprocess_exec(
            *ffmpeg_command,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.DEVNULL,
            stderr=asyncio.subprocess.PIPE,
        )
        job.record_url = record_url
        job.latest_output_path = str(save_path)
        job.recording_dir = str(output_dir)
        job.status_info = STATUS_RECORDING
        job.error_message = ""
        job.recording_started_at = utc_now()
        job.duration_text = "00:00:00"
        job.speed_text = "0 KB/s"
        job.refresh_derived_fields(STATUS_RECORDING)
        self.log(f"开始录制: {job.url} -> {save_path}")
        await self._notify_event(job, "start")
        watcher = asyncio.create_task(self._watch_recording(job.id, process))
        started_at = datetime.now(UTC)
        self.active_recordings[job.id] = ActiveRecording(
            job_id=job.id,
            process=process,
            output_path=str(save_path),
            record_url=record_url,
            watcher=watcher,
            started_at=started_at,
            last_measure_time=started_at,
            last_measure_bytes=0,
        )

    async def _watch_recording(
        self, job_id: str, process: asyncio.subprocess.Process
    ) -> None:
        stderr_task = (
            asyncio.create_task(process.stderr.readline()) if process.stderr else None
        )
        try:
            await process.wait()
            error_line = ""
            if stderr_task is not None:
                with contextlib.suppress(Exception):
                    stderr_bytes = await asyncio.wait_for(stderr_task, timeout=0.2)
                    error_line = stderr_bytes.decode("utf-8", errors="ignore").strip()

            job = self._find_job(job_id)
            if job is None:
                return

            if process.returncode in {0, 255}:
                job.status_info = (
                    STATUS_MONITORING if job.monitor_status else STATUS_STOPPED
                )
                job.error_message = ""
                job.speed_text = "X KB/s"
                self.log(f"录制结束: {job.url}")
                await self._notify_event(job, "end")
                await self._run_post_actions(job)
            else:
                job.status_info = STATUS_ERROR
                job.error_message = error_line or f"ffmpeg 退出码: {process.returncode}"
                job.speed_text = "X KB/s"
                self.log(f"录制异常结束: {job.url} | {job.error_message}")
                await self._notify_event(job, "error")

            job.updated_at = utc_now()
            self.persist()
            await self.publish_snapshot()
        finally:
            self.active_recordings.pop(job_id, None)
            if stderr_task is not None and not stderr_task.done():
                stderr_task.cancel()

    async def _stop_recording_if_needed(self, job: RecordingJob) -> None:
        active = self.active_recordings.get(job.id)
        if active is None:
            return

        process = active.process
        if process.returncode is not None:
            return

        try:
            if os.name == "nt":
                if process.stdin is not None:
                    process.stdin.write(b"q")
                    await process.stdin.drain()
                await asyncio.wait_for(process.wait(), timeout=8)
            else:
                process.send_signal(signal.SIGINT)
                await asyncio.wait_for(process.wait(), timeout=8)
        except Exception:
            process.kill()
            await process.wait()

    def _build_output_dir(self, job: RecordingJob, stream_info: Any) -> Path:
        if job.recording_dir:
            return self._resolve_output_path(job.recording_dir)

        base_dir_text = str(self.settings.get("live_save_path") or "").strip()
        if not base_dir_text:
            base_dir = self.store.data_root / "recordings"
        else:
            base_dir = self._resolve_output_path(base_dir_text)

        anchor_name = self._sanitize_name(
            stream_info.anchor_name or job.streamer_name or "直播间"
        )
        platform_name = self._sanitize_name(
            job.platform or stream_info.platform or "直播"
        )
        folder = base_dir
        if self.settings.get("folder_name_platform"):
            folder = folder / platform_name
        if self.settings.get("folder_name_author"):
            folder = folder / anchor_name
        if self.settings.get("folder_name_time"):
            folder = folder / datetime.now().strftime("%Y-%m-%d")
        if self.settings.get("folder_name_title") and getattr(
            stream_info, "title", None
        ):
            folder = folder / self._sanitize_name(stream_info.title)
        return folder

    def _resolve_output_path(self, path_text: str) -> Path:
        path = Path(path_text).expanduser()
        if path.is_absolute():
            return path
        return (self.store.data_root.parent / path).resolve(strict=False)

    def _build_base_name(self, job: RecordingJob, stream_info: Any) -> str:
        now_text = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
        anchor_name = self._sanitize_name(
            stream_info.anchor_name or job.streamer_name or "直播间"
        )
        title = self._sanitize_name(getattr(stream_info, "title", "") or "")
        template = str(
            self.settings.get("custom_filename_template")
            or "{anchor_name}_{title}_{time}"
        )
        if not self.settings.get("filename_includes_title"):
            title = ""
        base_name = (
            template.replace("{anchor_name}", anchor_name)
            .replace("{title}", title)
            .replace("{time}", now_text)
            .replace("{platform}", self._sanitize_name(job.platform))
        )
        base_name = "_".join(part for part in base_name.split("_") if part)
        return base_name or f"{anchor_name}_{now_text}"

    def _select_record_url(self, job: RecordingJob, stream_info: Any) -> str:
        default_source = str(self.settings.get("default_live_source") or "FLV").upper()
        flv_url = getattr(stream_info, "flv_url", None) or ""
        m3u8_url = getattr(stream_info, "m3u8_url", None) or ""
        record_url = getattr(stream_info, "record_url", None) or ""
        chosen = ""
        if default_source == "HLS":
            chosen = m3u8_url or record_url or flv_url
        else:
            chosen = flv_url or record_url or m3u8_url

        if not chosen:
            raise RuntimeError("没有可用的直播流地址")

        if self.settings.get("force_https_recording") and chosen.startswith("http://"):
            chosen = "https://" + chosen[len("http://") :]

        if job.platform_key in {"shopee", "migu"}:
            chosen = chosen.replace("https://", "http://")
        return chosen

    def _get_job_format(self, job: RecordingJob, stream_info: Any) -> str:
        save_format = (
            job.record_format or self.settings.get("video_format") or "TS"
        ).lower()
        flv_url = getattr(stream_info, "flv_url", None) or ""
        if flv_url and save_format == "flv":
            return "flv"
        return save_format

    @staticmethod
    def _sanitize_name(text: str) -> str:
        value = (text or "").strip()
        for bad in '\\/:*?"<>|':
            value = value.replace(bad, "_")
        value = value.replace("，", "_").replace("。", "_").replace(" ", "_")
        while "__" in value:
            value = value.replace("__", "_")
        return value.strip("_")[:60] or "直播间"

    def _has_enough_disk_space(self, job: RecordingJob) -> bool:
        threshold_text = str(self.settings.get("recording_space_threshold") or "2.0")
        try:
            threshold = float(threshold_text)
        except ValueError:
            threshold = 2.0

        target_dir = (
            Path(job.recording_dir)
            if job.recording_dir
            else (self.store.data_root / "recordings")
        )
        target_dir.mkdir(parents=True, exist_ok=True)
        free_gb = shutil.disk_usage(target_dir).free / (1024**3)
        return free_gb >= threshold

    async def _notify_event(self, job: RecordingJob, message_type: str) -> None:
        title, content = self._build_notification_message(job, message_type)
        should_notify_desktop = self.settings.get("system_notification_enabled") and (
            (
                message_type == "start"
                and self.settings.get("system_stream_start_notification_enabled", True)
            )
            or (
                message_type == "end"
                and self.settings.get("system_stream_end_notification_enabled", True)
            )
            or (
                message_type == "error"
                and self.settings.get("system_error_notification_enabled", True)
            )
        )
        if should_notify_desktop:
            await self.writer.write(
                event("desktop_notification", {"title": title, "message": content})
            )

        if not self._is_any_push_channel_enabled():
            return

        should_push = (
            (
                message_type == "start"
                and self.settings.get("stream_start_notification_enabled")
            )
            or (
                message_type == "end"
                and self.settings.get("stream_end_notification_enabled")
            )
            or (
                message_type == "error"
                and self.settings.get("stream_error_notification_enabled")
            )
        )
        if should_push:
            await self._push_messages(title, content)

    def _build_notification_message(
        self, job: RecordingJob, message_type: str
    ) -> tuple[str, str]:
        custom_title = str(self.settings.get("custom_notification_title") or "").strip()
        default_title = f"StreamRecorder - {job.streamer_name}"
        title = custom_title or default_title
        if message_type == "start":
            template = str(
                self.settings.get("custom_stream_start_content") or ""
            ).strip()
            default_content = f"检测到 {job.streamer_name} 正在直播，已开始录制。"
        elif message_type == "end":
            template = str(self.settings.get("custom_stream_end_content") or "").strip()
            default_content = f"{job.streamer_name} 的录制已结束。"
        else:
            template = str(
                self.settings.get("custom_stream_error_content") or ""
            ).strip()
            default_content = f"{job.streamer_name} 的录制异常结束：{job.error_message or '请查看日志。'}"
        content = template or default_content
        return (
            title,
            content.replace("{streamer_name}", job.streamer_name)
            .replace("{title}", job.live_title or job.title)
            .replace("{error}", job.error_message or ""),
        )

    def _is_any_push_channel_enabled(self) -> bool:
        return any(
            self.settings.get(key)
            for key in (
                "dingtalk_enabled",
                "wechat_enabled",
                "bark_enabled",
                "ntfy_enabled",
                "telegram_enabled",
                "email_enabled",
                "serverchan_enabled",
            )
        )

    async def _push_messages(self, title: str, content: str) -> None:
        proxy = self._get_proxy_for_push()
        if self.settings.get("dingtalk_enabled"):
            await self.notification_service.send_to_dingtalk(
                url=str(self.settings.get("dingtalk_webhook_url") or ""),
                content=content,
                number=str(self.settings.get("dingtalk_at_objects") or ""),
                is_atall=bool(self.settings.get("dingtalk_at_all")),
            )
        if self.settings.get("wechat_enabled"):
            await self.notification_service.send_to_wechat(
                url=str(self.settings.get("wechat_webhook_url") or ""),
                title=title,
                content=content,
            )
        if self.settings.get("bark_enabled"):
            await self.notification_service.send_to_bark(
                api=str(self.settings.get("bark_webhook_url") or ""),
                title=title,
                content=content,
                level=str(self.settings.get("bark_interrupt_level") or "active"),
                sound=str(self.settings.get("bark_sound") or ""),
            )
        if self.settings.get("ntfy_enabled"):
            await self.notification_service.send_to_ntfy(
                api=str(self.settings.get("ntfy_server_url") or ""),
                title=title,
                content=content,
                tags=str(self.settings.get("ntfy_tags") or "tada"),
                action_url=str(self.settings.get("ntfy_action_url") or ""),
                email=str(self.settings.get("ntfy_email") or ""),
            )
        if self.settings.get("telegram_enabled"):
            await self.notification_service.send_to_telegram(
                chat_id=str(self.settings.get("telegram_chat_id") or ""),
                token=str(self.settings.get("telegram_api_token") or ""),
                content=content,
                proxy=proxy,
            )
        if self.settings.get("email_enabled"):
            await self.notification_service.send_to_email(
                email_host=str(self.settings.get("smtp_server") or "smtp.qq.com"),
                login_email=str(self.settings.get("email_username") or ""),
                password=str(self.settings.get("email_password") or ""),
                sender_email=str(self.settings.get("sender_email") or ""),
                sender_name=str(self.settings.get("sender_name") or ""),
                to_email=str(self.settings.get("recipient_email") or ""),
                title=title,
                content=content,
            )
        if self.settings.get("serverchan_enabled"):
            await self.notification_service.send_to_serverchan(
                sendkey=str(self.settings.get("serverchan_sendkey") or ""),
                title=title,
                content=content,
                channel=str(self.settings.get("serverchan_channel") or "9"),
                tags=str(self.settings.get("serverchan_tags") or "直播通知"),
            )

    def _get_proxy_for_push(self) -> str | None:
        if not self.settings.get("enable_proxy"):
            return None
        proxy = str(self.settings.get("proxy_address") or "").strip()
        if not proxy:
            return None
        return proxy if proxy.startswith("http") else f"http://{proxy}"

    async def _run_post_actions(self, job: RecordingJob) -> None:
        output_path = Path(job.latest_output_path) if job.latest_output_path else None
        if (
            output_path
            and output_path.suffix.lower() == ".ts"
            and self.settings.get("convert_to_mp4")
        ):
            await self._convert_ts_outputs(job)
        if self.settings.get("execute_custom_script"):
            await self._execute_custom_script(job)

    async def _convert_ts_outputs(self, job: RecordingJob) -> None:
        output_path = Path(job.latest_output_path)
        candidates: list[Path]
        if job.segment_record:
            prefix = output_path.stem.rsplit("_", 1)[0]
            candidates = sorted(output_path.parent.glob(f"{prefix}_*.ts"))
        else:
            candidates = [output_path] if output_path.exists() else []
        for source_path in candidates:
            target_path = source_path.with_suffix(".mp4")
            command = [
                "ffmpeg",
                "-y",
                "-i",
                str(source_path),
                "-c",
                "copy",
                str(target_path),
            ]
            process = await asyncio.create_subprocess_exec(
                *command,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            await process.wait()
            if process.returncode == 0 and self.settings.get("delete_original"):
                with contextlib.suppress(Exception):
                    source_path.unlink()

    async def _execute_custom_script(self, job: RecordingJob) -> None:
        command = str(self.settings.get("custom_script_command") or "").strip()
        if not command:
            return
        env = os.environ.copy()
        env.update(
            {
                "STREAMRECORDER_JOB_ID": job.id,
                "STREAMRECORDER_STREAMER_NAME": job.streamer_name,
                "STREAMRECORDER_TITLE": job.live_title or job.title,
                "STREAMRECORDER_OUTPUT": job.latest_output_path,
                "STREAMRECORDER_URL": job.url,
            }
        )
        process = await asyncio.create_subprocess_shell(
            command,
            env=env,
            stdout=asyncio.subprocess.DEVNULL,
            stderr=asyncio.subprocess.DEVNULL,
        )
        await process.wait()

    async def serve_stdio(self) -> None:
        await self.publish_health()
        await self.publish_snapshot()
        heartbeat_task = asyncio.create_task(self.heartbeat_loop())
        self.monitor_loop_task = asyncio.create_task(self.monitor_loop())
        try:
            while not self.should_exit:
                raw_line = await asyncio.to_thread(sys.stdin.readline)
                if raw_line == "":
                    break
                line = raw_line.strip()
                if not line:
                    continue
                try:
                    import json

                    payload = json.loads(line)
                    response = await self.dispatch(payload)
                except Exception as exc:  # noqa: BLE001
                    message_id = (
                        str(payload.get("id") or "") if "payload" in locals() else ""
                    )
                    self.log(f"worker error: {exc}")
                    if message_id:
                        response = error(message_id, str(exc))
                    else:
                        continue
                await self.writer.write(response)
        finally:
            await self._shutdown_recordings()
            heartbeat_task.cancel()
            if self.monitor_loop_task is not None:
                self.monitor_loop_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await heartbeat_task
            if self.monitor_loop_task is not None:
                with contextlib.suppress(asyncio.CancelledError):
                    await self.monitor_loop_task

    async def _shutdown_recordings(self) -> None:
        for job in list(self.jobs):
            await self._stop_recording_if_needed(job)
