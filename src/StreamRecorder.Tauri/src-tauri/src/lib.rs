mod core;
mod desktop;
mod paths;
mod ui_state;
use core::{CoreService, models::*};
use serde::Serialize;
use parking_lot::Mutex;
use paths::ProjectPaths;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};
use tauri::{Emitter, Manager, State};

pub struct DesktopState {
    paths: ProjectPaths,
    core: Arc<CoreService>,
    renderer_ready: AtomicBool,
    exit_request: Mutex<Option<String>>,
    exiting: AtomicBool,
    pub tray: AtomicUsize,
}
#[tauri::command]
async fn health_ping(state: State<'_, DesktopState>) -> Result<PingReply, String> {
    state.core.health_ping()
}
#[tauri::command]
async fn settings_update(settings: CoreSettings, state: State<'_, DesktopState>) -> Result<SettingsReply, String> {
    state.core.settings_update(settings).await
}
#[tauri::command]
async fn cookies_get(state: State<'_, DesktopState>) -> Result<CookiesReply, String> {
    state.core.cookies_get()
}
#[tauri::command]
async fn cookies_update(cookies: BTreeMap<String, String>, state: State<'_, DesktopState>) -> Result<CookiesReply, String> {
    state.core.cookies_update(cookies).await
}
#[tauri::command]
async fn accounts_get(state: State<'_, DesktopState>) -> Result<AccountsReply, String> {
    state.core.accounts_get()
}
#[tauri::command]
async fn accounts_update(accounts: BTreeMap<String, Value>, state: State<'_, DesktopState>) -> Result<AccountsReply, String> {
    state.core.accounts_update(accounts).await
}
#[tauri::command]
async fn jobs_upsert(jobs: Vec<JobInput>, state: State<'_, DesktopState>) -> Result<UpsertReply, String> {
    state.core.jobs_upsert(jobs).await
}
#[tauri::command]
async fn jobs_delete(ids: Vec<String>, state: State<'_, DesktopState>) -> Result<DeleteReply, String> {
    state.core.jobs_delete(ids).await
}
#[tauri::command]
async fn jobs_start_monitoring(ids: Vec<String>, state: State<'_, DesktopState>) -> Result<MonitoringReply, String> {
    state.core.jobs_start_monitoring(ids).await
}
#[tauri::command]
async fn jobs_stop_monitoring(ids: Vec<String>, state: State<'_, DesktopState>) -> Result<MonitoringReply, String> {
    state.core.jobs_stop_monitoring(ids).await
}
#[tauri::command]
async fn jobs_recheck(ids: Vec<String>, state: State<'_, DesktopState>) -> Result<RecheckReply, String> {
    state.core.jobs_recheck(ids)
}
#[derive(Serialize)]
struct DesktopBootstrap {
    #[serde(flatten)]
    core: CoreBootstrap,
    dependencies: Value,
    ui_state: ui_state::UiState,
    autostart_enabled: bool,
    app_root: PathBuf,
    data_root: PathBuf,
}
#[tauri::command]
async fn desktop_bootstrap(state: State<'_, DesktopState>) -> Result<DesktopBootstrap, String> {
    let _ = state.core.wait_ready().await;
    state.renderer_ready.store(true, Ordering::Release);
    let dependencies = core::dependencies::inspect().await;
    state.core.set_ffmpeg_available(dependencies["ffmpeg"]["available"].as_bool().unwrap_or(false));
    let path = state.paths.data_root.join("desktop_ui_state.json");
    let ui_state = tokio::task::spawn_blocking(move || ui_state::load(&path)).await.map_err(|error| error.to_string())?;
    Ok(DesktopBootstrap {
        core: state.core.bootstrap(),
        dependencies,
        ui_state,
        autostart_enabled: desktop::get_autostart().unwrap_or(false),
        app_root: state.paths.app_root.clone(),
        data_root: state.paths.data_root.clone(),
    })
}
#[tauri::command]
fn ui_state_set(desktop: State<'_, DesktopState>, state: ui_state::UiState) -> Result<(), String> {
    ui_state::save(
        &desktop.paths.data_root.join("desktop_ui_state.json"),
        &state,
    )
}
#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    desktop::set_autostart(enabled)
}
#[tauri::command]
async fn refresh_dependencies(state: State<'_, DesktopState>) -> Result<Value, String> {
    let deps = core::dependencies::inspect().await;
    state.core.set_ffmpeg_available(deps["ffmpeg"]["available"].as_bool().unwrap_or(false));
    Ok(deps)
}
#[tauri::command]
fn open_data_folder(state: State<'_, DesktopState>) -> Result<(), String> {
    desktop::open_path(&state.paths.data_root)
}
fn job_path(state: &DesktopState, id: &str, latest: bool) -> Result<PathBuf, String> {
    let job = state.core.job(id).ok_or("任务不存在")?;
    let output = job.latest_output_path.as_str();
    if latest {
        return if output.is_empty() {
            Err("当前任务还没有可播放的录制文件".into())
        } else {
            paths::latest_file(&state.paths.resolve(output))
                .ok_or("当前任务还没有可播放的录制文件".into())
        };
    }
    if !output.is_empty() {
        if let Some(parent) = state.paths.resolve(output).parent() {
            return Ok(parent.to_path_buf());
        }
    }
    let specific = job.input.recording_dir.as_str();
    let snapshot = state.core.snapshot();
    let global = snapshot.settings.live_save_path.as_str();
    Ok(if !specific.is_empty() {
        state.paths.resolve(specific)
    } else if !global.is_empty() {
        state.paths.resolve(global)
    } else {
        state.paths.data_root.join("recordings")
    })
}
#[tauri::command]
fn open_job_folder(id: String, state: State<'_, DesktopState>) -> Result<(), String> {
    let path = job_path(&state, &id, false)?;
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    desktop::open_path(&path)
}
#[tauri::command]
fn open_latest_file(id: String, state: State<'_, DesktopState>) -> Result<(), String> {
    desktop::open_path(&job_path(&state, &id, true)?)
}
#[tauri::command]
fn request_exit(app: tauri::AppHandle) {
    begin_exit(&app)
}
#[tauri::command]
fn cancel_exit(request_id: String, state: State<'_, DesktopState>) -> Result<(), String> {
    let mut request = state.exit_request.lock();
    if request.as_deref() != Some(&request_id) {
        return Err("退出请求已失效".into());
    }
    *request = None;
    Ok(())
}
#[tauri::command]
async fn finish_exit(request_id: String, app: tauri::AppHandle) -> Result<(), String> {
    {
        let state = app.state::<DesktopState>();
        if state.exit_request.lock().as_deref() != Some(&request_id) {
            return Err("退出请求已失效".into());
        }
    }
    shutdown(&app).await;
    Ok(())
}
pub(crate) fn begin_exit(app: &tauri::AppHandle) {
    let state = app.state::<DesktopState>();
    if state.exiting.load(Ordering::Acquire) {
        return;
    }
    let id = uuid::Uuid::new_v4().to_string();
    {
        let mut request = state.exit_request.lock();
        if request.is_some() {
            return;
        }
        *request = Some(id.clone());
    }
    if !state.renderer_ready.load(Ordering::Acquire) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            shutdown(&app).await;
        });
        return;
    }
    let target = app.clone();
    let _ = app.run_on_main_thread(move || desktop::restore(&target));
    let _ = app.emit("desktop-exit-request", json!({"request_id":id}));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        let state = app.state::<DesktopState>();
        if state.exiting.load(Ordering::Acquire)
            || state.exit_request.lock().as_deref() != Some(&id)
        {
            return;
        }
        let target = app.clone();
        let _ = app.run_on_main_thread(move || {
            if desktop::confirm(
                "退出程序",
                "界面未响应，强制退出会丢失未保存更改。是否退出？",
            ) {
                let app = target.clone();
                tauri::async_runtime::spawn(async move {
                    shutdown(&app).await;
                });
            } else {
                target.state::<DesktopState>().exit_request.lock().take();
            }
        });
    });
}
async fn shutdown(app: &tauri::AppHandle) {
    let state = app.state::<DesktopState>();
    if state.exiting.swap(true, Ordering::AcqRel) {
        return;
    }
    state.core.shutdown().await;
    desktop::stop_tray(state.tray.swap(0, Ordering::AcqRel));
    app.exit(0);
}

