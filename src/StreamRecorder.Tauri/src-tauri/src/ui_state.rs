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
    crate::core::config::save_json(path, &payload, false)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowState {
    pub width: f64,
    pub height: f64,
    pub maximized: bool,
}
impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 1380.0,
            height: 860.0,
            maximized: false,
        }
    }
}
pub fn load_window(path: &Path) -> WindowState {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
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
