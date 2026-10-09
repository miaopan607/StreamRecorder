use parking_lot::Mutex;
use std::{collections::VecDeque, process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
    sync::{mpsc, watch},
    time::Instant,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::*,
};

const ERROR_CAPACITY: usize = 64 * 1024;
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
#[derive(Clone, Debug)]
pub struct ProcessExit {
    pub success: bool,
    pub code: Option<i32>,
    pub stderr: String,
    pub error: String,
}
enum Control {
    Stop,
    Force,
}
pub struct ManagedProcess {
    job: Mutex<Option<ProcessJob>>,
    control: mpsc::UnboundedSender<Control>,
    exit: watch::Receiver<Option<ProcessExit>>,
}
impl ManagedProcess {
    pub async fn spawn(command: &mut Command) -> Result<Arc<Self>, String> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .creation_flags(0x08000000);
        let mut child = command.spawn().map_err(|e| format!("无法启动进程：{e}"))?;
        let attached = child
            .raw_handle()
            .ok_or_else(|| "无法取得进程句柄".to_string())
            .and_then(|handle| ProcessJob::attach(handle as HANDLE));
        let job = match attached {
            Ok(job) => job,
            Err(error) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(format!("无法管理录制进程树：{error}"));
            }
        };
        let (control, commands) = mpsc::unbounded_channel();
        let (finished, exit) = watch::channel(None);
        let process = Arc::new(Self {
            job: Mutex::new(Some(job)),
            control,
            exit,
        });
        let tail = Arc::new(Mutex::new(VecDeque::with_capacity(ERROR_CAPACITY)));
        let mut stderr = child.stderr.take().ok_or("无法取得进程错误管道")?;
        let error_tail = tail.clone();
        // 始终排空字节管道，长行不能扩大缓存或堵住子进程。
        let mut drain = tokio::spawn(async move {
            let mut bytes = [0u8; 8192];
            loop {
                let count = stderr.read(&mut bytes).await?;
                if count == 0 {
                    break;
                }
                let mut tail = error_tail.lock();
                let remove = (tail.len() + count).saturating_sub(ERROR_CAPACITY);
                tail.drain(..remove);
                tail.extend(&bytes[..count]);
            }
            Ok::<(), std::io::Error>(())
        });
        let owner = process.clone();
        let running = tokio::spawn(async move { Self::run(child, commands).await });
        tokio::spawn(async move {
            let result = running.await;
            // 根进程自然退出也回收它的后代，之后才等错误管道 EOF。
            owner.job.lock().take();
            let mut error = String::new();
            match tokio::time::timeout(Duration::from_secs(2), &mut drain).await {
                Ok(Ok(Ok(()))) => {}
                Ok(Ok(Err(e))) => error = format!("错误管道读取失败：{e}"),
                Ok(Err(e)) => error = format!("错误管道任务失败：{e}"),
                Err(_) => {
                    drain.abort();
                    let _ = drain.await;
                    error = "错误管道未按时关闭".into()
                }
            }
            let (success, code) = match result {
                Ok(Ok(status)) => (status.success(), status.code()),
                Ok(Err(e)) => {
                    error = format!("进程等待失败：{e}");
                    (false, None)
                }
                Err(e) => {
                    error = format!("进程管理任务失败：{e}");
                    (false, None)
                }
            };
            let bytes: Vec<u8> = tail.lock().iter().copied().collect();
            finished.send_replace(Some(ProcessExit {
                success,
                code,
                stderr: String::from_utf8_lossy(&bytes).into_owned(),
                error,
            }));
        });
        Ok(process)
    }
    async fn run(
        mut child: Child,
        mut commands: mpsc::UnboundedReceiver<Control>,
    ) -> std::io::Result<std::process::ExitStatus> {
        let mut deadline = None;
        let mut forced = false;
        let mut stdin = child.stdin.take();
        loop {
            let expires = deadline.unwrap_or_else(|| Instant::now() + Duration::from_secs(86400));
            tokio::select! {
                status=child.wait()=>return status,
                command=commands.recv()=>match command{
                    Some(Control::Stop) if deadline.is_none()=>{
                        deadline=Some(Instant::now()+Duration::from_secs(8));
                        if let Some(mut input)=stdin.take(){let _=tokio::time::timeout(Duration::from_millis(100),input.write_all(&[0x71,0x0a])).await;}
                    },
                    Some(Control::Force)=>{child.start_kill()?;forced=true;deadline=Some(Instant::now()+Duration::from_secs(2));},
                    None=>return child.kill().await.and_then(|_|Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe,"进程控制器已关闭"))),
                    _=>{}
                },
                _=tokio::time::sleep_until(expires),if deadline.is_some()=>{
                    if forced{return Err(std::io::Error::new(std::io::ErrorKind::TimedOut,"进程未按时回收"))}
                    child.start_kill()?;forced=true;deadline=Some(Instant::now()+Duration::from_secs(2));
                }
            }
        }
    }
    pub fn force(&self) {
        self.job.lock().take();
        let _ = self.control.send(Control::Force);
    }
    pub async fn wait(&self) -> Result<ProcessExit, String> {
        let mut exit = self.exit.clone();
        loop {
            if let Some(result) = exit.borrow_and_update().clone() {
                return Ok(result);
            }
            exit.changed()
                .await
                .map_err(|_| "进程管理器已停止".to_string())?;
        }
    }
    pub async fn stop(&self) -> Result<ProcessExit, String> {
        let _ = self.control.send(Control::Stop);
        match tokio::time::timeout(Duration::from_secs(8), self.wait()).await {
            Ok(result) => result,
            Err(_) => {
                self.force();
                tokio::time::timeout(Duration::from_secs(2), self.wait())
                    .await
                    .map_err(|_| "进程未按时回收".to_string())?
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn powershell(script: &str) -> Command {
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        command
    }
    #[tokio::test]
    async fn flood_stderr_exits_and_keeps_last_diagnostic() {
        let process=ManagedProcess::spawn(&mut powershell("$s=[Console]::OpenStandardError();$b=New-Object byte[] 8192;for($i=0;$i -lt 300;$i++){$s.Write($b,0,$b.Length)};[Console]::Error.WriteLine('FINAL_DIAGNOSTIC')")).await.unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), process.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(result.success, "{result:?}");
        assert!(result.stderr.contains("FINAL_DIAGNOSTIC"));
        assert!(result.stderr.len() <= ERROR_CAPACITY);
    }
    #[tokio::test]
    async fn batch_stop_has_one_wall_clock_bound() {
        let mut processes = Vec::new();
        for _ in 0..3 {
            processes.push(
                ManagedProcess::spawn(&mut powershell("Start-Sleep -Seconds 60"))
                    .await
                    .unwrap(),
            );
        }
        let started = Instant::now();
        let mut stops = tokio::task::JoinSet::new();
        for process in processes {
            stops.spawn(async move { process.stop().await });
        }
        while let Some(result) = stops.join_next().await {
            result.unwrap().unwrap();
        }
        assert!(started.elapsed() < Duration::from_secs(10));
    }
    #[tokio::test]
    async fn root_exit_and_force_reclaim_descendants() {
        use windows_sys::Win32::System::Threading::{
            OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
        };
        for natural in [true, false] {
            let directory = tempfile::tempdir().unwrap();
            let pid_path = directory.path().join("child.pid");
            let release_path = directory.path().join("release");
            let quote = |path: &std::path::Path| path.to_string_lossy().replace('\'', "''");
            // 后代也使用 CREATE_NO_WINDOW，避免默认终端启动与进程树回收发生竞争。
            let script = format!(
                "$start=New-Object System.Diagnostics.ProcessStartInfo;\
                 $start.FileName=[Diagnostics.Process]::GetCurrentProcess().MainModule.FileName;\
                 $start.Arguments='-NoProfile -NonInteractive -Command Start-Sleep -Seconds 60';\
                 $start.UseShellExecute=$false;$start.CreateNoWindow=$true;\
                 $p=[Diagnostics.Process]::Start($start);\
                 [IO.File]::WriteAllText('{}',[string]$p.Id);\
                 while(-not(Test-Path '{}')){{Start-Sleep -Milliseconds 10}};exit 7",
                quote(&pid_path),
                quote(&release_path),
            );
            let process = ManagedProcess::spawn(&mut powershell(&script))
                .await
                .unwrap();
            tokio::time::timeout(Duration::from_secs(5), async {
                while !pid_path.is_file() {
                    tokio::time::sleep(Duration::from_millis(10)).await
                }
            })
            .await
            .unwrap();
            let pid = std::fs::read_to_string(&pid_path)
                .unwrap()
                .parse::<u32>()
                .unwrap();
            let child = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
            assert!(!child.is_null());
            assert_eq!(
                unsafe { WaitForSingleObject(child, 0) },
                258,
                "受管后代必须先处于运行状态"
            );
            if natural {
                std::fs::write(release_path, b"exit").unwrap();
            } else {
                process.force();
            }
            let result = tokio::time::timeout(Duration::from_secs(3), process.wait())
                .await
                .unwrap()
                .unwrap();
            if natural {
                assert_eq!(result.code, Some(7));
            }
            let stopped = unsafe { WaitForSingleObject(child, 2000) };
            unsafe {
                CloseHandle(child);
            }
            assert_eq!(stopped, 0, "根进程退出后仍有受管后代存活");
        }
    }
}
