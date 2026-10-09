use crate::DesktopState;
use std::{path::Path, ptr::null_mut};
use tauri::{AppHandle, Manager};
use windows_sys::Win32::{
    Foundation::*,
    System::{LibraryLoader::GetModuleHandleW, Registry::*},
    UI::{Shell::*, WindowsAndMessaging::*},
};
const CALLBACK: u32 = WM_APP + 1;
const BALLOON: u32 = WM_APP + 2;
struct TrayContext {
    app: AppHandle,
    icon: HICON,
    taskbar: u32,
}
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
fn fill<const N: usize>(buffer: &mut [u16; N], text: &str) {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut count = units.len().min(N - 1);
    if count > 0 && (0xD800..=0xDBFF).contains(&units[count - 1]) {
        count -= 1
    }
    buffer[..count].copy_from_slice(&units[..count]);
}
unsafe fn data(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
    let mut d: NOTIFYICONDATAW = std::mem::zeroed();
    d.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    d.hWnd = hwnd;
    d.uID = 1;
    d.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
    d.hIcon = icon;
    d.uCallbackMessage = CALLBACK;
    fill(&mut d.szTip, "StreamRecorder");
    d
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(l as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TrayContext;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, w, l);
    }
    let context = &*ptr;
    if msg == context.taskbar {
        let d = data(hwnd, context.icon);
        Shell_NotifyIconW(NIM_ADD, &d);
        return 0;
    }
    match msg {
        CALLBACK => {
            match l as u32 {
                WM_LBUTTONUP | NIN_BALLOONUSERCLICK => {
                    let app = context.app.clone();
                    let target = app.clone();
                    let _ = app.run_on_main_thread(move || restore(&target));
                }
                WM_RBUTTONUP => {
                    let menu = CreatePopupMenu();
                    let restore_text = wide("恢复窗口");
                    let exit_text = wide("退出程序");
                    AppendMenuW(menu, MF_STRING, 1, restore_text.as_ptr());
                    AppendMenuW(menu, MF_STRING, 2, exit_text.as_ptr());
                    let mut point: POINT = std::mem::zeroed();
                    GetCursorPos(&mut point);
                    SetForegroundWindow(hwnd);
                    let command = TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                        point.x,
                        point.y,
                        0,
                        hwnd,
                        std::ptr::null(),
                    );
                    DestroyMenu(menu);
                    let app = context.app.clone();
                    let target = app.clone();
                    if command == 1 {
                        let _ = app.run_on_main_thread(move || restore(&target));
                    } else if command == 2 {
                        crate::begin_exit(&target);
                    }
                    PostMessageW(hwnd, WM_NULL, 0, 0);
                }
                _ => {}
            };
            0
        }
        BALLOON => {
            let message = Box::from_raw(l as *mut (String, String));
            let mut d = data(hwnd, context.icon);
            d.uFlags = NIF_INFO;
            d.dwInfoFlags = NIIF_INFO;
            d.Anonymous.uTimeout = 4000;
            fill(&mut d.szInfoTitle, &message.0);
            fill(&mut d.szInfo, &message.1);
            Shell_NotifyIconW(NIM_MODIFY, &d);
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            let d = data(hwnd, context.icon);
            Shell_NotifyIconW(NIM_DELETE, &d);
            DestroyIcon(context.icon);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(ptr));
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}
pub fn start_tray(app: &AppHandle, icon_path: &Path) -> Result<usize, String> {
    let app = app.clone();
    let icon_path = icon_path.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || unsafe {
        let class = wide("StreamRecorder.Tray");
        let instance = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        if RegisterClassW(&wc) == 0 {
            let _ = tx.send(Err(std::io::Error::last_os_error().to_string()));
            return;
        }
        let path = wide(&icon_path.to_string_lossy());
        let icon = LoadImageW(
            null_mut(),
            path.as_ptr(),
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE | LR_DEFAULTSIZE,
        ) as HICON;
        if icon.is_null() {
            let _ = tx.send(Err("无法载入托盘图标".into()));
            return;
        }
        let taskbar = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
        let context = Box::into_raw(Box::new(TrayContext { app, icon, taskbar }));
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            class.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            instance,
            context as _,
        );
        if hwnd.is_null() {
            drop(Box::from_raw(context));
            DestroyIcon(icon);
            let _ = tx.send(Err(std::io::Error::last_os_error().to_string()));
            return;
        }
        let d = data(hwnd, icon);
        if Shell_NotifyIconW(NIM_ADD, &d) == 0 {
            DestroyWindow(hwnd);
            let _ = tx.send(Err("无法创建系统托盘图标".into()));
            return;
        }
        let _ = tx.send(Ok(hwnd as usize));
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
    rx.recv().map_err(|e| e.to_string())?
}
pub fn stop_tray(handle: usize) {
    if handle != 0 {
        unsafe {
            SendMessageW(handle as HWND, WM_CLOSE, 0, 0);
        }
    }
}
pub fn notify(app: &AppHandle, title: &str, message: &str) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false) {
            return;
        }
    }
    let handle = app
        .state::<DesktopState>()
        .tray
        .load(std::sync::atomic::Ordering::Acquire);
    if handle == 0 {
        return;
    }
    let payload = Box::into_raw(Box::new((title.to_string(), message.to_string())));
    if unsafe { PostMessageW(handle as HWND, BALLOON, 0, payload as isize) } == 0 {
        unsafe {
            drop(Box::from_raw(payload));
        }
    }
}
pub fn fit_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("无法获取当前显示器工作区")?;
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let outer = window.outer_size().map_err(|e| e.to_string())?;
    let inner = window.inner_size().map_err(|e| e.to_string())?;
    let frame_width = outer.width.saturating_sub(inner.width);
    let frame_height = outer.height.saturating_sub(inner.height);
    let margin = (24.0 * scale).round() as u32;
    // 工作区不含任务栏；保留原生边框和两侧留白，避免操作按钮落到屏幕外。
    let room_width = area.size.width.saturating_sub(frame_width + margin * 2);
    let room_height = area.size.height.saturating_sub(frame_height + margin * 2);
    if room_width == 0 || room_height == 0 {
        return Err("当前显示器工作区过小，无法显示窗口".into());
    }
    let width = ((1380.0 * scale).round() as u32).min(room_width);
    let height = ((860.0 * scale).round() as u32).min(room_height);
    window
        .set_min_size(Some(tauri::PhysicalSize::new(
            ((960.0 * scale).round() as u32).min(width),
            ((640.0 * scale).round() as u32).min(height),
        )))
        .map_err(|e| e.to_string())?;
    window
        .set_size(tauri::PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    let outer = window.outer_size().map_err(|e| e.to_string())?;
    window
        .set_position(tauri::PhysicalPosition::new(
            area.position.x + (area.size.width.saturating_sub(outer.width) / 2) as i32,
            area.position.y + (area.size.height.saturating_sub(outer.height) / 2) as i32,
        ))
        .map_err(|e| e.to_string())
}

