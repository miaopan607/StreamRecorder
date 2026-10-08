#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(not(test))]
fn main() {
    streamrecorder_desktop::run();
}

// 二进制测试继承 Tauri 的 Windows manifest，正确载入 Common Controls v6。
#[cfg(test)]
include!("lib.rs");
