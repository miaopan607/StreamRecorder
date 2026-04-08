from __future__ import annotations

import argparse
import asyncio
from pathlib import Path

from .app import StreamCapWorkerApp


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the StreamCapRe2 headless worker."
    )
    parser.add_argument(
        "--stdio", action="store_true", help="Use JSON lines over stdio."
    )
    parser.add_argument(
        "--data-root", type=Path, default=Path("runtime"), help="Worker data directory."
    )
    return parser.parse_args()


async def main() -> None:
    args = parse_args()
    if not args.stdio:
        raise SystemExit("Only --stdio mode is currently implemented.")

    app = StreamCapWorkerApp(args.data_root)
    await app.serve_stdio()


if __name__ == "__main__":
    asyncio.run(main())