pub fn restore(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_skip_taskbar(false);
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
pub fn hide(app: &AppHandle, closing: bool) {
    let state = app.state::<DesktopState>();
    let notify_enabled = state.core.tray_notification_enabled(closing);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
        let _ = window.set_skip_taskbar(true);
    }
    if notify_enabled {
        notify(
            app,
            "StreamRecorder",
            if closing {
                "程序已关闭到托盘，录制继续运行"
            } else {
                "程序已最小化到托盘"
            },
        );
    }
}
pub fn message(title: &str, text: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
pub fn confirm(title: &str, text: &str) -> bool {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            MB_YESNO | MB_ICONWARNING,
        ) == IDYES
    }
}
pub fn open_path(path: &Path) -> Result<(), String> {
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            wide("open").as_ptr(),
            wide(&path.to_string_lossy()).as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if result <= 32 {
        Err(format!("打开失败，系统错误代码：{result}"))
    } else {
        Ok(())
    }
}
const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
fn autostart_value() -> Result<Option<String>, String> {
    unsafe {
        let mut key: HKEY = null_mut();
        let code = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        );
        if code == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if code != 0 {
            return Err(format!("读取启动项失败：{code}"));
        }
        let name = wide("StreamRecorder");
        let mut len = 0;
        let mut kind = 0;
        let code = RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            std::ptr::null_mut(),
            &mut len,
        );
        if code == ERROR_FILE_NOT_FOUND {
            RegCloseKey(key);
            return Ok(None);
        }
        if code != 0 || kind != REG_SZ {
            RegCloseKey(key);
            return Err(format!("读取启动项失败：{code}"));
        }
        let mut buffer = vec![0u16; len as usize / 2 + 1];
        let code = RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            buffer.as_mut_ptr() as _,
            &mut len,
        );
        RegCloseKey(key);
        if code != 0 {
            return Err(format!("读取启动项失败：{code}"));
        }
        let n = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        Ok(Some(String::from_utf16_lossy(&buffer[..n])))
    }
}
pub fn get_autostart() -> Result<bool, String> {
    Ok(autostart_value()?.is_some_and(|v| !v.is_empty()))
}
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err("请在发布版中设置开机启动".into());
    }
    set_run_value(if enabled {
        Some(format!(
            "\"{}\" --startup",
            std::env::current_exe()
                .map_err(|e| e.to_string())?
                .display()
        ))
    } else {
        None
    })
}
fn set_run_value(value: Option<String>) -> Result<(), String> {
    unsafe {
        let mut key: HKEY = null_mut();
        let code = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        );
        if code != 0 {
            return Err(format!("无法访问系统启动项注册表：{code}"));
        }
        let name = wide("StreamRecorder");
        let code = if let Some(value) = value {
            let value = wide(&value);
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                value.as_ptr() as _,
                (value.len() * 2) as u32,
            )
        } else {
            let code = RegDeleteValueW(key, name.as_ptr());
            if code == ERROR_FILE_NOT_FOUND {
                0
            } else {
                code
            }
        };
        RegCloseKey(key);
        if code == 0 {
            Ok(())
        } else {
            Err(format!("更新启动项失败：{code}"))
        }
    }
}
pub fn migrate_autostart() -> Result<(), String> {
    if !cfg!(debug_assertions) && get_autostart()? {
        let expected = format!(
            "\"{}\" --startup",
            std::env::current_exe()
                .map_err(|e| e.to_string())?
                .display()
        );
        if autostart_value()?.as_deref() != Some(&expected) {
            set_run_value(Some(expected))?;
        }
    }
    Ok(())
}
