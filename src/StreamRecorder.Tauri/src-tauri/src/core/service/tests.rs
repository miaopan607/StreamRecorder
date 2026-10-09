use super::*;
use std::fs;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::mpsc,
};

async fn core(root: &PathBuf) -> Arc<CoreService> {
    let core = CoreService::new(ProjectPaths::at(root.clone()), None);
    core.start().await.unwrap();
    core
}
#[tokio::test]
async fn upsert_is_atomic_and_empty_delete_preserves_jobs() {
    let directory = tempfile::tempdir().unwrap();
    let core = core(&directory.path().to_owned()).await;
    let original = core
        .jobs_upsert(vec![JobInput {
            url: "https://example.org/live.m3u8".into(),
            monitor_status: false,
            streamer_name: "原主播".into(),
            ..JobInput::default()
        }])
        .await
        .unwrap()
        .changed_ids[0]
        .clone();
    let update = core
        .jobs_upsert(vec![JobInput {
            url: "HTTPS://EXAMPLE.ORG/live.m3u8".into(),
            monitor_status: false,
            streamer_name: "更新主播".into(),
            ..JobInput::default()
        }])
        .await
        .unwrap();
    assert_eq!(update.changed_ids, vec![original.clone()]);
    assert_eq!(core.snapshot().jobs.len(), 1);
    assert_eq!(core.job(&original).unwrap().input.streamer_name, "更新主播");
    let stored = fs::read(core.paths.data_root.join("jobs.json")).unwrap();
    let result = core
        .jobs_upsert(vec![
            JobInput {
                url: "https://example.org/second.m3u8".into(),
                monitor_status: false,
                ..JobInput::default()
            },
            JobInput {
                url: "not a URL".into(),
                ..JobInput::default()
            },
        ])
        .await;
    assert!(result.is_err());
    assert_eq!(
        fs::read(core.paths.data_root.join("jobs.json")).unwrap(),
        stored
    );
    assert_eq!(core.snapshot().jobs.len(), 1);
    assert_eq!(core.jobs_delete(Vec::new()).await.unwrap().deleted, 0);
    assert!(core.job(&original).is_some());
    core.shutdown().await;
}
#[tokio::test]
async fn disk_failure_does_not_commit_settings_or_revision() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = tempfile::tempdir().unwrap();
    let core = core(&directory.path().to_owned()).await;
    let path = core.paths.data_root.join("core_settings.json");
    let before = fs::read(&path).unwrap();
    let snapshot = core.snapshot();
    let _locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ)
        .open(&path)
        .unwrap();
    let mut settings = snapshot.settings.clone();
    settings.custom_filename_template = "不得提交的模板".into();
    assert!(core.settings_update(settings).await.is_err());
    assert_eq!(core.snapshot().revision, snapshot.revision);
    assert_eq!(
        core.snapshot().settings.custom_filename_template,
        snapshot.settings.custom_filename_template
    );
    assert_eq!(fs::read(path).unwrap(), before);
    core.shutdown().await;
}
#[tokio::test]
async fn failed_start_finishes_ready_watch() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("runtime"), b"not a directory").unwrap();
    let core = CoreService::new(ProjectPaths::at(directory.path().to_owned()), None);
    assert!(core.start().await.is_err());
    assert!(
        tokio::time::timeout(Duration::from_secs(1), core.wait_ready())
            .await
            .unwrap()
            .is_err()
    );
    assert_eq!(core.connection().status, "disconnected");
}
struct Network {
    endpoint: String,
    requests: mpsc::UnboundedReceiver<String>,
    release: Arc<Notify>,
    cancel: CancellationToken,
    server: tokio::task::JoinHandle<()>,
}
impl Network {
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (send, requests) = mpsc::unbounded_channel();
        let release = Arc::new(Notify::new());
        let cancel = CancellationToken::new();
        let active = cancel.clone();
        let wake = release.clone();
        let corpus: Value =
            serde_json::from_str(include_str!("../platforms/fixtures/platforms.json")).unwrap();
        let fast = corpus
            .as_array()
            .unwrap()
            .iter()
            .find(|fixture| fixture["platform"] == "bigo")
            .unwrap()["offline"][0]["body"]
            .as_str()
            .unwrap()
            .to_owned();
        let server = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    _=active.cancelled()=>break,
                    accepted=listener.accept()=>{let (stream,_)=accepted.unwrap();let send=send.clone();let release=wake.clone();let cancel=active.clone();let fast=fast.clone();
                        connections.spawn(async move{let mut reader=BufReader::new(stream);let mut line=String::new();reader.read_line(&mut line).await.unwrap();let target=line.split_whitespace().nth(1).unwrap().to_owned();let mut length=0;
                            loop{line.clear();reader.read_line(&mut line).await.unwrap();if line=="\r\n"{break}if let Some((key,value))=line.split_once(':'){if key.eq_ignore_ascii_case("content-length"){length=value.trim().parse().unwrap();}}}
                            let mut body=vec![0;length];reader.read_exact(&mut body).await.unwrap();send.send(target.clone()).unwrap();
                            let response=if target.contains("/langweb/"){tokio::select!{_=cancel.cancelled()=>return,_=release.notified()=>{}};"{\"data\":{\"live_info\":{\"nickname\":\"过期主播\",\"live_status\":1,\"liveurl\":\"https://media.example/stale.flv\",\"liveurl_hls\":\"https://media.example/stale.m3u8\"}}}"}else{fast.as_str()};
                            let mut stream=reader.into_inner();let header=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",response.len());let _=stream.write_all(header.as_bytes()).await;let _=stream.write_all(response.as_bytes()).await;
                        });
                    }
                }
            }
            connections.shutdown().await;
        });
        Self {
            endpoint,
            requests,
            release,
            cancel,
            server,
        }
    }
    async fn request(&mut self) -> String {
        tokio::time::timeout(Duration::from_secs(1), self.requests.recv())
            .await
            .unwrap()
            .unwrap()
    }
    async fn close(self) {
        self.cancel.cancel();
        self.server.await.unwrap();
    }
}
#[tokio::test]
async fn slow_requests_do_not_block_commands_or_bypass_reduced_limit() {
    let directory = tempfile::tempdir().unwrap();
    let core = core(&directory.path().to_owned()).await;
    let mut network = Network::new().await;
    *core.probe.endpoint.lock() = Some(network.endpoint.clone());
    let mut settings = core.snapshot().settings;
    settings.only_notify_no_record = true;
    settings.platform_max_concurrent_requests = "3".into();
    core.settings_update(settings.clone()).await.unwrap();
    let jobs = (10001..10005)
        .map(|id| JobInput {
            url: format!("https://www.lang.live/{id}"),
            ..JobInput::default()
        })
        .collect();
    let ids = core.jobs_upsert(jobs).await.unwrap().changed_ids;
    for _ in 0..3 {
        assert!(network.request().await.contains("/langweb/"));
    }
    let began = Instant::now();
    settings.platform_max_concurrent_requests = "1".into();
    core.settings_update(settings).await.unwrap();
    core.health_ping().unwrap();
    assert!(began.elapsed() < Duration::from_secs(1));
    let normal = core
        .jobs_upsert(vec![JobInput {
            url: "https://www.bigo.tv/10001".into(),
            ..JobInput::default()
        }])
        .await
        .unwrap()
        .changed_ids[0]
        .clone();
    assert!(network.request().await.contains("getInternalStudioInfo"));
    tokio::time::timeout(Duration::from_secs(1), async {
        while core.job(&normal).unwrap().status_info != WAITING {
            tokio::time::sleep(Duration::from_millis(5)).await
        }
    })
    .await
    .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), network.requests.recv())
            .await
            .is_err()
    );
    for id in &ids[..2] {
        core.jobs_stop_monitoring(vec![id.clone()]).await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(100), network.requests.recv())
                .await
                .is_err()
        );
    }
    core.jobs_delete(vec![ids[2].clone()]).await.unwrap();
    assert!(network.request().await.contains("/langweb/"));
    core.jobs_stop_monitoring(vec![ids[3].clone()])
        .await
        .unwrap();
    network.release.notify_waiters();
    tokio::time::timeout(Duration::from_secs(1), async {
        while core.data.lock().active_probe_count != 0 {
            tokio::time::sleep(Duration::from_millis(5)).await
        }
    })
    .await
    .unwrap();
    assert!(core.job(&ids[2]).is_none());
    for id in [ids[0].clone(), ids[1].clone(), ids[3].clone()] {
        let job = core.job(&id).unwrap();
        assert_eq!(job.status_info, STOPPED);
        assert_ne!(job.input.streamer_name, "过期主播");
        assert!(job.record_url.is_empty());
        assert!(core.data.lock().runtime[&id].checking.is_none());
    }
    core.shutdown().await;
    network.close().await;
}

