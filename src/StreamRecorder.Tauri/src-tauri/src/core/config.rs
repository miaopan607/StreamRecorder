use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub struct ConfigStore {
    pub root: PathBuf,
}
impl ConfigStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn load<T: DeserializeOwned + Serialize + Default>(
        &self,
        name: &str,
        logs: &mut Vec<String>,
    ) -> Result<T, String> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let path = self.root.join(name);
        for (candidate, label) in [
            (path.clone(), "主文件"),
            (suffix(&path, ".bak"), "备份"),
            (suffix(&path, ".tmp"), "临时文件"),
        ] {
            if let Some((value, repaired)) = read_valid::<T>(&candidate)? {
                if candidate != path || repaired {
                    save_json(&path, &value, false)?;
                    logs.push(format!("{name} 已从{label}恢复。"));
                }
                return Ok(value);
            }
        }
        if path.exists() {
            let base = format!(".corrupt-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"));
            let mut target = suffix(&path, &base);
            let mut counter = 1;
            while target.exists() {
                target = suffix(&path, &format!("{base}-{counter}"));
                counter += 1;
            }
            fs::rename(&path, &target).map_err(|e| e.to_string())?;
            logs.push(format!(
                "{name} 已损坏，已隔离为 {}。",
                target.file_name().unwrap().to_string_lossy()
            ));
        }
        let value = T::default();
        save_json(&path, &value, false)?;
        logs.push(format!("{name} 已重建默认内容。"));
        Ok(value)
    }
    pub fn save<T: Serialize>(&self, name: &str, value: &T) -> Result<(), String> {
        save_json(&self.root.join(name), value, true)
    }
}
fn suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut text = path.as_os_str().to_os_string();
    text.push(suffix);
    PathBuf::from(text)
}
fn read_valid<T: DeserializeOwned>(path: &Path) -> Result<Option<(T, bool)>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("无法读取 {}：{e}", path.display())),
    };
    if let Ok(value) = serde_json::from_slice(&bytes) {
        return Ok(Some((value, false)));
    }
    let trimmed = bytes.trim_ascii_end();
    let end = trimmed.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
    Ok(serde_json::from_slice(&trimmed[..end])
        .ok()
        .map(|v| (v, true)))
}
pub fn save_json<T: Serialize>(path: &Path, value: &T, backup: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp = suffix(path, ".tmp");
    let result = (|| {
        let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(&mut file, value).map_err(|e| e.to_string())?;
        file.write_all(b"\n")
            .and_then(|_| file.flush())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        // 损坏的主文件不覆盖最后一份有效备份。
        if backup && read_valid::<serde_json::Value>(path)?.is_some() {
            fs::copy(path, suffix(path, ".bak")).map_err(|e| e.to_string())?;
        }
        replace_file(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
pub(crate) fn replace_file(from: &Path, to: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe {
            MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::rename(from, to).map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::super::models::{CoreSettings, JobInput, RecordingJob};
    use super::*;
    #[test]
    fn recovery_keeps_unknown_and_legacy_data() {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(dir.path().into());
        let mut logs = vec![];
        fs::write(
            dir.path().join("core_settings.json"),
            "{\"live_save_path\":\"中文 录制\",\"Extra\":{\"enabled\":true}}\0\0\n",
        )
        .unwrap();
        let settings: CoreSettings = store.load("core_settings.json", &mut logs).unwrap();
        assert_eq!(settings.live_save_path, "中文 录制");
        assert_eq!(settings.extra["Extra"]["enabled"], true);
        store.save("core_settings.json", &settings).unwrap();
        fs::write(dir.path().join("core_settings.json"), b"broken").unwrap();
        let restored: CoreSettings = store.load("core_settings.json", &mut logs).unwrap();
        assert_eq!(restored.live_save_path, "中文 录制");
        assert_eq!(restored.extra, settings.extra);
        fs::write(dir.path().join("jobs.json.tmp"), r#"[{"rec_id":"old-id","url":"https://live.bilibili.com/123","recordFormat":"MP4","monitorStatus":false}]"#).unwrap();
        let jobs: Vec<JobInput> = store.load("jobs.json", &mut logs).unwrap();
        let job = RecordingJob::new(jobs[0].clone());
        assert_eq!(job.id(), "old-id");
        assert_eq!(job.input.record_format, "MP4");
        assert_eq!(job.status_info, "已停止");
        assert_eq!(job.platform_key, "bilibili");
    }
    #[cfg(windows)]
    #[test]
    fn failed_replace_preserves_existing_file() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        save_json(&path, &serde_json::json!({"value":"old"}), false).unwrap();
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&path)
            .unwrap();
        assert!(save_json(&path, &serde_json::json!({"value":"new"}), true).is_err());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&path).unwrap()).unwrap()
                ["value"],
            "old"
        );
        drop(held);
    }
}
