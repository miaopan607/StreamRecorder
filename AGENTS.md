# AGENTS.md

## 项目定位
- `StreamRecorder` 是 Windows 直播录制工具。
- 架构是 `WPF` 桌面壳 + `Python worker` 核心服务。
- 桌面端和 worker 通过 `stdio + JSON lines` 通信，不是 named pipe / HTTP。

## 目录速览
- `src/StreamRecorder.Desktop/`: WPF 界面、ViewModel、托盘、自启动、路径与发布逻辑。
- `worker/streamrecorder_worker/`: 核心录制逻辑，包含探测、录制、通知、配置存储。
- `runtime/`: 运行时 JSON 数据与日志，程序会自动创建。
- `artifacts/publish/win-x64/`: 正式发布输出目录。

## 文件速览
- `src/StreamRecorder.Desktop/ViewModels/MainViewModel.cs`: 桌面端主状态与 UI 行为入口。
- `src/StreamRecorder.Desktop/Services/WorkerClient.cs`: 启动 worker、stdio 通信、日志接收。
- `src/StreamRecorder.Desktop/Services/ProjectPaths.cs`: 应用根目录、`worker/`、`runtime/` 路径规则。
- `src/StreamRecorder.Desktop/Services/EmbeddedWorkerService.cs`: 将内嵌 worker 文件解压到 exe 同级目录。
- `worker/streamrecorder_worker/__main__.py`: worker 启动入口。
- `worker/streamrecorder_worker/app.py`: worker 主循环、命令分发、监控与录制流程。
- `worker/streamrecorder_worker/probe.py` / `ffmpeg.py`: 开播探测与 ffmpeg 录制。

## 当前真实运行方式
1. 桌面端启动时先把内嵌的 worker `.py` 解压到 exe 同级 `worker/streamrecorder_worker/`。
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
- 改界面：`src/StreamRecorder.Desktop/MainWindow.xaml`、`src/StreamRecorder.Desktop/Views/JobEditorWindow.xaml`
- 改桌面交互：`src/StreamRecorder.Desktop/MainWindow.xaml.cs`、`src/StreamRecorder.Desktop/ViewModels/MainViewModel.cs`
- 改托盘/自启动/路径：`src/StreamRecorder.Desktop/Services/TrayService.cs`、`AutoStartService.cs`、`ProjectPaths.cs`
- 改配置持久化：`worker/streamrecorder_worker/config.py`
- 改通知：`worker/streamrecorder_worker/notification_service.py`

## 构建发布
- 开发运行：`dotnet run --project .\src\StreamRecorder.Desktop\StreamRecorder.csproj`
- 正式发布：运行仓库根目录 `./publish-release.ps1`
- 当前正式发布是 framework-dependent single-file；目标机器仍需要 `.NET 8 Desktop Runtime`、`Python 3`、`ffmpeg`
