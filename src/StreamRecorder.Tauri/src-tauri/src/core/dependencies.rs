use super::recording::find_ffmpeg;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::process::Command;

pub async fn inspect() -> Value {
    let path = find_ffmpeg();
    let (available, version, error) = if let Some(binary) = &path {
        let mut command = Command::new(binary);
        command.arg("-version").kill_on_drop(true).creation_flags(0x08000000);
        match tokio::time::timeout(Duration::from_secs(3), command.output()).await {
            Ok(Ok(output)) => {
                let version = String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("").trim().to_owned();
                if output.status.success() { (true, version, String::new()) }
                else { (false, version, "FFmpeg 检测失败".into()) }
            }
            Ok(Err(error)) => (false, String::new(), error.to_string()),
            Err(_) => (false, String::new(), "依赖检测超时".into()),
        }
    } else { (false, String::new(), "未检测到 ffmpeg".into()) };
    json!({"ffmpeg":{"available":available,"version":version,"path":path.map(|path|path.to_string_lossy().into_owned()).unwrap_or_default(),"error":error}})
}
