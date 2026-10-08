use crate::paths::ProjectPaths;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tauri::{AppHandle, Emitter};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, Command},
    sync::{oneshot, Mutex as AsyncMutex, Notify},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::*,
};

pub const METHODS: &[&str] = &[
    "initialize",
    "get_snapshot",
    "health.ping",
    "settings.get",
    "settings.update",
    "cookies.get",
    "cookies.update",
    "accounts.get",
    "accounts.update",
    "dependencies.get",
    "jobs.upsert",
    "jobs.delete",
    "jobs.start_monitoring",
    "jobs.stop_monitoring",
    "jobs.recheck",
];
#[derive(Clone, Serialize)]
pub struct Connection {
    pub status: String,
    pub error: String,
}
#[derive(Clone)]
pub struct Cache {
    pub snapshot: Value,
    pub cookies: Value,
    pub accounts: Value,
    pub dependencies: Value,
    pub connection: Connection,
    pub logs: Vec<String>,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            snapshot: Value::Null,
            cookies: json!({}),
            accounts: json!({}),
            dependencies: json!({}),
            connection: Connection {
                status: "starting".into(),
                error: String::new(),
            },
            logs: vec![],
        }
    }
}
struct ProcessJob(usize);
impl ProcessJob {
    fn attach(process: HANDLE) -> Result<Self, String> {
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
                || AssignProcessToJobObject(handle, process) == 0
            {
                let error = std::io::Error::last_os_error().to_string();
                CloseHandle(handle);
                return Err(error);
            }
            Ok(Self(handle as usize))
        }
    }
}
impl Drop for ProcessJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0 as HANDLE);
        }
    }
}
type Pending = HashMap<String, oneshot::Sender<Result<Value, String>>>;
pub struct WorkerClient {
    pub cache: Mutex<Cache>,
    stdin: AsyncMutex<Option<ChildStdin>>,
    pending: Mutex<Pending>,
    job: Mutex<Option<ProcessJob>>,
    exited: Notify,
    alive: std::sync::atomic::AtomicBool,
    app: Option<AppHandle>,
}
impl WorkerClient {
    pub fn new(app: Option<AppHandle>) -> Arc<Self> {
        Arc::new(Self {
            cache: Mutex::new(Cache::default()),
            stdin: AsyncMutex::new(None),
            pending: Mutex::new(HashMap::new()),
            job: Mutex::new(None),
            exited: Notify::new(),
            alive: std::sync::atomic::AtomicBool::new(false),
            app,
        })
    }
    pub fn log(&self, text: &str) {
        let line = format!("[{}] {text}", chrono::Local::now().format("%H:%M:%S"));
        let mut cache = self.cache.lock();
        cache.logs.insert(0, line.clone());
        cache.logs.truncate(300);
        drop(cache);
        if let Some(app) = &self.app {
            let _ = app.emit("worker-log", line);
        }
    }
    pub fn connection(&self, status: &str, error: &str) {
        let state = Connection {
            status: status.into(),
            error: error.into(),
        };
        self.cache.lock().connection = state.clone();
        if let Some(app) = &self.app {
            let _ = app.emit("worker-state", state);
        }
    }
    fn disconnect(&self, error: &str) {
        self.connection("disconnected", error);
        self.log(error);
        for (_, sender) in self.pending.lock().drain() {
            let _ = sender.send(Err(error.into()));
        }
    }
    pub async fn start(self: &Arc<Self>, paths: &ProjectPaths) -> Result<(), String> {
        let mut cmd = Command::new("python");
        cmd.args(["-m", "streamrecorder_worker", "--stdio", "--data-root"])
            .arg(&paths.data_root)
            .current_dir(&paths.worker_root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .creation_flags(0x08000000);
        let mut pythonpath = vec![paths.worker_root.clone()];
        if let Some(existing) = std::env::var_os("PYTHONPATH") {
            pythonpath.extend(std::env::split_paths(&existing))
        }
        cmd.env(
            "PYTHONPATH",
            std::env::join_paths(pythonpath).map_err(|e| e.to_string())?,
        )
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8");
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("无法启动 Python 核心服务：{e}"))?;
        let job =
            ProcessJob::attach(child.raw_handle().ok_or("无法取得 Python 进程句柄")? as HANDLE)
                .map_err(|e| format!("无法管理录制进程树：{e}"))?;
        *self.job.lock() = Some(job);
        *self.stdin.lock().await = child.stdin.take();
        self.alive.store(true, std::sync::atomic::Ordering::Release);
        let stdout = child.stdout.take().ok_or("缺少核心服务输出流")?;
        let stderr = child.stderr.take().ok_or("缺少核心服务错误流")?;
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if !line.trim().is_empty() {
                            if let Err(error) = this.receive(&line) {
                                this.disconnect(&error);
                                this.job.lock().take();
                                break;
                            }
                        }
                    }
                    Ok(None) => {
                        this.disconnect("核心服务输出已关闭");
                        break;
                    }
                    Err(e) => {
                        this.disconnect(&format!("核心服务输出读取失败：{e}"));
                        break;
                    }
                }
            }
        });
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    this.log(&line)
                }
            }
        });
        self.watch_exit(child);
        let snapshot = self.call("get_snapshot", json!({})).await?;
        if self.cache.lock().snapshot.is_null() {
            self.cache.lock().snapshot = snapshot;
        }
        let cookies = self.call("cookies.get", json!({})).await?;
        self.cache.lock().cookies = cookies["cookies"].clone();
        let accounts = self.call("accounts.get", json!({})).await?;
        self.cache.lock().accounts = accounts["accounts"].clone();
        let dependencies = self.call("dependencies.get", json!({})).await?;
        if let Some(items) = dependencies.as_object() {
            let mut cache = self.cache.lock();
            for (key, value) in items {
                if let Some(values) = value.as_object() {
                    if !cache.dependencies[key].is_object() {
                        cache.dependencies[key] = json!({});
                    }
                    cache.dependencies[key]
                        .as_object_mut()
                        .unwrap()
                        .extend(values.clone());
                }
            }
        }
        self.connection("connected", "");
        Ok(())
    }
    fn receive(&self, line: &str) -> Result<(), String> {
        let payload: Value =
            serde_json::from_str(line).map_err(|e| format!("核心服务协议解析失败：{e}"))?;
        match payload["kind"].as_str() {
            Some("event") => {
                let name = payload["name"].as_str().ok_or("核心服务事件缺少名称")?;
                let body = payload.get("body").cloned().unwrap_or(json!({}));
                if name == "snapshot_changed" {
                    self.cache.lock().snapshot = body.clone()
                }
                if let Some(app) = &self.app {
                    let _ = app.emit("worker-event", json!({"name":name,"body":body}));
                    if name == "desktop_notification" {
                        crate::desktop::worker_notification(app, &body)
                    }
                }
            }
            Some("result") | Some("error") => {
                let id = payload["id"].as_str().ok_or("核心服务回复缺少 ID")?;
                let sender = self.pending.lock().remove(id);
                if let Some(sender) = sender {
                    let result = if payload["kind"] == "result" {
                        Ok(payload.get("body").cloned().unwrap_or(json!({})))
                    } else {
                        Err(payload["error"]["message"]
                            .as_str()
                            .unwrap_or("核心服务调用失败")
                            .into())
                    };
                    let _ = sender.send(result);
                }
            }
            _ => return Err("核心服务协议包含未知消息类型".into()),
        }
        Ok(())
    }
    pub async fn call(&self, method: &str, body: Value) -> Result<Value, String> {
        self.call_timeout(method, body, Duration::from_secs(8))
            .await
    }
    async fn call_timeout(
        &self,
        method: &str,
        body: Value,
        deadline: Duration,
    ) -> Result<Value, String> {
        if !self.alive.load(std::sync::atomic::Ordering::Acquire) {
            return Err("核心服务当前未运行".into());
        }
        let id = uuid::Uuid::new_v4().simple().to_string();
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().insert(id.clone(), sender);
        let bytes = serde_json::to_vec(&json!({"kind":"cmd","id":id,"method":method,"body":body}))
            .map_err(|e| e.to_string())?;
        let expires = tokio::time::Instant::now() + deadline;
        let write = async {
            let mut guard = self.stdin.lock().await;
            let stdin = guard.as_mut().ok_or("核心服务输入已关闭".to_string())?;
            stdin.write_all(&bytes).await.map_err(|e| e.to_string())?;
            stdin.write_all(b"\n").await.map_err(|e| e.to_string())?;
            stdin.flush().await.map_err(|e| e.to_string())
        };
        let sent = tokio::time::timeout_at(expires, write)
            .await
            .unwrap_or_else(|_| Err("核心服务请求写入超时".into()));
        if let Err(error) = sent {
            self.disconnect(&format!("核心服务输入失败：{error}"));
            return Err(error);
        }
        let result = tokio::time::timeout_at(expires, receiver)
            .await
            .map_err(|_| format!("核心服务请求超时：{method}"))
            .and_then(|reply| reply.map_err(|_| "核心服务请求已中断".to_string()))
            .and_then(|reply| reply);
        self.pending.lock().remove(&id);
        result
    }
    fn watch_exit(self: &Arc<Self>, mut child: tokio::process::Child) {
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let status = child.wait().await;
            this.alive
                .store(false, std::sync::atomic::Ordering::Release);
            // 启动器或核心异常退出时立即结束整棵录制进程树。
            this.job.lock().take();
            this.disconnect(&format!(
                "核心服务进程已退出：{}",
                status
                    .map(|s| s.to_string())
                    .unwrap_or_else(|e| e.to_string())
            ));
            this.exited.notify_waiters();
        });
    }

    pub async fn shutdown(&self) {
        if self.alive.load(std::sync::atomic::Ordering::Acquire) {
            let _ = self.call("core.shutdown", json!({})).await;
            let wait = self.exited.notified();
            if self.alive.load(std::sync::atomic::Ordering::Acquire) {
                let _ = tokio::time::timeout(Duration::from_secs(2), wait).await;
            }
        }
        self.job.lock().take();
        self.stdin.lock().await.take();
    }
}

