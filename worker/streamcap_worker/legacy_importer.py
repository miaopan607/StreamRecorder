from __future__ import annotations

import json
from pathlib import Path
from typing import Any


class LegacyImporter:
    def __init__(self, legacy_root: Path) -> None:
        self.legacy_root = legacy_root
        self.config_root = legacy_root / "config"

    def import_all(self) -> dict[str, Any]:
        if not self.config_root.exists():
            raise FileNotFoundError(f"未找到旧版配置目录: {self.config_root}")

        return {
            "settings": self._load_json("user_settings.json", {}),
            "cookies": self._load_json("cookies.json", {}),
            "accounts": self._load_json("accounts.json", {}),
            "jobs": self._load_json("recordings.json", []),
            "version": self._load_json("version.json", {}),
        }

    def _load_json(self, name: str, fallback: Any) -> Any:
        path = self.config_root / name
        if not path.exists():
            return fallback
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)
