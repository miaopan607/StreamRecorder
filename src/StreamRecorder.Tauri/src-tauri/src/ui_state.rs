use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UiState {
    #[serde(default = "yes")]
    pub is_card_layout: bool,
    #[serde(default)]
    pub visible_columns: BTreeMap<String, bool>,
}
fn yes() -> bool {
    true
}
impl Default for UiState {
    fn default() -> Self {
        Self {
            is_card_layout: true,
            visible_columns: BTreeMap::new(),
        }
    }
}
pub fn load(path: &Path) -> UiState {
    let value = fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    let Some(value) = value else {
        return UiState::default();
    };
    UiState {
        is_card_layout: value
            .get("IsCardLayout")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        visible_columns: value
            .get("VisibleColumns")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
    }
}
pub fn save(path: &Path, state: &UiState) -> Result<(), String> {
    let mut payload = fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .filter(Value::is_object)
        .unwrap_or(json!({}));
    payload["IsCardLayout"] = json!(state.is_card_layout);
    payload["VisibleColumns"] = json!(state.visible_columns);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp = path.with_extension("json.tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(&payload).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // Windows 的 rename 不覆盖已有文件，使用原生替换保证旧文件在写失败时仍在。
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
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
    }
    #[cfg(not(windows))]
    fs::rename(temp, path).map_err(|e| e.to_string())?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_unknown_and_old_columns() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("desktop_ui_state.json");
        fs::write(
            &p,
            r#"{"IsCardLayout":false,"VisibleColumns":{"UrlColumn":false},"Extra":42}"#,
        )
        .unwrap();
        let mut state = load(&p);
        assert!(!state.is_card_layout);
        assert_eq!(state.visible_columns["UrlColumn"], false);
        state.is_card_layout = true;
        save(&p, &state).unwrap();
        let value: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
        assert_eq!(value["Extra"], 42);
        assert_eq!(value["VisibleColumns"]["UrlColumn"], false);
    }
}
