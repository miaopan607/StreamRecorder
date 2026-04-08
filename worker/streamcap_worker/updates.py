from __future__ import annotations

from dataclasses import dataclass
from typing import Any

import httpx


@dataclass(slots=True)
class UpdateChecker:
    current_version: str
    github_repo: str = "ihmily/StreamCap"

    async def check_for_updates(self) -> dict[str, Any]:
        url = f"https://api.github.com/repos/{self.github_repo}/releases/latest"
        try:
            async with httpx.AsyncClient(timeout=10) as client:
                response = await client.get(url)
                response.raise_for_status()
                latest_release = response.json()
        except Exception as exc:  # noqa: BLE001
            return {"has_update": False, "error": str(exc), "source": "GitHub"}

        latest_version = str(latest_release.get("tag_name", "")).lstrip("v")
        if not latest_version:
            return {
                "has_update": False,
                "error": "无法解析最新版本",
                "source": "GitHub",
            }

        download_urls: dict[str, str] = {}
        for asset in latest_release.get("assets", []):
            name = str(asset.get("name", "")).lower()
            if ("win" in name or "windows" in name) and "noff" not in name:
                download_urls["windows"] = asset.get(
                    "browser_download_url", latest_release.get("html_url", "")
                )
            elif ("mac" in name or "macos" in name) and "noff" not in name:
                download_urls["macos"] = asset.get(
                    "browser_download_url", latest_release.get("html_url", "")
                )
            elif "linux" in name:
                download_urls["linux"] = asset.get(
                    "browser_download_url", latest_release.get("html_url", "")
                )

        has_update = self._compare_versions(latest_version, self.current_version) > 0
        return {
            "has_update": has_update,
            "latest_version": latest_version,
            "current_version": self.current_version,
            "release_notes": latest_release.get("body", ""),
            "download_url": latest_release.get(
                "html_url", "https://github.com/ihmily/StreamCap/releases/latest"
            ),
            "download_urls": download_urls,
            "source": "GitHub",
        }

    @staticmethod
    def _compare_versions(version1: str, version2: str) -> int:
        def parse_version(version: str) -> tuple[list[int], int]:
            if "-" in version:
                v_parts, pre_release = version.split("-", 1)
                pre_release_value = {"alpha": -3, "beta": -2, "rc": -1}.get(
                    pre_release, 0
                )
            else:
                v_parts = version
                pre_release_value = 0

            numbers: list[int] = []
            for part in v_parts.split("."):
                digits = ""
                for char in part:
                    if char.isdigit():
                        digits += char
                    else:
                        break
                numbers.append(int(digits or 0))
            return numbers, pre_release_value

        v1_parts, v1_pre = parse_version(version1)
        v2_parts, v2_pre = parse_version(version2)
        for index in range(max(len(v1_parts), len(v2_parts))):
            v1 = v1_parts[index] if index < len(v1_parts) else 0
            v2 = v2_parts[index] if index < len(v2_parts) else 0
            if v1 > v2:
                return 1
            if v1 < v2:
                return -1
        if v1_pre > v2_pre:
            return 1
        if v1_pre < v2_pre:
            return -1
        return 0
