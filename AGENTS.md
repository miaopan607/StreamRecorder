# AGENTS.md

## 项目定位
- `StreamRecorder` 是 Windows 直播录制工具。
- 架构是 `Tauri 2 + React + TypeScript` 桌面壳 + `Python worker` 核心服务。
- 桌面端和 worker 通过 `stdio + JSON lines` 通信，不是 named pipe / HTTP。

## 目录速览
- `src/StreamRecorder.Tauri/`: React 界面与 Rust 桌面壳，包含托盘、自启动、路径和发布逻辑。
- `worker/streamrecorder_worker/`: 核心录制逻辑，包含探测、录制、通知、配置存储。
- `runtime/`: 运行时 JSON 数据、默认录制目录与 WebView2 缓存；最近 300 条日志由桌面端内存保存。
- `artifacts/publish/win-x64/`: 正式发布输出目录。

## 文件速览
- `src/StreamRecorder.Tauri/src/App.tsx`: 主导航、主题与退出确认。
- `src/StreamRecorder.Tauri/src/lib/store.ts`: 快照、选择与 500ms 自动保存状态。
- `src/StreamRecorder.Tauri/src-tauri/src/lib.rs`: 桌面启动、生命周期与 Tauri 命令入口。
- `src/StreamRecorder.Tauri/src-tauri/src/worker.rs`: Python 子进程、UTF-8 stdio、请求等待与进程树管理。
- `src/StreamRecorder.Tauri/src-tauri/src/paths.rs` / `src/StreamRecorder.Tauri/src-tauri/build.rs`: 应用路径与 worker 内嵌资源。
- `worker/streamrecorder_worker/__main__.py`: worker 启动入口。
- `worker/streamrecorder_worker/app.py`: worker 主循环、命令分发、监控与录制流程。
- `worker/streamrecorder_worker/probe.py` / `ffmpeg.py`: 开播探测与 ffmpeg 录制。

## 当前真实运行方式
1. 正式版先注册单实例，再提取内嵌 worker `.py` 到 exe 同级 `worker/streamrecorder_worker/`；开发版直接复用仓库源目录。
2. 然后执行 `python -m streamrecorder_worker --stdio --data-root <app>/runtime`。
3. worker 读取和写入 `runtime/*.json`。
4. worker 负责探测开播、启动 `ffmpeg`、更新状态、发通知。

## 改动时必须记住
- 设置是自动保存，不要再加“保存设置”“刷新快照”这类手动按钮。
- UI 方向是 Windows 原生感，文案简洁直接，少“开发者味”。
- 发布目标是正式单文件：发布目录最终只保留 `StreamRecorder.exe`；首次运行再生成 `worker/` 和 `runtime/`。
- 相对保存路径按应用根目录解析，不按 worker 当前目录解析。
- 如果改 stdio / 日志编码，保持 UTF-8 无 BOM 链路，否则中文、emoji 和 JSON 解析会出问题。

## 常见落点
- 改界面：`src/StreamRecorder.Tauri/src/pages/`、`src/StreamRecorder.Tauri/src/components/`、`src/StreamRecorder.Tauri/src/styles.css`
- 改桌面交互与状态：`src/StreamRecorder.Tauri/src/App.tsx`、`src/StreamRecorder.Tauri/src/lib/store.ts`
- 改托盘/自启动/文件操作：`src/StreamRecorder.Tauri/src-tauri/src/desktop.rs`、`src/StreamRecorder.Tauri/src-tauri/src/lib.rs`
- 改配置持久化：`worker/streamrecorder_worker/config.py`
- 改通知：`worker/streamrecorder_worker/notification_service.py`

## 构建发布
- 开发运行：在 `src/StreamRecorder.Tauri/` 执行 `npm.cmd ci`、`npm.cmd run tauri -- dev`。
- 正式发布：运行仓库根目录 `./publish-release.ps1`
- 正式发布是 Windows x64 原生便携单 EXE；目标机器需要 WebView2、Python 3.10+（安装 `streamget==4.0.10`）、ffmpeg，部分平台还需要 Node.js；不再需要 .NET Desktop Runtime。
