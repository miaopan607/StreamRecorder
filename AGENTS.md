# AGENTS.md

## 项目定位
- `StreamRecorder` 是 Windows 直播录制工具。
- 架构为 `Tauri 2 + React + TypeScript` 界面和 Rust 原生核心。
- 前端通过具体 Tauri commands 调用业务，使用 `core-event`、`core-state`、`core-log` 订阅状态；不再运行 Python worker 或 stdio 业务协议。

## 目录速览
- `src/StreamRecorder.Tauri/src/`：React 界面、草稿自动保存和原生调用契约。
- `src/StreamRecorder.Tauri/src-tauri/src/core/`：配置、平台探测、签名、并发调度、录制、通知与进程回收。
- `src/StreamRecorder.Tauri/src-tauri/src/`：Windows 桌面壳、托盘、自启动、路径与生命周期。
- `runtime/`：运行时 JSON、默认录制目录和 WebView2 缓存；最近 300 条日志保存在核心内存。
- `artifacts/publish/win-x64/`：正式单文件发布输出。

## 文件速览
- `src/StreamRecorder.Tauri/src/App.tsx`：主导航、主题和退出确认。
- `src/StreamRecorder.Tauri/src/lib/store.ts`：snapshot/core revision、任务选择、500ms 自动保存和草稿保护。
- `src/StreamRecorder.Tauri/src/lib/desktop.ts`：具体原生命令调用，不使用 method/body 二级协议。
- `src/StreamRecorder.Tauri/src-tauri/src/lib.rs`：桌面启动、bootstrap、生命周期和 Tauri 命令入口。
- `src/StreamRecorder.Tauri/src-tauri/src/core/service.rs`：权威状态、配置事务、FIFO 探测调度、generation/取消和录制生命周期。
- `src/StreamRecorder.Tauri/src-tauri/src/core/config.rs`：JSON 恢复、损坏隔离、备份及 Windows 原子替换。
- `src/StreamRecorder.Tauri/src-tauri/src/core/probe.rs` / `platforms/`：真实 HTTP 和 51 个平台键 / 50 个解析器。
- `src/StreamRecorder.Tauri/src-tauri/src/core/signing.rs`：受限 QuickJS、原生密码算法及内嵌上游许可。
- `src/StreamRecorder.Tauri/src-tauri/src/core/recording.rs`：录制目录、FFmpeg 参数、输出统计与后处理。
- `src/StreamRecorder.Tauri/src-tauri/src/core/process.rs`：Job Object、持续 stderr 排空、退出观察与有界停止。
- `src/StreamRecorder.Tauri/src-tauri/src/core/notifications.rs`：七个实际通知渠道及异步 SMTP SSL。
- `src/StreamRecorder.Tauri/src-tauri/src/paths.rs`：开发/便携根目录和相对路径解析。

## 当前运行方式
1. 正式版先注册单实例，按 EXE 所在目录准备 runtime。
2. Rust 核心加载旧 JSON，完成启动状态，再独立安排平台探测。
3. 探测使用 FIFO 队列与既有活跃计数；慢请求不阻塞配置、ping 或其他平台。
4. FFmpeg、转封装和自定义脚本受 Job Object 管理；关闭到托盘不停止录制。
5. 退出取消探测、通知和后处理，并行回收录制进程。

## 改动时必须记住
- 设置自动保存，不增加“保存设置”“刷新快照”按钮。
- UI 保持 Windows 原生感，文案简洁，不随迁移改变布局。
- 便携发布只分发 `StreamRecorder.exe`；首次运行仅生成 runtime，不执行或清理已有 worker。
- 相对录制路径按应用根目录解析；不要把生成子目录写回任务配置目录。
- 配置落盘成功后才提交内存、递增 revision 和发布快照；失败保留前端草稿。
- 快照和核心状态拒绝旧 revision；SettingsDraft 的本地编辑 revision 是独立机制。
- 不跨 await 持有全局状态锁；网络、文件 I/O、进程等待和通知独立执行。
- 保留每任务 generation、取消令牌、生命周期锁及 recording_id，旧结果不能重建已停止或已删除的任务。
- 空 ids：start/stop/recheck 表示全部任务，delete 必须不删除任何任务。
- HTTP 保持 TLS 校验、明确代理、响应大小限制与取消边界；真实错误不得伪装成未开播。
- 内嵌签名 JS 不暴露文件/进程/网络；WASM 受 fuel、内存及宿主 I/O 限制。不要回退 Python/Node 或制造假签名。
- JSON 文件和日志保持 UTF-8 无 BOM。未知设置、嵌套账号和历史任务输入别名继续保留。

## 构建与验证
- 开发：在 `src/StreamRecorder.Tauri/` 执行 `npm.cmd ci`、`npm.cmd run tauri -- dev`。
- 前端：`npm.cmd run test`、`npm.cmd run build`。
- Rust：`cargo test --manifest-path src-tauri/Cargo.toml --bin StreamRecorder`；lib test 已关闭，不能只跑 `--lib`。
- 媒体：`core::recording::tests::ffmpeg_smoke` 为 ignored 的真实 CoreService + FFmpeg/ffprobe 验收，命令见 README。
- 发布：仓库根目录 `./publish-release.ps1`。
- 目标机器只需 WebView2 和外部 ffmpeg，不再需要 Python、StreamGet、Node 或 .NET Desktop Runtime。Node.js 和测试工具 Python 仅用于开发/外部自动化，不是应用后端。
