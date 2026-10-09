use super::*;
use crate::core::{
    models::{JobInput, ERROR, RECORDING},
    CoreService,
};
use parking_lot::Mutex;
use serde_json::Value;
use std::{collections::HashMap, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time::Instant,
};

struct MediaServer {
    url: String,
    counts: Arc<Mutex<HashMap<String, usize>>>,
    cancel: CancellationToken,
    task: tokio::task::JoinHandle<()>,
}
impl MediaServer {
    async fn new(binary: PathBuf) -> Self {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../artifacts/tauri-smoke-media");
        let mut files = HashMap::new();
        for entry in fs::read_dir(root).unwrap().filter_map(Result::ok) {
            if entry.path().is_file() {
                files.insert(
                    format!("/{}", entry.file_name().to_string_lossy()),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
        let files = Arc::new(files);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let cancel = CancellationToken::new();
        let active = cancel.clone();
        let counts = Arc::new(Mutex::new(HashMap::new()));
        let seen = counts.clone();
        let task = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    _=active.cancelled()=>break,
                    accepted=listener.accept()=>{let (stream,_)=accepted.unwrap();let files=files.clone();let counts=seen.clone();let cancel=active.clone();let binary=binary.clone();
                        connections.spawn(async move{let mut reader=BufReader::new(stream);let mut line=String::new();if reader.read_line(&mut line).await.unwrap_or(0)==0{return}let path=line.split_whitespace().nth(1).unwrap().split('?').next().unwrap().to_owned();let mut length=0usize;
                            loop{line.clear();if reader.read_line(&mut line).await.unwrap_or(0)==0{return}if line=="\r\n"{break}if let Some((key,value))=line.split_once(':'){if key.eq_ignore_ascii_case("content-length"){length=value.trim().parse().unwrap();}}}
                            let mut request=vec![0;length];if reader.read_exact(&mut request).await.is_err(){return}*counts.lock().entry(path.clone()).or_default()+=1;let mut stream=reader.into_inner();
                            if path=="/continuous.flv"{continuous(&mut stream,&binary,cancel).await;return}
                            if path.starts_with("/langweb/")||path=="/bad-hook"{cancel.cancelled().await;return}
                            let Some(body)=files.get(&path)else{let _=stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;return};
                            let header=format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());let _=stream.write_all(header.as_bytes()).await;let _=stream.write_all(body).await;
                        });
                    }
                }
            }
            while connections.join_next().await.is_some() {}
        });
        Self {
            url,
            counts,
            cancel,
            task,
        }
    }
    fn count(&self, path: &str) -> usize {
        self.counts.lock().get(path).copied().unwrap_or(0)
    }
    async fn close(self) {
        self.cancel.cancel();
        self.task.await.unwrap();
    }
}
async fn continuous(stream: &mut TcpStream, binary: &Path, cancel: CancellationToken) {
    use std::process::Stdio;
    let mut command = Command::new(binary);
    command
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-re",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x180:rate=25",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-g",
            "25",
            "-c:a",
            "aac",
            "-f",
            "flv",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .creation_flags(0x08000000);
    let mut producer = command.spawn().unwrap();
    let mut output = producer.stdout.take().unwrap();
    let _ = stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: video/x-flv\r\nConnection: close\r\n\r\n")
        .await;
    let mut bytes = [0; 8192];
    loop {
        let count = tokio::select! {_=cancel.cancelled()=>break,result=output.read(&mut bytes)=>match result{Ok(0)|Err(_)=>break,Ok(count)=>count}};
        if stream.write_all(&bytes[..count]).await.is_err() {
            break;
        }
    }
    let _ = producer.kill().await;
    let _ = producer.wait().await;
}
async fn inspect(binary: &Path, path: &Path) -> Value {
    let ffprobe = binary.parent().unwrap().join("ffprobe.exe");
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
        ])
        .arg(path)
        .kill_on_drop(true)
        .creation_flags(0x08000000)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}：{}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
