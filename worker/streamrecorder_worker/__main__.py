from __future__ import annotations

import argparse
import asyncio
from pathlib import Path
import sys

from .app import StreamRecorderWorkerApp


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the StreamRecorder headless worker."
    )
    parser.add_argument(
        "--stdio", action="store_true", help="Use JSON lines over stdio."
    )
    parser.add_argument(
        "--data-root", type=Path, default=Path("runtime"), help="Worker data directory."
    )
    return parser.parse_args()


def configure_stdio() -> None:
    for stream in (sys.stdin, sys.stdout):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8")
    if hasattr(sys.stderr, "reconfigure"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")


async def main() -> None:
    configure_stdio()
    args = parse_args()
    if not args.stdio:
        raise SystemExit("Only --stdio mode is currently implemented.")

    app = StreamRecorderWorkerApp(args.data_root)
    await app.serve_stdio()


if __name__ == "__main__":
    asyncio.run(main())
