use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct ProjectPaths {
    pub app_root: PathBuf,
    pub data_root: PathBuf,
}
impl ProjectPaths {
    pub fn new() -> Result<Self, String> {
        let root = if cfg!(debug_assertions) {
            std::env::var_os("STREAMRECORDER_DEV_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.."))
                .canonicalize()
                .map_err(|e| e.to_string())?
        } else {
            std::env::current_exe()
                .map_err(|e| e.to_string())?
                .parent()
                .ok_or("无法确定应用目录")?
                .to_path_buf()
        };
        Ok(Self::at(root))
    }
    pub fn at(app_root: PathBuf) -> Self {
        Self {
            data_root: app_root.join("runtime"),
            app_root,
        }
    }
    pub fn prepare(&self) -> Result<(), String> {
        fs::create_dir_all(&self.data_root)
            .map_err(|e| format!("无法写入应用目录，请将程序移到可写目录：{e}"))
    }
    pub fn resolve(&self, text: &str) -> PathBuf {
        let expanded = if text == "~" || text.starts_with("~/") || text.starts_with("~\\") {
            std::env::var_os("USERPROFILE")
                .map(|home| PathBuf::from(home).join(text.get(2..).unwrap_or("")))
                .unwrap_or_else(|| PathBuf::from(text))
        } else {
            PathBuf::from(text)
        };
        if expanded.is_absolute() {
            expanded
        } else {
            self.app_root.join(expanded)
        }
    }
}

pub fn latest_file(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    let name = path.file_name()?.to_str()?;
    let (prefix, suffix) = name.split_once("%03d")?;
    fs::read_dir(path.parent()?)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            entry.path().is_file() && name.starts_with(prefix) && name.ends_with(suffix)
        })
        .max_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok())
        .map(|entry| entry.path())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_paths_and_segments() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ProjectPaths::at(dir.path().join("中文 空格"));
        let recordings = paths.resolve("录制");
        fs::create_dir_all(&recordings).unwrap();
        assert_eq!(recordings, paths.app_root.join("录制"));
        let first = recordings.join("直播_000.ts");
        fs::write(&first, b"first").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        let last = recordings.join("直播_001.ts");
        fs::write(&last, b"last").unwrap();
        assert_eq!(latest_file(&recordings.join("直播_%03d.ts")), Some(last));
        assert!(latest_file(&recordings.join("不存在.ts")).is_none());
    }
    #[test]
    fn preparing_runtime_keeps_existing_user_directories() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ProjectPaths::at(dir.path().to_path_buf());
        let old_worker = paths.app_root.join("worker");
        fs::create_dir_all(&old_worker).unwrap();
        fs::write(old_worker.join("user-file.txt"), b"keep").unwrap();
        paths.prepare().unwrap();
        let jobs = paths.data_root.join("jobs.json");
        fs::write(&jobs, b"user jobs").unwrap();
        paths.prepare().unwrap();
        assert_eq!(fs::read(jobs).unwrap(), b"user jobs");
        assert_eq!(fs::read(old_worker.join("user-file.txt")).unwrap(), b"keep");
    }
}
