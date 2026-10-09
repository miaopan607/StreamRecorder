use super::{
    config::ConfigStore,
    models::*,
    notifications::{NotificationEvent, NotificationService},
    probe::{ProbeInput, ProbeService},
    process::ManagedProcess,
    recording,
};
use crate::paths::ProjectPaths;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter};
use tokio::{
    sync::{watch, Mutex as AsyncMutex, Notify},
    time::Instant,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

struct ActiveRecording {
    id: String,
    process: Arc<ManagedProcess>,
    output: PathBuf,
    started: Instant,
    last_measure: Instant,
    last_bytes: u64,
}
struct JobRuntime {
    generation: u64,
    cancel: CancellationToken,
    lifecycle: Arc<AsyncMutex<()>>,
    checking: Option<u64>,
    queued: bool,
    recording: Option<ActiveRecording>,
    was_live: bool,
}
impl JobRuntime {
    fn new(cancel: CancellationToken) -> Self {
        Self {
            generation: 0,
            cancel,
            lifecycle: Arc::new(AsyncMutex::new(())),
            checking: None,
            queued: false,
            recording: None,
            was_live: false,
        }
    }
}
#[derive(Clone)]
struct QueuedProbe {
    id: String,
    generation: u64,
    platform: String,
}
struct ProbeTicket {
    queue: QueuedProbe,
    input: ProbeInput,
    settings: Arc<CoreSettings>,
    credentials_revision: u64,
    cancel: CancellationToken,
    lifecycle: Arc<AsyncMutex<()>>,
}
struct CoreData {
    settings: CoreSettings,
    cookies: BTreeMap<String, String>,
    accounts: BTreeMap<String, Value>,
    jobs: Vec<RecordingJob>,
    runtime: HashMap<String, JobRuntime>,
    revision: u64,
    connection: CoreState,
    logs: VecDeque<String>,
    initialized: bool,
    closing: bool,
    ffmpeg_available: bool,
    credentials_revision: u64,
    queue: VecDeque<QueuedProbe>,
    active_probe_count: usize,
    active_per_platform: HashMap<String, usize>,
}
impl CoreData {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            revision: self.revision,
            app: CoreInfo {
                name: "StreamRecorder 核心".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ffmpeg_available: self.ffmpeg_available,
                updated_at: utc_now(),
            },
            settings: self.settings.clone(),
            jobs: self.jobs.clone(),
        }
    }
    fn changed(&mut self) -> Snapshot {
        self.revision += 1;
        self.connection.revision = self.revision;
        self.snapshot()
    }
    fn ready(&self) -> Result<(), String> {
        if self.closing {
            return Err("核心服务正在退出".into());
        }
        if self.connection.status != "connected" {
            return Err(if self.connection.error.is_empty() {
                "核心服务尚未就绪".into()
            } else {
                self.connection.error.clone()
            });
        }
        Ok(())
    }
    fn current(&self, id: &str, generation: u64) -> bool {
        !self.closing
            && self
                .runtime
                .get(id)
                .is_some_and(|runtime| runtime.generation == generation)
            && self
                .jobs
                .iter()
                .any(|job| job.id() == id && job.input.monitor_status)
    }
    fn invalidate(&mut self, id: &str, root: &CancellationToken) {
        if let Some(runtime) = self.runtime.get_mut(id) {
            runtime.generation += 1;
            runtime.cancel.cancel();
            runtime.cancel = root.child_token();
            runtime.queued = false;
            runtime.was_live = false;
        }
        self.queue.retain(|queued| queued.id != id);
    }
    fn enqueue(&mut self, ids: &[String], all: bool) -> Vec<String> {
        if self.closing {
            return Vec::new();
        }
        let mut queued = Vec::new();
        for job in &self.jobs {
            if !job.input.monitor_status || (!all && !ids.iter().any(|id| id == job.id())) {
                continue;
            }
            let Some(runtime) = self.runtime.get_mut(job.id()) else {
                continue;
            };
            if runtime.queued || runtime.checking.is_some() || runtime.recording.is_some() {
                continue;
            }
            runtime.queued = true;
            self.queue.push_back(QueuedProbe {
                id: job.id().into(),
                generation: runtime.generation,
                platform: job.platform_key.clone(),
            });
            queued.push(job.id().into());
        }
        queued
    }
}
#[derive(Clone, Serialize)]
struct SnapshotEvent<'a> {
    name: &'static str,
    body: &'a Snapshot,
}
pub struct CoreService {
    pub paths: ProjectPaths,
    data: Mutex<CoreData>,
    transactions: AsyncMutex<()>,
    pub probe: ProbeService,
    notifications: Arc<NotificationService>,
    cancel: CancellationToken,
    ready: watch::Sender<bool>,
    stopped: watch::Sender<bool>,
    wake: Notify,
    stats_busy: AtomicBool,
    tasks: TaskTracker,
    app: Option<AppHandle>,
}
impl CoreService {
    pub fn new(paths: ProjectPaths, app: Option<AppHandle>) -> Arc<Self> {
        // 分发必须保留上游许可，阻止优化器删除未展示的内嵌文本。
        std::hint::black_box(super::signing::STREAMGET_LICENSE);
        let (ready, _) = watch::channel(false);
        let (stopped, _) = watch::channel(false);
        Arc::new(Self {
            paths,
            data: Mutex::new(CoreData {
                settings: CoreSettings::default(),
                cookies: BTreeMap::new(),
                accounts: BTreeMap::new(),
                jobs: Vec::new(),
                runtime: HashMap::new(),
                revision: 0,
                connection: CoreState {
                    status: "starting".into(),
                    error: String::new(),
                    revision: 0,
                },
                logs: VecDeque::new(),
                initialized: false,
                closing: false,
                ffmpeg_available: false,
                credentials_revision: 0,
                queue: VecDeque::new(),
                active_probe_count: 0,
                active_per_platform: HashMap::new(),
            }),
            transactions: AsyncMutex::new(()),
            probe: ProbeService::default(),
            notifications: Arc::new(NotificationService::default()),
            cancel: CancellationToken::new(),
            ready,
            stopped,
            wake: Notify::new(),
            stats_busy: AtomicBool::new(false),
            tasks: TaskTracker::new(),
            app,
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        self.data.lock().snapshot()
    }
    pub fn bootstrap(&self) -> CoreBootstrap {
        let data = self.data.lock();
        CoreBootstrap {
            snapshot: data.snapshot(),
            core_state: data.connection.clone(),
            cookies: data.cookies.clone(),
            accounts: data.accounts.clone(),
            logs: data.logs.iter().cloned().collect(),
        }
    }
    pub fn startup_failed(&self, error: &str) {
        self.fail(error);
        self.ready.send_replace(true);
    }
    pub fn close_to_tray(&self) -> bool {
        self.data.lock().settings.minimize_to_tray_on_close
    }
    pub fn minimize_to_tray(&self) -> bool {
        self.data.lock().settings.minimize_to_tray_on_minimize
    }
    pub fn tray_notification_enabled(&self, closing: bool) -> bool {
        let data = self.data.lock();
        data.settings.system_notification_enabled && if closing {
            data.settings.system_close_to_tray_notification_enabled
        } else {
            data.settings.system_minimize_to_tray_notification_enabled
        }
    }
    pub fn connection(&self) -> CoreState {
        self.data.lock().connection.clone()
    }
    pub fn job(&self, id: &str) -> Option<RecordingJob> {
        self.data
            .lock()
            .jobs
            .iter()
            .find(|job| job.id() == id)
            .cloned()
    }
    fn publish(&self, snapshot: Snapshot) {
        if let Some(app) = &self.app {
            let _ = app.emit(
                "core-event",
                SnapshotEvent {
                    name: "snapshot_changed",
                    body: &snapshot,
                },
            );
        }
    }
    fn publish_connection(&self) {
        if let Some(app) = &self.app {
            let _ = app.emit("core-state", self.connection());
        }
    }
    pub fn log(&self, text: &str) {
        let line = format!("[{}] {text}", chrono::Local::now().format("%H:%M:%S"));
        {
            let mut data = self.data.lock();
            data.logs.push_front(line.clone());
            data.logs.truncate(300);
        }
        if let Some(app) = &self.app {
            let _ = app.emit("core-log", line);
        }
    }
    pub async fn wait_ready(&self) -> Result<(), String> {
        let mut ready = self.ready.subscribe();
        if !*ready.borrow_and_update() {
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    ready
                        .changed()
                        .await
                        .map_err(|_| "核心启动状态已关闭".to_string())?;
                    if *ready.borrow_and_update() {
                        return Ok::<(), String>(());
                    }
                }
            })
            .await
            .map_err(|_| "核心服务启动超时".to_string())??;
        }
        self.data.lock().ready()
    }
    pub async fn start(self: &Arc<Self>) -> Result<(), String> {
        let store = ConfigStore::new(self.paths.data_root.clone());
        let loading = tokio::task::spawn_blocking(move || {
            let mut logs = Vec::new();
            let settings = store.load::<CoreSettings>("core_settings.json", &mut logs)?;
            let jobs = store.load::<Vec<JobInput>>("jobs.json", &mut logs)?;
            let cookies = store.load::<BTreeMap<String, String>>("cookies.json", &mut logs)?;
            let accounts = store.load::<BTreeMap<String, Value>>("accounts.json", &mut logs)?;
            Ok::<_, String>((
                settings,
                jobs,
                cookies,
                accounts,
                logs,
                recording::find_ffmpeg().is_some(),
            ))
        });
        let loaded = match tokio::time::timeout(Duration::from_secs(10), loading).await {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => Err(format!("核心初始化任务失败：{error}")),
            Err(_) => Err("核心配置加载超时".into()),
        };
        let (settings, jobs, cookies, accounts, logs, ffmpeg) = match loaded {
            Ok(value) => value,
            Err(error) => {
                self.fail(&error);
                self.ready.send_replace(true);
                return Err(error);
            }
        };
        let snapshot = {
            let mut data = self.data.lock();
            if data.initialized || data.closing {
                return Err("核心服务已启动或正在退出".into());
            }
            data.settings = settings;
            data.jobs = jobs.into_iter().map(RecordingJob::new).collect();
            data.cookies = cookies;
            data.accounts = accounts;
            data.ffmpeg_available = ffmpeg;
            let ids = data
                .jobs
                .iter()
                .map(|job| job.id().to_owned())
                .collect::<Vec<_>>();
            for id in ids {
                data.runtime
                    .insert(id, JobRuntime::new(self.cancel.child_token()));
            }
            data.initialized = true;
            data.connection.status = "connected".into();
            data.connection.error.clear();
            data.changed()
        };
        for line in logs {
            self.log(&line)
        }
        self.publish(snapshot);
        self.publish_connection();
        self.ready.send_replace(true);
        let core = self.clone();
        let supervised = self.clone();
        let running = tokio::spawn(async move { core.monitor().await });
        tokio::spawn(async move {
            let result = running.await;
            if !supervised.data.lock().closing {
                let error = match result {
                    Ok(()) => "监控调度意外停止".into(),
                    Err(error) => format!("监控调度异常结束：{error}"),
                };
                supervised.fail(&error);
                supervised.shutdown().await;
            }
        });
        Ok(())
    }
    fn fail(&self, error: &str) {
        let snapshot = {
            let mut data = self.data.lock();
            data.connection.status = "disconnected".into();
            data.connection.error = error.into();
            data.changed()
        };
        self.log(error);
        self.publish(snapshot);
        self.publish_connection();
    }
    pub fn health_ping(&self) -> Result<PingReply, String> {
        self.data.lock().ready()?;
        Ok(PingReply { utc: utc_now() })
    }
    pub fn cookies_get(&self) -> Result<CookiesReply, String> {
        let data = self.data.lock();
        data.ready()?;
        Ok(CookiesReply {
            cookies: data.cookies.clone(),
        })
    }
    pub fn accounts_get(&self) -> Result<AccountsReply, String> {
        let data = self.data.lock();
        data.ready()?;
        Ok(AccountsReply {
            accounts: data.accounts.clone(),
        })
    }
    async fn save<T: Serialize + Send + 'static>(
        &self,
        name: &'static str,
        value: T,
    ) -> Result<T, String> {
        let store = ConfigStore::new(self.paths.data_root.clone());
        tokio::task::spawn_blocking(move || {
            store.save(name, &value)?;
            Ok(value)
        })
        .await
        .map_err(|e| format!("配置保存任务失败：{e}"))?
    }
    // 配置事务由独立任务持有，invoke 被丢弃也不能让尚未结束的落盘越过后续写入。
    pub async fn settings_update(
        self: &Arc<Self>,
        mut settings: CoreSettings,
    ) -> Result<SettingsReply, String> {
        let core = self.clone();
        self.tasks
            .spawn(async move {
                let _transaction = core.transactions.lock().await;
                {
                    let data = core.data.lock();
                    data.ready()?;
                    for (key, value) in &data.settings.extra {
                        settings
                            .extra
                            .entry(key.clone())
                            .or_insert_with(|| value.clone());
                    }
                }
                let settings = core.save("core_settings.json", settings).await?;
                let snapshot = {
                    let mut data = core.data.lock();
                    data.settings = settings.clone();
                    data.changed()
                };
                core.publish(snapshot);
                core.wake.notify_one();
                Ok(SettingsReply { settings })
            })
            .await
            .map_err(|e| format!("设置事务失败：{e}"))?
    }
    pub async fn cookies_update(
        self: &Arc<Self>,
        cookies: BTreeMap<String, String>,
    ) -> Result<CookiesReply, String> {
        let core = self.clone();
        self.tasks
            .spawn(async move {
                let _transaction = core.transactions.lock().await;
                core.data.lock().ready()?;
                let cookies = core.save("cookies.json", cookies).await?;
                let snapshot = {
                    let mut data = core.data.lock();
                    data.cookies = cookies.clone();
                    data.credentials_revision += 1;
                    data.changed()
                };
                core.probe.invalidate_sessions();
                core.publish(snapshot);
                Ok(CookiesReply { cookies })
            })
            .await
            .map_err(|e| format!("Cookie 事务失败：{e}"))?
    }
    pub async fn accounts_update(
        self: &Arc<Self>,
        accounts: BTreeMap<String, Value>,
    ) -> Result<AccountsReply, String> {
        let core = self.clone();
        self.tasks
            .spawn(async move {
                let _transaction = core.transactions.lock().await;
                core.data.lock().ready()?;
                let accounts = core.save("accounts.json", accounts).await?;
                let snapshot = {
                    let mut data = core.data.lock();
                    data.accounts = accounts.clone();
                    data.credentials_revision += 1;
                    data.changed()
                };
                core.probe.invalidate_sessions();
                core.publish(snapshot);
                Ok(AccountsReply { accounts })
            })
            .await
            .map_err(|e| format!("账号事务失败：{e}"))?
    }
    pub async fn jobs_upsert(self: &Arc<Self>, jobs: Vec<JobInput>) -> Result<UpsertReply, String> {
        let candidates = jobs.into_iter().map(RecordingJob::new).collect::<Vec<_>>();
        for job in &candidates {
            if !valid_url(&job.input.url) {
                return Err("直播地址无效".into());
            }
        }
        if candidates.is_empty() {
            self.data.lock().ready()?;
            return Ok(UpsertReply {
                changed_ids: Vec::new(),
            });
        }
        let core = self.clone();
        self.tasks
            .spawn(async move {
                let transaction = core.transactions.lock().await;
                let mut desired = {
                    let data = core.data.lock();
                    data.ready()?;
                    data.jobs
                        .iter()
                        .map(|job| job.input.clone())
                        .collect::<Vec<_>>()
                };
                let mut changed_ids = Vec::new();
                for mut candidate in candidates {
                    let existing = desired
                        .iter()
                        .position(|job| job.id.as_deref() == Some(candidate.id()))
                        .or_else(|| {
                            let url = candidate.input.url.to_lowercase();
                            desired.iter().position(|job| {
                                job.url.to_lowercase() == url
                            })
                        });
                    if let Some(index) = existing {
                        candidate.input.id = desired[index].id.clone();
                        candidate.input.created_at = desired[index].created_at.clone();
                        candidate.input.updated_at = utc_now();
                        changed_ids.push(candidate.id().to_owned());
                        desired[index] = candidate.input;
                    } else {
                        changed_ids.push(candidate.id().to_owned());
                        desired.push(candidate.input);
                    }
                }
                let desired = core.save("jobs.json", desired).await?;
                let (snapshot, stops) = {
                    let mut data = core.data.lock();
                    let mut stops = Vec::new();
                    for input in desired {
                        let id = input.id.as_deref().unwrap().to_owned();
                        if !changed_ids.contains(&id) {
                            continue;
                        }
                        data.invalidate(&id, &core.cancel);
                        if !input.monitor_status {
                            if let Some(runtime) = data.runtime.get(&id) {
                                stops.push((runtime.lifecycle.clone(), runtime.recording.as_ref().map(|recording| recording.process.clone())));
                            }
                        }
                        if let Some(job) = data.jobs.iter_mut().find(|job| job.id() == id) {
                            job.input = input;
                            job.last_checked_at.clear();
                            job.error_message.clear();
                            job.refresh(true);
                            if !job.input.monitor_status { job.status_info = STOPPED.into(); }
                        } else {
                            data.runtime
                                .insert(id, JobRuntime::new(core.cancel.child_token()));
                            data.jobs.push(RecordingJob::new(input));
                        }
                    }
                    (data.changed(), stops)
                };
                core.publish(snapshot);
                core.wake.notify_one();
                drop(transaction);
                core.stop_collected(stops).await?;
                Ok(UpsertReply { changed_ids })
            })
            .await
            .map_err(|e| format!("任务事务失败：{e}"))?
    }
    pub async fn jobs_delete(self: &Arc<Self>, ids: Vec<String>) -> Result<DeleteReply, String> {
        if ids.is_empty() {
            self.data.lock().ready()?;
            return Ok(DeleteReply { deleted: 0 });
        }
        let core = self.clone();
        self.tasks
            .spawn(async move {
                let transaction = core.transactions.lock().await;
                let (desired, deleted) = {
                    let data = core.data.lock();
                    data.ready()?;
                    let desired = data.jobs
                        .iter()
                        .filter(|job| !ids.iter().any(|id| id == job.id()))
                        .map(|job| job.input.clone())
                        .collect::<Vec<_>>();
                    let deleted = data.jobs.len() - desired.len();
                    (desired, deleted)
                };
                if deleted == 0 {
                    return Ok(DeleteReply { deleted: 0 });
                }
                core.save("jobs.json", desired).await?;
                let (snapshot, stops) = {
                    let mut data = core.data.lock();
                    let mut stops = Vec::new();
                    for id in &ids {
                        data.invalidate(id, &core.cancel);
                        if let Some(runtime) = data.runtime.remove(id) {
                            stops.push((
                                runtime.lifecycle,
                                runtime.recording.map(|recording| recording.process),
                            ));
                        }
                    }
                    data.jobs.retain(|job| !ids.iter().any(|id| id == job.id()));
                    (data.changed(), stops)
                };
                core.publish(snapshot);
                drop(transaction);
                core.stop_collected(stops).await?;
                Ok(DeleteReply { deleted })
            })
            .await
            .map_err(|e| format!("删除事务失败：{e}"))?
    }
    pub async fn jobs_start_monitoring(
        self: &Arc<Self>,
        ids: Vec<String>,
    ) -> Result<MonitoringReply, String> {
        self.set_monitoring(ids, true).await
    }
    pub async fn jobs_stop_monitoring(
        self: &Arc<Self>,
        ids: Vec<String>,
    ) -> Result<MonitoringReply, String> {
        self.set_monitoring(ids, false).await
    }
    async fn set_monitoring(
        self: &Arc<Self>,
        ids: Vec<String>,
        enabled: bool,
    ) -> Result<MonitoringReply, String> {
        let core = self.clone();
        self.tasks
            .spawn(async move {
                let transaction = core.transactions.lock().await;
                let (mut desired, changed_ids) = {
                    let data = core.data.lock();
                    data.ready()?;
                    let changed = data
                        .jobs
                        .iter()
                        .filter(|job| ids.is_empty() || ids.iter().any(|id| id == job.id()))
                        .map(|job| job.id().to_owned())
                        .collect::<Vec<_>>();
                    (
                        data.jobs
                            .iter()
                            .map(|job| job.input.clone())
                            .collect::<Vec<_>>(),
                        changed,
                    )
                };
                if changed_ids.is_empty() {
                    return Ok(MonitoringReply {
                        changed_ids,
                        monitoring: enabled,
                    });
                }
                for input in &mut desired {
                    if changed_ids
                        .iter()
                        .any(|id| Some(id.as_str()) == input.id.as_deref())
                    {
                        input.monitor_status = enabled;
                    }
                }
                core.save("jobs.json", desired).await?;
                let (snapshot, stops) = {
                    let mut data = core.data.lock();
                    let mut stops = Vec::new();
                    for id in &changed_ids {
                        data.invalidate(id, &core.cancel);
                        if let Some(job) = data.jobs.iter_mut().find(|job| job.id() == id) {
                            job.input.monitor_status = enabled;
                            job.last_checked_at.clear();
                            job.error_message.clear();
                            job.status_info = if enabled { MONITORING } else { STOPPED }.into();
                            job.refresh(true);
                        }
                        if !enabled {
                            if let Some(runtime) = data.runtime.get(id) {
                                stops.push((
                                    runtime.lifecycle.clone(),
                                    runtime
                                        .recording
                                        .as_ref()
                                        .map(|recording| recording.process.clone()),
                                ));
                            }
                        }
                    }
                    (data.changed(), stops)
                };
                core.publish(snapshot);
                core.wake.notify_one();
                drop(transaction);
                core.stop_collected(stops).await?;
                Ok(MonitoringReply {
                    changed_ids,
                    monitoring: enabled,
                })
            })
            .await
            .map_err(|e| format!("监控事务失败：{e}"))?
    }
    async fn stop_collected(
        &self,
        stops: Vec<(Arc<AsyncMutex<()>>, Option<Arc<ManagedProcess>>)>,
    ) -> Result<(), String> {
        let mut tasks = tokio::task::JoinSet::new();
        for (lifecycle, process) in stops {
            tasks.spawn(async move {
                let _lifecycle = lifecycle.lock().await;
                if let Some(process) = process {
                    process.stop().await?;
                }
                Ok::<(), String>(())
            });
        }
        let mut errors = Vec::new();
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => errors.push(error),
                Err(error) => errors.push(error.to_string()),
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("；"))
        }
    }
    pub fn jobs_recheck(&self, ids: Vec<String>) -> Result<RecheckReply, String> {
        let queued_ids = {
            let mut data = self.data.lock();
            data.ready()?;
            data.enqueue(&ids, ids.is_empty())
        };
        self.wake.notify_one();
        Ok(RecheckReply { queued_ids })
    }
    async fn monitor(self: &Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            tokio::select! {_=self.cancel.cancelled()=>break,_=interval.tick()=>{},_=self.wake.notified()=>{}}
            {
                let mut data = self.data.lock();
                let seconds = data
                    .settings
                    .loop_time_seconds
                    .parse::<f64>()
                    .ok()
                    .filter(|value| value.is_finite())
                    .map(|value| value.max(10.0) as i64)
                    .unwrap_or(180);
                let now = chrono::Utc::now();
                let ids =
                    data.jobs
                        .iter()
                        .filter(|job| {
                            job.last_checked_at.is_empty()
                                || chrono::DateTime::parse_from_rfc3339(&job.last_checked_at)
                                    .map_or(true, |checked| {
                                        now.signed_duration_since(checked).num_seconds() >= seconds
                                    })
                        })
                        .map(|job| job.id().to_owned())
                        .collect::<Vec<_>>();
                data.enqueue(&ids, false);
            }
            self.admit();
            self.refresh_stats();
        }
    }
    fn admit(self: &Arc<Self>) {
        loop {
            let ticket = {
                let mut data = self.data.lock();
                if data.closing || data.active_probe_count >= 16 {
                    return;
                }
                let limit = data
                    .settings
                    .platform_max_concurrent_requests
                    .parse::<usize>()
                    .ok()
                    .filter(|limit| *limit > 0)
                    .unwrap_or(3);
                let Some(position) = data.queue.iter().position(|queued| {
                    data.active_per_platform
                        .get(&queued.platform)
                        .copied()
                        .unwrap_or(0)
                        < limit
                }) else {
                    return;
                };
                let queue = data.queue.remove(position).unwrap();
                if !data.current(&queue.id, queue.generation) {
                    continue;
                }
                let Some(job) = data.jobs.iter().find(|job| job.id() == queue.id) else {
                    continue;
                };
                let input = ProbeInput {
                    platform_key: job.platform_key.clone(),
                    live_url: job.input.url.clone(),
                    quality: job.input.quality.clone(),
                    proxy: job_proxy(&data.settings, &job.platform_key),
                    cookies: data
                        .cookies
                        .get(&job.platform_key)
                        .cloned()
                        .filter(|value| !value.is_empty()),
                    username: account(&data.accounts, &job.platform_key, "username"),
                    password: account(&data.accounts, &job.platform_key, "password"),
                    account_type: account(&data.accounts, &job.platform_key, "account_type"),
                };
                let settings = Arc::new(data.settings.clone());
                let credentials_revision = data.credentials_revision;
                let runtime = data.runtime.get_mut(&queue.id).unwrap();
                runtime.queued = false;
                if runtime.checking.is_some() || runtime.recording.is_some() {
                    continue;
                }
                runtime.checking = Some(queue.generation);
                let cancel = runtime.cancel.clone();
                let lifecycle = runtime.lifecycle.clone();
                data.active_probe_count += 1;
                *data
                    .active_per_platform
                    .entry(queue.platform.clone())
                    .or_default() += 1;
                let job = data
                    .jobs
                    .iter_mut()
                    .find(|job| job.id() == queue.id)
                    .unwrap();
                job.status_info = CHECKING.into();
                job.error_message.clear();
                job.last_checked_at = utc_now();
                job.input.updated_at = utc_now();
                let snapshot = data.changed();
                let ticket = ProbeTicket {
                    queue,
                    input,
                    settings,
                    credentials_revision,
                    cancel,
                    lifecycle,
                };
                (ticket, snapshot)
            };
            self.publish(ticket.1);
            let core = self.clone();
            let completing = self.clone();
            let queued = ticket.0.queue.clone();
            let token = ticket.0.cancel.clone();
            let running = self
                .tasks
                .spawn(async move { core.perform_probe(ticket.0).await });
            self.tasks.spawn(async move {
                let result = running.await;
                let error = match result {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(error),
                    Err(error) => Some(format!("平台探测任务异常结束：{error}")),
                };
                completing.complete_probe(&queued, &token, error);
            });
        }
    }
    async fn perform_probe(self: &Arc<Self>, ticket: ProbeTicket) -> Result<(), String> {
        let stream = self
            .probe
            .probe(ticket.input, ticket.cancel.clone())
            .await?;
        if !self
            .data
            .lock()
            .current(&ticket.queue.id, ticket.queue.generation)
        {
            return Ok(());
        }
        if let Some(cookie) = stream.new_cookies.as_deref() {
            self.persist_probe_cookie(&ticket.queue, ticket.credentials_revision, cookie)
                .await?;
        }
        let job = {
            let mut data = self.data.lock();
            if !data.current(&ticket.queue.id, ticket.queue.generation) {
                return Ok(());
            }
            let job = data
                .jobs
                .iter_mut()
                .find(|job| job.id() == ticket.queue.id)
                .unwrap();
            if let Some(name) = stream
                .anchor_name
                .as_deref()
                .filter(|name| !name.trim().is_empty())
            {
                job.input.streamer_name = name.trim().into();
            }
            job.live_title = stream.title.as_deref().unwrap_or("").trim().into();
            job.refresh(false);
            job.status_info.clear();
            job.status_info.push_str(CHECKING);
            job.clone()
        };
        if !stream.is_live {
            let snapshot = {
                let mut data = self.data.lock();
                if !data.current(job.id(), ticket.queue.generation) {
                    return Ok(());
                }
                data.runtime.get_mut(job.id()).unwrap().was_live = false;
                let job = data
                    .jobs
                    .iter_mut()
                    .find(|candidate| candidate.id() == job.id())
                    .unwrap();
                job.status_info = WAITING.into();
                job.record_url.clear();
                data.changed()
            };
            self.publish(snapshot);
            return Ok(());
        }
        if ticket.settings.only_notify_no_record || job.input.only_notify_no_record {
            let (snapshot, notify) = {
                let mut data = self.data.lock();
                if !data.current(job.id(), ticket.queue.generation) {
                    return Ok(());
                }
                let runtime = data.runtime.get_mut(job.id()).unwrap();
                let notify = !runtime.was_live;
                runtime.was_live = true;
                data.jobs
                    .iter_mut()
                    .find(|candidate| candidate.id() == job.id())
                    .unwrap()
                    .status_info = MONITORING.into();
                (data.changed(), notify)
            };
            self.publish(snapshot);
            if notify {
                self.notify(ticket.settings, job, NotificationEvent::Start);
            }
            return Ok(());
        }
        let paths = self.paths.clone();
        let settings = ticket.settings.clone();
        let prepared_job = job.clone();
        let proxy = job_proxy(&settings, &ticket.queue.platform);
        let preparing = tokio::task::spawn_blocking(move || {
            recording::prepare(&paths, &settings, &prepared_job, &stream, proxy.as_deref())
        });
        let mut plan = tokio::select! {_=ticket.cancel.cancelled()=>return Ok(()),result=preparing=>result.map_err(|e|format!("录制准备失败：{e}"))??};
        let _lifecycle = ticket.lifecycle.lock().await;
        {
            let data = self.data.lock();
            if !data.current(job.id(), ticket.queue.generation)
                || data
                    .runtime
                    .get(job.id())
                    .is_some_and(|runtime| runtime.recording.is_some())
            {
                return Ok(());
            }
        }
        let process = ManagedProcess::spawn(&mut plan.command).await?;
        let recording_id = uuid::Uuid::new_v4().simple().to_string();
        let registered = {
            let mut data = self.data.lock();
            if !data.current(job.id(), ticket.queue.generation) {
                None
            } else {
                let runtime = data.runtime.get_mut(job.id()).unwrap();
                let now = Instant::now();
                runtime.was_live = true;
                runtime.recording = Some(ActiveRecording {
                    id: recording_id.clone(),
                    process: process.clone(),
                    output: plan.output.clone(),
                    started: now,
                    last_measure: now,
                    last_bytes: 0,
                });
                let job = data
                    .jobs
                    .iter_mut()
                    .find(|candidate| candidate.id() == job.id())
                    .unwrap();
                job.record_url = plan.url;
                job.latest_output_path = plan.output.to_string_lossy().into_owned();
                job.status_info = RECORDING.into();
                job.recording_started_at = utc_now();
                job.error_message.clear();
                job.duration_text = "00:00:00".into();
                job.speed_text = "0 KB/s".into();
                let job = job.clone();
                Some((data.changed(), job))
            }
        };
        let Some((snapshot, job)) = registered else {
            process.force();
            let _ = tokio::time::timeout(Duration::from_secs(2), process.wait()).await;
            return Ok(());
        };
        self.publish(snapshot);
        let core = self.clone();
        let watched_job = job.clone();
        let settings = ticket.settings.clone();
        let output = plan.output;
        self.tasks.spawn(async move {
            core.watch_recording(watched_job, recording_id, process, output, settings)
                .await;
        });
        self.notify(ticket.settings, job, NotificationEvent::Start);
        Ok(())
    }
    async fn persist_probe_cookie(
        &self,
        queued: &QueuedProbe,
        credentials_revision: u64,
        cookie: &str,
    ) -> Result<(), String> {
        let _transaction = self.transactions.lock().await;
        let mut cookies = {
            let data = self.data.lock();
            if data.closing
                || data.credentials_revision != credentials_revision
                || !data.current(&queued.id, queued.generation)
            {
                return Ok(());
            }
            data.cookies.clone()
        };
        if cookies
            .get(&queued.platform)
            .is_some_and(|saved| saved == cookie)
        {
            return Ok(());
        }
        cookies.insert(queued.platform.clone(), cookie.into());
        let cookies = self.save("cookies.json", cookies).await?;
        let snapshot = {
            let mut data = self.data.lock();
            data.cookies = cookies;
            data.credentials_revision += 1;
            data.changed()
        };
        self.publish(snapshot);
        Ok(())
    }
    fn complete_probe(
        &self,
        queued: &QueuedProbe,
        cancel: &CancellationToken,
        error: Option<String>,
    ) {
        let snapshot = {
            let mut data = self.data.lock();
            data.active_probe_count = data.active_probe_count.saturating_sub(1);
            if let Some(count) = data.active_per_platform.get_mut(&queued.platform) {
                *count = count.saturating_sub(1);
            }
            if let Some(runtime) = data.runtime.get_mut(&queued.id) {
                if runtime.checking == Some(queued.generation) {
                    runtime.checking = None;
                }
            }
            if data.current(&queued.id, queued.generation) && !cancel.is_cancelled() {
                if let Some(error) = error {
                    if let Some(job) = data.jobs.iter_mut().find(|job| job.id() == queued.id) {
                        job.status_info = ERROR.into();
                        job.error_message = error;
                        job.input.updated_at = utc_now();
                    }
                } else if let Some(job) = data.jobs.iter_mut().find(|job| job.id() == queued.id) {
                    if job.status_info == CHECKING {
                        job.status_info = MONITORING.into();
                    }
                }
            }
            data.changed()
        };
        self.publish(snapshot);
        self.wake.notify_one();
    }
    fn notify(
        self: &Arc<Self>,
        settings: Arc<CoreSettings>,
        job: RecordingJob,
        event: NotificationEvent,
    ) {
        if self.cancel.is_cancelled() {
            return;
        }
        let core = self.clone();
        let service = self.notifications.clone();
        let app = self.app.clone();
        let cancel = self.cancel.child_token();
        self.tasks.spawn(async move {
            for error in service.send(settings, job, event, app, cancel).await {
                core.log(&error)
            }
        });
    }
    async fn watch_recording(
        self: &Arc<Self>,
        job: RecordingJob,
        recording_id: String,
        process: Arc<ManagedProcess>,
        output: PathBuf,
        settings: Arc<CoreSettings>,
    ) {
        let result = process.wait().await;
        let (snapshot, finished, event) = {
            let mut data = self.data.lock();
            let matches = data
                .runtime
                .get(job.id())
                .and_then(|runtime| runtime.recording.as_ref())
                .is_some_and(|recording| recording.id == recording_id);
            if !matches {
                return;
            }
            data.runtime.get_mut(job.id()).unwrap().recording = None;
            let closing = data.closing;
            let Some(current) = data
                .jobs
                .iter_mut()
                .find(|candidate| candidate.id() == job.id())
            else {
                return;
            };
            current.recording_started_at.clear();
            current.status_info = if current.input.monitor_status && !closing {
                MONITORING
            } else {
                STOPPED
            }
            .into();
            let error = match &result {
                Ok(result) if result.success => None,
                Ok(result) => Some(if result.error.is_empty() {
                    format!(
                        "录制进程异常结束：{}",
                        if result.stderr.trim().is_empty() {
                            result.code.map_or_else(
                                || "未返回退出码".into(),
                                |code| format!("退出码 {code}"),
                            )
                        } else {
                            result.stderr.trim().into()
                        }
                    )
                } else {
                    result.error.clone()
                }),
                Err(error) => Some(error.clone()),
            };
            let event = if let Some(error) = error {
                if current.input.monitor_status && !closing {
                    current.status_info = ERROR.into();
                    current.error_message = error;
                    NotificationEvent::Error
                } else {
                    NotificationEvent::End
                }
            } else {
                NotificationEvent::End
            };
            let finished = current.clone();
            (data.changed(), finished, event)
        };
        self.publish(snapshot);
        self.wake.notify_one();
        if !self.data.lock().closing {
            self.notify(settings.clone(), finished.clone(), event);
        }
        if result.as_ref().is_ok_and(|result| result.success) && !self.cancel.is_cancelled() {
            let cancel = self.cancel.child_token();
            if let Err(error) =
                recording::post_actions(&settings, &finished, &output, &cancel).await
            {
                if !cancel.is_cancelled() {
                    self.log(&error)
                }
            }
        }
    }
    fn refresh_stats(self: &Arc<Self>) {
        if self.stats_busy.swap(true, Ordering::AcqRel) {
            return;
        }
        let readings = {
            let data = self.data.lock();
            data.runtime
                .iter()
                .filter_map(|(id, runtime)| {
                    runtime.recording.as_ref().map(|recording| {
                        (id.clone(), recording.id.clone(), recording.output.clone())
                    })
                })
                .collect::<Vec<_>>()
        };
        if readings.is_empty() {
            self.stats_busy.store(false, Ordering::Release);
            return;
        }
        let core = self.clone();
        self.tasks.spawn(async move {
            let measured = tokio::task::spawn_blocking(move || {
                readings
                    .into_iter()
                    .map(|(id, recording_id, path)| {
                        (id, recording_id, recording::output_bytes(&path))
                    })
                    .collect::<Vec<_>>()
            })
            .await;
            if let Ok(measured) = measured {
                let snapshot = {
                    let mut data = core.data.lock();
                    let mut changed = false;
                    for (id, recording_id, bytes) in measured {
                        let Some(active) = data
                            .runtime
                            .get_mut(&id)
                            .and_then(|runtime| runtime.recording.as_mut())
                            .filter(|recording| recording.id == recording_id)
                        else {
                            continue;
                        };
                        let now = Instant::now();
                        let speed = bytes.saturating_sub(active.last_bytes) as f64
                            / (now - active.last_measure).as_secs_f64().max(1.0)
                            / 1024.0;
                        let elapsed = (now - active.started).as_secs();
                        active.last_bytes = bytes;
                        active.last_measure = now;
                        let duration = format!(
                            "{:02}:{:02}:{:02}",
                            elapsed / 3600,
                            elapsed / 60 % 60,
                            elapsed % 60
                        );
                        let speed = if speed < 1024.0 {
                            format!("{speed:.1} KB/s")
                        } else {
                            format!("{:.2} MB/s", speed / 1024.0)
                        };
                        if let Some(job) = data.jobs.iter_mut().find(|job| job.id() == id) {
                            if job.duration_text != duration || job.speed_text != speed {
                                job.duration_text = duration;
                                job.speed_text = speed;
                                changed = true;
                            }
                        }
                    }
                    if changed {
                        Some(data.changed())
                    } else {
                        None
                    }
                };
                if let Some(snapshot) = snapshot {
                    core.publish(snapshot)
                }
            }
            core.stats_busy.store(false, Ordering::Release);
        });
    }
    pub fn set_ffmpeg_available(&self, available: bool) {
        let snapshot = {
            let mut data = self.data.lock();
            if data.ffmpeg_available == available {
                return;
            }
            data.ffmpeg_available = available;
            data.changed()
        };
        self.publish(snapshot);
    }
    pub async fn shutdown(self: &Arc<Self>) {
        let beginning = {
            let mut data = self.data.lock();
            if data.closing {
                None
            } else {
                data.closing = true;
                self.cancel.cancel();
                data.queue.clear();
                let mut stops = Vec::new();
                for runtime in data.runtime.values_mut() {
                    runtime.generation += 1;
                    runtime.cancel.cancel();
                    runtime.queued = false;
                    stops.push((
                        runtime.lifecycle.clone(),
                        runtime
                            .recording
                            .as_ref()
                            .map(|recording| recording.process.clone()),
                    ));
                }
                for job in &mut data.jobs {
                    job.status_info = STOPPED.into();
                }
                data.connection.status = "disconnected".into();
                if data.connection.error.is_empty() {
                    data.connection.error = "核心服务已关闭".into();
                }
                Some((data.changed(), stops))
            }
        };
        if let Some((snapshot, stops)) = beginning {
            self.publish(snapshot);
            self.publish_connection();
            self.ready.send_replace(true);
            self.tasks.close();
            let core = self.clone();
            // 退出请求重复或调用方离开时，共享同一个真实回收任务。
            tokio::spawn(async move {
                let processes = stops
                    .iter()
                    .filter_map(|(_, process)| process.clone())
                    .collect::<Vec<_>>();
                let finishing = async {
                    let (result, ()) = tokio::join!(core.stop_collected(stops), core.tasks.wait());
                    result
                };
                tokio::pin!(finishing);
                match tokio::time::timeout(Duration::from_secs(8), &mut finishing).await {
                    Ok(Err(error)) => core.log(&error),
                    Ok(Ok(())) => {}
                    Err(_) => {
                        for process in processes {
                            process.force();
                        }
                        match tokio::time::timeout(Duration::from_secs(2), &mut finishing).await {
                            Ok(Err(error)) => core.log(&error),
                            Err(_) => core.log("部分后台任务未按时结束"),
                            _ => {}
                        }
                    }
                }
                core.stopped.send_replace(true);
            });
        }
        let mut stopped = self.stopped.subscribe();
        if !*stopped.borrow_and_update() {
            let waiting = async {
                while stopped.changed().await.is_ok() {
                    if *stopped.borrow_and_update() {
                        break;
                    }
                }
            };
            if tokio::time::timeout(Duration::from_secs(10), waiting)
                .await
                .is_err()
            {
                self.log("等待核心退出超时");
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn active_processes(&self) -> Vec<Arc<ManagedProcess>> {
        self.data
            .lock()
            .runtime
            .values()
            .filter_map(|runtime| {
                runtime
                    .recording
                    .as_ref()
                    .map(|recording| recording.process.clone())
            })
            .collect()
    }
    #[cfg(test)]
    pub(crate) async fn wait_idle(&self) {
        while !self.tasks.is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}
fn valid_url(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https" | "rtmp" | "rtmps") && url.host_str().is_some()
    })
}
fn account(accounts: &BTreeMap<String, Value>, platform: &str, field: &str) -> Option<String> {
    let value = accounts
        .get(platform)
        .and_then(|account| account.get(field))
        .or_else(|| accounts.get(&format!("{platform}_{field}")))?;
    if value.is_null() {
        return None;
    }
    let text = super::probe::text(value);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}
fn job_proxy(settings: &CoreSettings, platform: &str) -> Option<String> {
    let proxy = super::notifications::proxy(settings)?;
    let platforms = settings
        .default_platform_with_proxy
        .split([',', '，'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if platforms.is_empty()
        || platforms
            .iter()
            .any(|key| key.eq_ignore_ascii_case(platform))
    {
        Some(proxy)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
