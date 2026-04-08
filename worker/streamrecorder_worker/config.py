from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .models import DEFAULT_SETTINGS


class WorkerConfigStore:
    def __init__(self, data_root: Path) -> None:
        self.data_root = data_root
        self.jobs_path = self.data_root / "jobs.json"
        self.settings_path = self.data_root / "core_settings.json"
        self.cookies_path = self.data_root / "cookies.json"
        self.accounts_path = self.data_root / "accounts.json"
        self.logs_path = self.data_root / "worker.log"
        self.data_root.mkdir(parents=True, exist_ok=True)
        self._ensure_defaults()

    def _ensure_defaults(self) -> None:
        if not self.settings_path.exists():
            self.save_settings(DEFAULT_SETTINGS)
        if not self.jobs_path.exists():
            self.save_jobs([])
        if not self.cookies_path.exists():
            self.save_cookies({})
        if not self.accounts_path.exists():
            self.save_accounts({})

    def load_settings(self) -> dict[str, Any]:
        payload = self._load_json(self.settings_path, DEFAULT_SETTINGS)
        merged = DEFAULT_SETTINGS.copy()
        merged.update(payload)
        return merged

    def save_settings(self, payload: dict[str, Any]) -> None:
        self._save_json(self.settings_path, payload)

    def load_jobs(self) -> list[dict[str, Any]]:
        payload = self._load_json(self.jobs_path, [])
        return payload if isinstance(payload, list) else []

    def save_jobs(self, payload: list[dict[str, Any]]) -> None:
        self._save_json(self.jobs_path, payload)

    def load_cookies(self) -> dict[str, str]:
        payload = self._load_json(self.cookies_path, {})
        return payload if isinstance(payload, dict) else {}

    def save_cookies(self, payload: dict[str, str]) -> None:
        self._save_json(self.cookies_path, payload)

    def load_accounts(self) -> dict[str, Any]:
        payload = self._load_json(self.accounts_path, {})
        return payload if isinstance(payload, dict) else {}

    def save_accounts(self, payload: dict[str, Any]) -> None:
        self._save_json(self.accounts_path, payload)

    @staticmethod
    def _load_json(path: Path, fallback: Any) -> Any:
        try:
            with path.open("r", encoding="utf-8") as handle:
                return json.load(handle)
        except FileNotFoundError:
            return fallback
        except json.JSONDecodeError:
            return fallback

    @staticmethod
    def _save_json(path: Path, payload: Any) -> None:
        with path.open("w", encoding="utf-8") as handle:
            json.dump(payload, handle, ensure_ascii=False, indent=2)
