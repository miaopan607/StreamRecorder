# StreamRecorder

Windows 直播录制工具，桌面端使用 **Tauri 2 + React + TypeScript**，Python worker 负责开播探测、录制和推送。两者通过 **stdio + UTF-8 JSON lines** 通信，不需要 HTTP 服务或 .NET Desktop Runtime。

## 功能

- 中文任务工作台：列表 / 卡片、列显示设置、任务新增 / 编辑 / 删除、批量导入和批量监控。
- 自动开播检测和 ffmpeg 录制；显示实际录制状态、时长、速度，打开保存目录和最新文件。
- 录制、网络、命名、推送和主题设置自动保存；Cookies 与账号单独保存、重新加载。
- 浅色 / 深色 / 跟随系统；托盘、系统通知、单实例和登录 Windows 后启动到托盘。
- 真实依赖诊断、核心服务测试和最近 300 条日志。

## 运行要求

- Windows 10 或更高版本，x64。
- Microsoft Edge WebView2 Evergreen Runtime。
- PATH 中可用的 Python 3.10+，并安装 worker 依赖：
  ```powershell
  python -m pip install streamget==4.0.10
  ```
- PATH 中可用的 `ffmpeg`。部分直播平台还需要 Node.js；诊断页会显示其状态。

程序不会自动安装这些依赖。缺少 Python 或 streamget 时，桌面界面仍可打开，具体错误显示在诊断页；缺少 WebView2 时会显示原生启动错误。

## 目录

```text
src/StreamRecorder.Tauri/
  src/                     React 界面、类型与应用状态
  src-tauri/               Rust 桌面壳、worker 通信、Windows 服务
worker/streamrecorder_worker/ Python 录制核心
runtime/                   开发运行数据
artifacts/publish/win-x64/  正式发布目录
```

任务、设置、Cookies、账号和列表布局继续读取原有 `runtime/*.json`。账号中的嵌套 JSON、未知配置键和已有布局设置会保留。配置和凭据仍存储在本地 JSON 文件中，请勿公开整个 runtime 目录。

## 开发运行

需要 Node.js、npm、Rust MSVC 工具链、Microsoft C++ Build Tools（桌面 C++ 与 Windows SDK）、WebView2 以及上面的 worker 依赖。

```powershell
cd .\src\StreamRecorder.Tauri
npm.cmd ci
npm.cmd run tauri -- dev
```

开发态按仓库根目录定位 worker 和 runtime，不按启动命令的工作目录定位。原生桌面验收可在开发态设置 `STREAMRECORDER_DEV_ROOT` 指向独立目录；该目录需要事先准备 worker/，发布版不读取这个环境变量。

## 发布便携版

从仓库根目录执行：

```powershell
.\publish-release.ps1
```

发布入口构建 Windows x64 原生 EXE，并复制到：

```text
artifacts/publish/win-x64/StreamRecorder.exe
```

只需分发 `StreamRecorder.exe`，不需要旁置前端资源、DLL 或 worker 文件。首次运行会在 EXE 同级生成：

```text
worker/streamrecorder_worker/  内嵌 Python 文件
runtime/                     JSON 配置、录制文件及 WebView2 缓存
```

已有 worker/runtime 和录制文件不会因重新发布而删除。相对保存路径始终按 **EXE 所在目录** 解析；默认录制目录是 `runtime/recordings`。便携目录必须可写，不可写时程序会提示移动目录，不会悄悄切换到 AppData。

单 EXE 并不包含 WebView2、Python、streamget 或 ffmpeg，这些运行依赖仍需由目标机器提供。开机启动使用当前 EXE 的绝对路径，移动便携目录后重新启动程序可更新已启用的启动项。

## 验证

```powershell
cd .\src\StreamRecorder.Tauri
npm.cmd run build
npm.cmd run test
cd .\src-tauri
cargo test
```

worker 原有测试可从仓库根目录执行：

```powershell
$env:PYTHONPATH = "$PWD\worker"
python -B -m unittest discover -s .\worker\streamrecorder_worker\tests
```

本次迁移已通过真实 Tauri 窗口操作、配置持久化、旧数据兼容、托盘、单实例、自启动与持续 HTTP FLV 录制验收；录制文件由 ffprobe 检查音视频流。没有使用模拟 worker 或浏览器页面代替桌面程序。

### 现有核心限制

Python worker 的录制算法和 FFmpeg 参数在此次前端迁移中保持不变。验证中发现，现有 `-reconnect_at_eof 1` 会使本地有限 HTTP HLS 清单被反复读取，无法完成录制输入探测；该场景未通过验收。持续 FLV 流的实际录制链路已通过。这一问题属于现有录制核心，不在此次前端迁移中改动。

## 许可证

Apache-2.0。