#[tokio::test]
async fn repeated_shutdown_waits_for_the_same_batch_reclamation() {
    let directory = tempfile::tempdir().unwrap();
    let core = core(&directory.path().to_owned()).await;
    let mut processes = Vec::new();
    for index in 0..3 {
        let id = core
            .jobs_upsert(vec![JobInput {
                url: format!("https://example.org/{index}.flv"),
                monitor_status: false,
                ..JobInput::default()
            }])
            .await
            .unwrap()
            .changed_ids[0]
            .clone();
        let mut command = tokio::process::Command::new("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 60",
        ]);
        let process = ManagedProcess::spawn(&mut command).await.unwrap();
        {
            let mut data = core.data.lock();
            let now = Instant::now();
            data.runtime.get_mut(&id).unwrap().recording = Some(ActiveRecording {
                id: format!("controlled-{index}"),
                process: process.clone(),
                output: directory.path().join(format!("{index}.ts")),
                started: now,
                last_measure: now,
                last_bytes: 0,
            });
        }
        processes.push(process);
    }
    let began = Instant::now();
    tokio::join!(core.shutdown(), core.shutdown());
    assert!(began.elapsed() < Duration::from_secs(10));
    for process in processes {
        tokio::time::timeout(Duration::from_millis(20), process.wait())
            .await
            .unwrap()
            .unwrap();
    }
}
