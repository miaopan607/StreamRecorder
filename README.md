# StreamRecorder

Windows 直播录制工具，使用 **Tauri 2 + React + TypeScript** 界面和 **Rust 原生核心**。界面直接调用具体 Tauri 命令，启动引导返回完整快照，状态由带 revision 的事件更新；不再启动 Python worker，也不使用 stdio 业务协议。

## 功能

- 中文任务工作台：列表 / 卡片、列设置、任务新增 / 编辑 / 删除、批量导入和监控。
- 自动开播检测和 ffmpeg 录制；显示录制状态、时长、速度，打开保存目录和最新文件。
- 录制、网络、命名、通知和主题设置自动保存；Cookies 与账号独立管理。
- 浅色 / 深色 / 跟随系统；托盘、系统通知、单实例和 Windows 登录后启动到托盘。
- 核心服务诊断、FFmpeg 检测和最近 300 条日志。

## 运行要求

- Windows 10 或更高版本，x64。
- Microsoft Edge WebView2 Evergreen Runtime。
- PATH 中可用的 `ffmpeg`。

目标机器不需要 Python、StreamGet、Node.js 或 .NET Desktop Runtime。少量平台签名 JS 和咪咕 WASM 解释器内嵌在 EXE 中，不调用外部脚本运行时。缺少 ffmpeg 只影响录制：界面、配置、探测和核心诊断仍可用；缺少 WebView2 会显示原生启动错误。

## 目录与数据

```text
src/StreamRecorder.Tauri/
  src/                        React 界面、类型与应用状态
  src-tauri/src/core/          Rust 配置、平台探测、调度、录制与通知
  src-tauri/src/               Windows 桌面、托盘和生命周期
runtime/                      开发运行数据
artifacts/publish/win-x64/     正式发布目录
```

沿用 `runtime/*.json`，保留账号嵌套 JSON、未知设置键、历史任务别名和列表布局。配置使用同目录临时文件、备份与原子替换；落盘失败不提交内存配置。账号和 Cookies 仍是本地明文 JSON，请勿公开整个 runtime 目录。

相对保存路径按应用根目录解析，不按启动命令的工作目录解析。任务配置目录不会被自动生成的录制子目录覆盖。

## 开发运行

需要 Node.js、npm、Rust MSVC 工具链、Microsoft C++ Build Tools（桌面 C++ 与 Windows SDK）、WebView2 和 ffmpeg。Node.js 仅用于前端开发与构建。

```powershell
cd .\src\StreamRecorder.Tauri
npm.cmd ci
npm.cmd run tauri -- dev
```

开发态默认使用仓库根目录。桌面验收可设置 `STREAMRECORDER_DEV_ROOT` 指向已存在的独立目录；不需要准备 worker。发布版不读取这个环境变量。

## 发布便携版

从仓库根目录执行：

```powershell
.\publish-release.ps1
```

输出：

```text
artifacts/publish/win-x64/StreamRecorder.exe
```

只需分发这个 EXE，不需要旁置前端资源、worker、JS 或 DLL。首次运行仅在 EXE 同级建立 `runtime/`，保存配置、图标、录制文件和 WebView2 缓存。已有 worker/runtime、未知文件和录制文件不会被扫描删除。

默认录制目录为 `runtime/recordings`。便携目录必须可写；无法写入时提示移动目录，不会改用 AppData。WebView2 和 ffmpeg 仍由目标机器提供。自启动使用当前 EXE 的绝对路径，移动便携目录后重新启动可更新已启用的启动项。

## 核心运行约束

- 探测使用 FIFO 队列，全局最多 16 个请求，各平台按设置限流；降限不会绕过已运行计数。
- 重检立即入队，不等待网络响应。停止、编辑和删除通过 generation 与取消令牌隔离旧结果。
- ffmpeg、转封装和自定义脚本受 Windows Job Object 管理；stderr 持续排空，诊断尾部最多 64 KiB。
- 批量停止并行执行：8 秒优雅停止，再用最多 2 秒强制回收，不按任务数累加。
- 通知独立于录制生命周期，每渠道限时 10 秒；HTTP 成功仍须检查业务确认，SMTP 使用异步 SSL 465。
- 有限 HLS 在 EOF 自然结束，不再设置 `reconnect_at_eof`。MP4 保留关键帧分片，避免提前写空 moov 导致 AAC 码流转换失败。

## 验证

```powershell
cd .\src\StreamRecorder.Tauri
npm.cmd run test
npm.cmd run build
cargo test --manifest-path .\src-tauri\Cargo.toml --bin StreamRecorder
```

