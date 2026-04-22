from __future__ import annotations

import json
import os
import shutil
from datetime import datetime
from pathlib import Path
from typing import Any, Callable

from .models import DEFAULT_SETTINGS


class WorkerConfigStore:
    def __init__(
        self,
        data_root: Path,
        logger: Callable[[str], None] | None = None,
    ) -> None:
        self.data_root = data_root
        self.jobs_path = self.data_root / "jobs.json"
        self.settings_path = self.data_root / "core_settings.json"
        self.cookies_path = self.data_root / "cookies.json"
        self.accounts_path = self.data_root / "accounts.json"
        self.logs_path = self.data_root / "worker.log"
        self._logger = logger
        self.data_root.mkdir(parents=True, exist_ok=True)
        self._ensure_defaults()

    def _ensure_defaults(self) -> None:
        if not self._has_any_version(self.settings_path):
            self.save_settings(DEFAULT_SETTINGS)
        if not self._has_any_version(self.jobs_path):
            self.save_jobs([])
        if not self._has_any_version(self.cookies_path):
            self.save_cookies({})
        if not self._has_any_version(self.accounts_path):
            self.save_accounts({})

    def load_settings(self) -> dict[str, Any]:
        payload = self._load_json(self.settings_path, DEFAULT_SETTINGS, dict)
        merged = DEFAULT_SETTINGS.copy()
        merged.update(payload)
        return merged

    def save_settings(self, payload: dict[str, Any]) -> None:
        self._save_json(self.settings_path, payload)

    def load_jobs(self) -> list[dict[str, Any]]:
        payload = self._load_json(self.jobs_path, [], list)
        return payload if isinstance(payload, list) else []

    def save_jobs(self, payload: list[dict[str, Any]]) -> None:
        self._save_json(self.jobs_path, payload)

    def load_cookies(self) -> dict[str, str]:
        payload = self._load_json(self.cookies_path, {}, dict)
        return payload if isinstance(payload, dict) else {}

    def save_cookies(self, payload: dict[str, str]) -> None:
        self._save_json(self.cookies_path, payload)

    def load_accounts(self) -> dict[str, Any]:
        payload = self._load_json(self.accounts_path, {}, dict)
        return payload if isinstance(payload, dict) else {}

    def save_accounts(self, payload: dict[str, Any]) -> None:
        self._save_json(self.accounts_path, payload)

    def _load_json(self, path: Path, fallback: Any, expected_type: type[Any]) -> Any:
        payload, repaired = self._try_load_json(path, expected_type)
        if payload is not None:
            if repaired:
                self._save_json(path, payload, update_backup=False)
                self._log(f"{path.name} 已自动修复。")
            return payload

        for candidate_path, source_label in (
            (self._backup_path(path), "备份"),
            (self._temp_path(path), "临时文件"),
        ):
            payload, repaired = self._try_load_json(candidate_path, expected_type)
            if payload is None:
                continue

            self._save_json(path, payload, update_backup=False)
            repaired_suffix = "，并清理了尾部空字节" if repaired else ""
            self._log(f"{path.name} 已从{source_label}恢复{repaired_suffix}。")
            return payload

        if path.exists():
            corrupt_path = self._quarantine_corrupt_file(path)
            self._log(f"{path.name} 已损坏，已隔离为 {corrupt_path.name}。")

        default_payload = self._clone_fallback(fallback)
        self._save_json(path, default_payload, update_backup=False)
        self._log(f"{path.name} 已重建默认内容。")
        return default_payload

    @staticmethod
    def _try_load_json(path: Path, expected_type: type[Any]) -> tuple[Any | None, bool]:
        if not path.exists():
            return None, False

        try:
            raw = path.read_bytes()
        except OSError:
            return None, False

        if WorkerConfigStore._is_effectively_empty(raw):
            return None, False

        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError:
            return None, False

        for candidate_text, repaired in (
            (text, False),
            (WorkerConfigStore._trim_trailing_nuls(text), True),
        ):
            if not candidate_text:
                continue
            try:
                payload = json.loads(candidate_text)
            except json.JSONDecodeError:
                continue

            if isinstance(payload, expected_type):
                return payload, repaired

        return None, False

    @staticmethod
    def _save_json(path: Path, payload: Any, *, update_backup: bool = True) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        temp_path = WorkerConfigStore._temp_path(path)
        backup_path = WorkerConfigStore._backup_path(path)
        content = json.dumps(payload, ensure_ascii=False, indent=2).encode("utf-8")

        try:
            with temp_path.open("wb") as handle:
                handle.write(content)
                handle.write(b"\n")
                handle.flush()
                os.fsync(handle.fileno())

            if update_backup and path.exists():
                shutil.copyfile(path, backup_path)

            os.replace(temp_path, path)
        finally:
            temp_path.unlink(missing_ok=True)

    @staticmethod
    def _has_any_version(path: Path) -> bool:
        return any(
            candidate.exists()
            for candidate in (
                path,
                WorkerConfigStore._backup_path(path),
                WorkerConfigStore._temp_path(path),
            )
        )

    @staticmethod
    def _backup_path(path: Path) -> Path:
        return Path(f"{path}.bak")

    @staticmethod
    def _temp_path(path: Path) -> Path:
        return Path(f"{path}.tmp")

    @staticmethod
    def _trim_trailing_nuls(text: str) -> str:
        return text.rstrip("\x00").rstrip()

    @staticmethod
    def _is_effectively_empty(raw: bytes) -> bool:
        return not raw or not raw.replace(b"\x00", b"").strip()

    @staticmethod
    def _clone_fallback(fallback: Any) -> Any:
        if isinstance(fallback, dict):
            return dict(fallback)
        if isinstance(fallback, list):
            return list(fallback)
        return fallback

    def _quarantine_corrupt_file(self, path: Path) -> Path:
        timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        corrupt_path = Path(f"{path}.corrupt-{timestamp}")
        os.replace(path, corrupt_path)
        return corrupt_path

    def _log(self, message: str) -> None:
        if self._logger is not None:
            self._logger(message)
