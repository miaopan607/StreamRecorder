from __future__ import annotations

import json
from asyncio import Lock
from typing import Any


class JsonLineWriter:
    def __init__(self) -> None:
        self._lock = Lock()

    async def write(self, payload: dict[str, Any]) -> None:
        import sys

        line = json.dumps(payload, ensure_ascii=False)
        async with self._lock:
            sys.stdout.write(line + "\n")
            sys.stdout.flush()


def ok(message_id: str, body: dict[str, Any] | None = None) -> dict[str, Any]:
    return {"kind": "result", "id": message_id, "body": body or {}}


def error(message_id: str, message: str) -> dict[str, Any]:
    return {"kind": "error", "id": message_id, "error": {"message": message}}


def event(name: str, body: dict[str, Any] | None = None) -> dict[str, Any]:
    return {"kind": "event", "name": name, "body": body or {}}