pub async fn dependencies() -> Value {
    let mut result = serde_json::Map::new();
    for (name,command,args) in [("python","python",vec!["--version"]),("streamget","python",vec!["-c","import streamget; import importlib.metadata; print(importlib.metadata.version('streamget'))"]),("ffmpeg","ffmpeg",vec!["-version"]),("node","node",vec!["--version"])]{
        let path=resolve_command(command).map(|p|p.to_string_lossy().into_owned()).unwrap_or_default();
        let mut cmd=Command::new(command);cmd.args(&args).creation_flags(0x08000000).kill_on_drop(true);
        let output=tokio::time::timeout(Duration::from_secs(3),cmd.output()).await;
        let (available,version,error)=match output {Ok(Ok(out))=>{let text=format!("{}\n{}",String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));let available=out.status.success()&&!text.contains("Microsoft Store")&&!text.contains("Python was not found");(available,text.lines().next().unwrap_or("").trim().to_string(),if available{String::new()}else{text.trim().to_string()})},Ok(Err(e))=>(false,String::new(),e.to_string()),Err(_)=>(false,String::new(),"依赖检测超时".into())};
        result.insert(name.into(),json!({"available":available,"version":version,"path":path,"error":error}));
    }
    Value::Object(result)
}
fn resolve_command(name: &str) -> Option<std::path::PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .flat_map(|dir| [dir.join(format!("{name}.exe")), dir.join(name)])
        .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_protocol_fails() {
        let client = WorkerClient::new(None);
        assert!(client.receive("bad json").unwrap_err().contains("解析失败"));
    }
    #[test]
    fn eof_cancels_pending() {
        let client = WorkerClient::new(None);
        let (tx, mut rx) = oneshot::channel();
        client.pending.lock().insert("id".into(), tx);
        client.disconnect("EOF");
        assert_eq!(rx.try_recv().unwrap(), Err("EOF".into()));
        assert!(client.pending.lock().is_empty());
    }
    #[test]
    fn unresponsive_process_times_out_and_cleans_request() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let client = WorkerClient::new(None);
            let mut child = Command::new("powershell.exe")
                .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"])
                .creation_flags(0x08000000)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            *client.stdin.lock().await = child.stdin.take();
            client
                .alive
                .store(true, std::sync::atomic::Ordering::Release);
            let result = client
                .call_timeout("health.ping", json!({}), Duration::from_millis(50))
                .await;
            child.kill().await.unwrap();
            assert!(result.unwrap_err().contains("超时"));
            assert!(client.pending.lock().is_empty());
        });
    }
    #[test]
    fn core_exit_terminates_live_descendants() {
        use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
        use windows_sys::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
        use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let client = WorkerClient::new(None);
            let script = "$exe=[System.Diagnostics.Process]::GetCurrentProcess().MainModule.FileName; $child=Start-Process -FilePath $exe -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 30' -PassThru -NoNewWindow; Write-Output $child.Id; Start-Sleep -Seconds 30";
            let mut root = Command::new("powershell.exe")
                .args(["-NoProfile", "-Command", script])
                .creation_flags(0x08000000)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            *client.job.lock() =
                Some(ProcessJob::attach(root.raw_handle().unwrap() as HANDLE).unwrap());
            client.alive.store(true, std::sync::atomic::Ordering::Release);
            let mut output = BufReader::new(root.stdout.take().unwrap()).lines();
            let pid: u32 = tokio::time::timeout(Duration::from_secs(10), output.next_line())
                .await.unwrap().unwrap().unwrap().trim().parse().unwrap();
            // SYNCHRONIZE：仅等待本测试启动的子进程，不修改其他进程。
            let handle = unsafe { OpenProcess(0x00100000, 0, pid) };
            assert!(!handle.is_null());
            let child = unsafe { OwnedHandle::from_raw_handle(handle) };
            assert_eq!(unsafe { WaitForSingleObject(child.as_raw_handle() as HANDLE, 0) }, WAIT_TIMEOUT);
            let done = client.exited.notified();
            root.start_kill().unwrap();
            client.watch_exit(root);
            tokio::time::timeout(Duration::from_secs(10), done).await.unwrap();
            assert_eq!(unsafe { WaitForSingleObject(child.as_raw_handle() as HANDLE, 5000) }, WAIT_OBJECT_0);
            assert_eq!(client.cache.lock().connection.status, "disconnected");
        });
    }
}
