# StreamCapRe2

`StreamCapRe2` is a fresh Windows-first rewrite of the old StreamCap desktop app.

The new direction is:

- `WPF` for the native Windows shell
- a separate `Python` worker for persistent job state and recording logic
- a clean IPC boundary so the recording engine is no longer coupled to the UI toolkit

## Current status

This first bootstrap commit delivers the foundation:

- git repo initialized
- classic `.sln` solution created for better Visual Studio compatibility
- WPF desktop shell with `Jobs`, `Settings`, and `Diagnostics` pages
- Python worker with JSON-lines IPC over stdio
- persisted `jobs.json` and `core_settings.json`
- job add/edit/delete and monitor toggle flow wired end to end
- platform recognition scaffold migrated into the worker

The real stream detection and recording engine is the next layer to plug into the worker contract.

## Project layout

```text
src/StreamCap.Desktop/
  WPF shell, screens, and worker client

worker/streamcap_worker/
  headless worker, job model, settings store, protocol
```

## Run the desktop shell

```powershell
dotnet build .\StreamCapRe2.sln
dotnet run --project .\src\StreamCap.Desktop\StreamCap.Desktop.csproj
```

The desktop app starts the Python worker automatically with:

- `python -m streamcap_worker --stdio`

using `worker/` as `PYTHONPATH` and `runtime/` as the data directory.

## Worker protocol

The current protocol uses newline-delimited JSON envelopes:

- `cmd`: request from WPF to the worker
- `result`: successful response
- `error`: failed response
- `event`: unsolicited worker event such as `core_health` or `snapshot_changed`

Implemented methods:

- `initialize`
- `get_snapshot`
- `health.ping`
- `settings.get`
- `settings.update`
- `jobs.upsert`
- `jobs.delete`
- `jobs.start_monitoring`
- `jobs.stop_monitoring`
- `core.shutdown`

## Next steps

Planned next work items:

1. Move the actual recording scheduler into the worker.
2. Port the reusable `streamget + ffmpeg` pipeline behind the new worker contract.
3. Add config import from `StreamCap-1.0.2`.
4. Add tray, notifications, and engine diagnostics on top of the stable core.
*** Delete File: D:\File\CODE\Git\StreamCapRe2\StreamCapRe2.slnx