pub fn run() {
    let paths = match ProjectPaths::new() {
        Ok(p) => p,
        Err(e) => {
            desktop::message("启动失败", &e);
            return;
        }
    };
    let startup = std::env::args().any(|a| a.eq_ignore_ascii_case("--startup"));
    let activation = Arc::new(AtomicBool::new(false));
    let second_activation = activation.clone();
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(move |app, _, _| {
            second_activation.store(true, Ordering::Release);
            desktop::restore(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            health_ping,
            settings_update,
            cookies_get,
            cookies_update,
            accounts_get,
            accounts_update,
            jobs_upsert,
            jobs_delete,
            jobs_start_monitoring,
            jobs_stop_monitoring,
            jobs_recheck,
            desktop_bootstrap,
            ui_state_set,
            set_autostart,
            refresh_dependencies,
            open_data_folder,
            open_job_folder,
            open_latest_file,
            request_exit,
            cancel_exit,
            finish_exit
        ])
        .setup(move |app| {
            paths
                .prepare()
                .map_err(std::io::Error::other)?;
            let client = CoreService::new(paths.clone(), Some(app.handle().clone()));
            app.manage(DesktopState {
                paths: paths.clone(),
                core: client.clone(),
                renderer_ready: AtomicBool::new(false),
                exit_request: Mutex::new(None),
                exiting: AtomicBool::new(false),
                tray: AtomicUsize::new(0),
            });
            let icon_path = paths.data_root.join("app.ico");
            std::fs::write(&icon_path, include_bytes!("../icons/StreamRecorder.ico"))?;
            let tray =
                desktop::start_tray(app.handle(), &icon_path).map_err(std::io::Error::other)?;
            app.state::<DesktopState>()
                .tray
                .store(tray, Ordering::Release);
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("StreamRecorder")
            .inner_size(1380., 860.)
            .min_inner_size(960., 640.)
            .center()
            .visible(false)
            .focused(!startup)
            .data_directory(paths.data_root.join("webview2"))
            .icon(tauri::image::Image::from_bytes(include_bytes!(
                "../icons/StreamRecorder.ico"
            ))?)?
            .on_navigation(|url| {
                let bundled = (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                    || (url.scheme() == "http" && url.host_str() == Some("tauri.localhost"));
                let development = cfg!(debug_assertions)
                    && url.scheme() == "http"
                    && matches!(url.host_str(), Some("localhost" | "127.0.0.1"))
                    && url.port() == Some(1420);
                bundled || development
            })
            .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
            .build()?;
            desktop::fit_window(&window).map_err(std::io::Error::other)?;
            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                let state = handle.state::<DesktopState>();
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        if state.exiting.load(Ordering::Acquire) {
                            return;
                        }
                        let close = state.core.close_to_tray();
                        if close {
                            desktop::hide(&handle, true)
                        } else {
                            begin_exit(&handle)
                        }
                    }
                    tauri::WindowEvent::Resized(_) => {
                        let minimize = state.core.minimize_to_tray();
                        if minimize
                            && handle
                                .get_webview_window("main")
                                .is_some_and(|w| w.is_minimized().unwrap_or(false))
                        {
                            desktop::hide(&handle, false)
                        }
                    }
                    _ => {}
                }
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = desktop::migrate_autostart() {
                    client.log(&e)
                }
                let starting = client.clone();
                let result = tauri::async_runtime::spawn(async move { starting.start().await }).await;
                if let Err(error) = result {
                    client.startup_failed(&format!("核心初始化任务异常结束：{error}"));
                }
                if !startup || activation.swap(false, Ordering::AcqRel) {
                    let app = handle.clone();
                    let _ = handle.run_on_main_thread(move || desktop::restore(&app));
                }
            });
            Ok(())
        });
    match builder.build(tauri::generate_context!()) {
        Ok(app) => app.run(|handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !handle
                    .state::<DesktopState>()
                    .exiting
                    .load(Ordering::Acquire)
                {
                    api.prevent_exit();
                    begin_exit(handle);
                }
            }
        }),
        Err(e) => desktop::message(
            "StreamRecorder 启动失败",
            &format!("{e}\n请确认已安装 Microsoft Edge WebView2 Runtime。"),
        ),
    }
}