默认 Rust 套件不访问公网、不使用用户 runtime，也不要求 ffmpeg。平台测试通过真实 loopback HTTP 和生产解析路径，覆盖 51 个平台键的开播 / 未开播、支持的画质、畸形响应、登录失败与 Cookie 更新；签名使用独立上游向量校对。核心回归覆盖配置失败保持、探测取消、并发降限、stderr 洪泛、进程树回收和重复退出。进程树测试的根进程与后代均以无控制台方式启动，仍验证自然退出与强制停止后的真实回收。

从仓库根目录执行真实媒体验收：

```powershell
$ffmpegBin = Join-Path $PWD 'artifacts\tauri-smoke-tools\ffmpeg\ffmpeg-9.0.2-essentials_build\bin'
$env:PATH = "$ffmpegBin;$env:PATH"
cargo test --manifest-path .\src\StreamRecorder.Tauri\src-tauri\Cargo.toml --bin StreamRecorder core::recording::tests -- --ignored --nocapture --test-threads=1
```

CoreService 的真实录制烟测已通过有限 HTTP HLS、持续 HTTP FLV、TS/FLV/MKV/MOV/MP4/MP3/M4A、12 个 TS 分段及转 MP4，输出由 ffprobe 检查。同次有限 HLS 录制只请求一次首分片；慢探测和失效 webhook 下仍可保存、ping 和停止。

HTTP 录制重试传输中断、TCP/TLS 连接错误及 HTTP 408、429、5xx；401/403 等非临时错误不重试。不启用 `reconnect_at_eof`，完整 HLS 分片和有限媒体读到 EOF 后正常结束。真实重连烟测覆盖首次 503 后恢复、FLV 响应中途截断后续传、有限 FLV 自然结束、403 不重试，以及连接拒绝后服务恢复；恢复后的媒体时长由 ffprobe 检查。无长度信息的 HTTP 流正常关闭连接时，单凭 EOF 无法区分下播与断线，仍按 EOF 结束，不进行盲目重连。

正式 EXE 已在中文空格便携目录中通过原生 Windows UIA 验收，应用 PATH 仅包含 ffmpeg、System32 和 Windows。实际操作覆盖添加 / 删除任务、中文设置连续自动保存、Cookie 字符串和嵌套账号、重检与 ping、重启恢复、关闭到托盘继续录制、单实例恢复窗口及托盘退出后无 FFmpeg 残留。缺 ffmpeg 时仍能保存和 ping；运行中提供 ffmpeg 后刷新依赖、重检并完成真实录制。

公开平台实际探测：虎牙 `11342412` 返回周星星及直播流，斗鱼 `288016` 返回英雄联盟赛事及直播流，抖音 `745964462470` 返回喜剧电影笑不停并明确未开播。该验收不等于这些平台全部账号、私密房间或所有其他平台已获授权验证。

## 平台边界

保留固定 `streamget==4.0.10` 的 51 个平台键 / 50 个解析器，不使用 Python 回退。平台停运、接口改版、风控或缺少授权会返回具体错误，不能视为未开播或录制成功。离线夹具通过不代表全部平台的真实授权录制通过。
旧任务链接按旧解析器处理空查询参数与备用 ID（例如淘宝 `id` 回退到 `liveId`、京东空 `authorId` 回退到直播间片段）。小红书先检查 `/livestream/...` 直播页；页面明确当前房间已结束时返回未开播及当前主播信息，不使用推荐直播数据。只有个人主页回退才要求主播 ID，缺少可判定的直播数据时仍报告解析错误。

离线元数据沿用旧解析器的空值契约：网易 CC 缺昵称、Bigo/ShowRoom 昵称为 `null` 时保留任务原名；六间房显式 `flvtitle=null` 表示没有直播流；VVXQ 的空昵称继续走备用查询。缺少必要状态、响应结构损坏及网络错误仍返回错误，不伪装为未开播。

咪咕支持识别旧 / 新 WASM ABI，执行受到 fuel、内存、宿主 I/O 和取消边界限制。真实新 SDK 夹具已验证资源限制，普通无 puData 直链已覆盖；授权分支尚缺可校对的有效输入，不能宣称其真实签名录制通过。

## 许可证

项目采用 Apache-2.0。迁移的 StreamGet 算法与必要签名资源保留上游 MIT 许可，文本位于 `src/StreamRecorder.Tauri/src-tauri/src/core/assets/STREAMGET-LICENSE`，同时内嵌进 EXE。
