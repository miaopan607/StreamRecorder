# StreamRecorder

`StreamRecorder` 是一个面向 Windows 的直播录制工具。

当前方案：

- `WPF` 负责原生桌面界面
- `Python worker` 负责任务、配置、探测和录制逻辑
- 桌面端与核心服务通过 `stdio + JSON lines` 通信

## 当前状态

已完成的基础能力：

- 中文桌面界面
- 任务新增、编辑、删除、批量操作
- 任务列表 / 卡片布局切换与持久化
- 监控、开播检测、自动拉起 `ffmpeg` 录制
- 录制时长、速度、状态显示
- 设置、Cookies、账号管理
- 托盘、通知、日志、依赖检查

## 目录结构

```text
src/                     WPF 桌面端
worker/                  Python 核心服务
runtime/                 运行时数据
```

## 开发运行

要求：

- Windows
- .NET 8 SDK
- Python 3
- `ffmpeg`

启动桌面端：

```powershell
dotnet run --project .\src\StreamRecorder.Desktop\StreamRecorder.csproj
```

程序会自动启动：

```powershell
python -m streamrecorder_worker --stdio --data-root .\runtime
```

## 构建 Release

先发布桌面程序：

```powershell
dotnet publish .\src\StreamRecorder.Desktop\StreamRecorder.csproj -c Release -r win-x64 --self-contained true -p:PublishSingleFile=false -o .\artifacts\publish\win-x64
```

然后把以下内容放到发布目录同级：

- `worker\`
- `runtime\`（可为空目录）

最终建议目录：

```text
artifacts/publish/win-x64/
  StreamRecorder.exe
  worker/
    streamrecorder_worker/
  runtime/
```

注意：当前版本默认通过 `python` 启动核心服务，因此目标机器仍需要可用的 Python 环境。

## 许可证

本项目使用 `Apache-2.0`。