async fn completed(core: &Arc<CoreService>, id: &str) -> PathBuf {
    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let job = core.job(id).unwrap();
            assert_ne!(job.status_info, ERROR, "{}", job.error_message);
            if !job.latest_output_path.is_empty() && job.recording_started_at.is_empty() {
                core.wait_idle().await;
                return PathBuf::from(job.latest_output_path);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
#[ignore = "需要真实 ffmpeg/ffprobe；默认套件不启动媒体工具"]
async fn ffmpeg_smoke() {
    let binary = find_ffmpeg().expect("请将验收 ffmpeg 目录加入 PATH");
    let server = MediaServer::new(binary.clone()).await;
    let directory = tempfile::Builder::new()
        .prefix("中文 空格 Rust 核心验收 ")
        .tempdir()
        .unwrap();
    let core = CoreService::new(ProjectPaths::at(directory.path().to_owned()), None);
    core.start().await.unwrap();
    let mut settings = core.snapshot().settings;
    settings.force_https_recording = false;
    settings.recording_space_threshold = "0".into();
    settings.custom_filename_template = "原生_录制_{time}".into();
    settings.system_notification_enabled = false;
    settings.loop_time_seconds = "180".into();
    core.settings_update(settings.clone()).await.unwrap();
    for (format, muxer) in [
        ("TS", "mpegts"),
        ("FLV", "flv"),
        ("MKV", "matroska"),
        ("MOV", "mov"),
        ("MP4", "mp4"),
        ("MP3", "mp3"),
        ("M4A", "m4a"),
    ] {
        let input = JobInput {
            url: format!("{}/live.m3u8?format={format}", server.url),
            record_format: format.into(),
            recording_dir: format!("录制 输出/{format}"),
            ..JobInput::default()
        };
        let first = server.count("/live0.ts");
        let id = core
            .jobs_upsert(vec![input.clone()])
            .await
            .unwrap()
            .changed_ids[0]
            .clone();
        let output = completed(&core, &id).await;
        let media = inspect(&binary, &output).await;
        let streams = media["streams"].as_array().unwrap();
        assert!(streams.iter().any(|stream| stream["codec_type"] == "audio"));
        if matches!(format, "MP3" | "M4A") {
            assert!(streams.iter().all(|stream| stream["codec_type"] != "video"));
        } else {
            assert!(streams.iter().any(|stream| stream["codec_type"] == "video"));
        }
        assert!(media["format"]["format_name"]
            .as_str()
            .unwrap()
            .contains(muxer));
        assert_eq!(
            core.job(&id).unwrap().input.recording_dir,
            input.recording_dir
        );
        assert_eq!(
            server.count("/live0.ts") - first,
            1,
            "同一次有限 HLS 录制重复读取首分片"
        );
        println!(
            "原生 {format}：{}，音视频/音频流可读",
            media["format"]["format_name"]
        );
        core.jobs_stop_monitoring(vec![id]).await.unwrap();
    }
    let id = core
        .jobs_upsert(vec![JobInput {
            url: format!("{}/live.m3u8?segment=1", server.url),
            record_format: "TS".into(),
            segment_record: true,
            segment_time: "1".into(),
            recording_dir: "录制 输出/分段".into(),
            ..JobInput::default()
        }])
        .await
        .unwrap()
        .changed_ids[0]
        .clone();
    let pattern = completed(&core, &id).await;
    let segments = outputs(&pattern);
    assert!(segments.len() >= 2, "分段没有产生多个完整片段");
    for segment in &segments {
        let media = inspect(&binary, segment).await;
        assert!(media["streams"]
            .as_array()
            .unwrap()
            .iter()
            .any(|stream| stream["codec_type"] == "video"));
    }
    println!("原生 TS 分段：{} 个可播放片段", segments.len());
    core.jobs_stop_monitoring(vec![id]).await.unwrap();
    settings.convert_to_mp4 = true;
    settings.delete_original = true;
    core.settings_update(settings.clone()).await.unwrap();
    let id = core
        .jobs_upsert(vec![JobInput {
            url: format!("{}/live.m3u8?convert=1", server.url),
            record_format: "TS".into(),
            recording_dir: "录制 输出/转封装".into(),
            ..JobInput::default()
        }])
        .await
        .unwrap()
        .changed_ids[0]
        .clone();
    let original = completed(&core, &id).await;
    assert!(!original.exists());
    let converted = original.with_extension("mp4");
    let media = inspect(&binary, &converted).await;
    assert!(media["format"]["format_name"]
        .as_str()
        .unwrap()
        .contains("mp4"));
    println!("原生 TS 转 MP4：文件可读，成功后删除原 TS");
    core.jobs_stop_monitoring(vec![id]).await.unwrap();
    settings.convert_to_mp4 = false;
    settings.bark_enabled = true;
    settings.bark_webhook_url = format!("{}/bad-hook", server.url);
    settings.stream_start_notification_enabled = true;
    core.settings_update(settings.clone()).await.unwrap();
    *core.probe.endpoint.lock() = Some(server.url.clone());
    let slow = core
        .jobs_upsert(vec![JobInput {
            url: "https://www.lang.live/10001".into(),
            ..JobInput::default()
        }])
        .await
        .unwrap()
        .changed_ids[0]
        .clone();
    let live = core
        .jobs_upsert(vec![JobInput {
            url: format!("{}/continuous.flv", server.url),
            record_format: "FLV".into(),
            recording_dir: "录制 输出/持续直播".into(),
            ..JobInput::default()
        }])
        .await
        .unwrap()
        .changed_ids[0]
        .clone();
    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let job = core.job(&live).unwrap();
            assert_ne!(job.status_info, ERROR, "{}", job.error_message);
            if job.status_info == RECORDING
                && job.duration_text != "00:00:00"
                && output_bytes(Path::new(&job.latest_output_path)) > 0
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    let processes = core.active_processes();
    assert_eq!(processes.len(), 1);
    assert!(server.count("/bad-hook") > 0);
    assert_eq!(
        core.job(&slow).unwrap().status_info,
        crate::core::models::CHECKING
    );
    let began = Instant::now();
    settings.custom_filename_template = "保存不等待探测或通知".into();
    core.settings_update(settings).await.unwrap();
    core.health_ping().unwrap();
    assert!(began.elapsed() < Duration::from_secs(1));
    let began = Instant::now();
    core.jobs_stop_monitoring(vec![live.clone()]).await.unwrap();
    assert!(began.elapsed() < Duration::from_secs(10));
    let output = PathBuf::from(core.job(&live).unwrap().latest_output_path);
    let media = inspect(&binary, &output).await;
    assert!(media["streams"]
        .as_array()
        .unwrap()
        .iter()
        .any(|stream| stream["codec_type"] == "video"));
    for process in processes {
        process.wait().await.unwrap();
    }
    println!("原生持续 FLV：状态/时长/速度更新，慢平台和坏 webhook 下保存、ping、停止成功");
    core.shutdown().await;
    server.close().await;
}
