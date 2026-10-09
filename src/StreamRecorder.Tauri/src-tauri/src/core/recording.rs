use super::{
    models::{CoreSettings, RecordingJob},
    probe::StreamData,
    process::ManagedProcess,
};
use crate::paths::ProjectPaths;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

pub struct RecordingPlan {
    pub output: PathBuf,
    pub url: String,
    pub command: Command,
}
fn nonempty<'a>(value: Option<&'a str>, fallback: &'a str) -> &'a str {
    value.filter(|s| !s.is_empty()).unwrap_or(fallback)
}
pub fn sanitize(text: &str) -> String {
    let mut result = String::new();
    let mut underscore = false;
    for character in text.trim().chars() {
        let character = if "\\/:*?\"<>|，。 ".contains(character) {
            '_'
        } else {
            character
        };
        if character == '_' {
            if !underscore {
                result.push(character)
            }
            underscore = true;
        } else {
            result.push(character);
            underscore = false;
        }
    }
    let result: String = result.trim_matches('_').chars().take(60).collect();
    if result.is_empty() {
        "直播间".into()
    } else {
        result
    }
}
fn anchor<'a>(job: &'a RecordingJob, stream: &'a StreamData) -> &'a str {
    nonempty(
        stream.anchor_name.as_deref(),
        nonempty(Some(&job.input.streamer_name), "直播间"),
    )
}
fn platform<'a>(job: &'a RecordingJob, stream: &'a StreamData) -> &'a str {
    nonempty(
        stream.platform.as_deref(),
        nonempty(Some(&job.platform), "直播"),
    )
}
fn normalized(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}
fn legacy_directory(
    job: &RecordingJob,
    stream: &StreamData,
    path: &Path,
    base: &Path,
    settings: &CoreSettings,
) -> bool {
    if !settings.folder_name_platform
        || !settings.folder_name_author
        || settings.folder_name_time
        || settings.folder_name_title
    {
        return false;
    }
    let Ok(relative) = path.strip_prefix(base) else {
        return false;
    };
    let parts = relative.components().collect::<Vec<_>>();
    if parts.len() != 2
        || parts[1].as_os_str() != std::ffi::OsStr::new(&sanitize(anchor(job, stream)))
    {
        return false;
    }
    let name = parts[0].as_os_str().to_string_lossy();
    let mut names = vec![
        platform(job, stream).to_owned(),
        job.platform.clone(),
        job.platform_key.clone(),
    ];
    let mut title = job.platform_key.clone();
    if let Some(first) = title.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    names.push(title);
    // 旧花猫委托解析器曾把自动目录写成飘飘，仍按历史自动目录处理。
    if job.platform_key == "catshow" {
        names.push("飘飘直播".into());
    }
    names.iter().any(|candidate| {
        name == sanitize(candidate)
            || candidate
                .strip_suffix("直播")
                .is_some_and(|short| name == sanitize(short))
    })
}
pub fn output_directory(
    paths: &ProjectPaths,
    settings: &CoreSettings,
    job: &RecordingJob,
    stream: &StreamData,
) -> PathBuf {
    let base = if settings.live_save_path.trim().is_empty() {
        paths.data_root.join("recordings")
    } else {
        paths.resolve(settings.live_save_path.trim())
    };
    if !job.input.recording_dir.is_empty() {
        let directory = paths.resolve(&job.input.recording_dir);
        let layout = settings.folder_name_platform
            || settings.folder_name_author
            || settings.folder_name_time
            || settings.folder_name_title;
        let normalized_base = normalized(&base);
        let normalized_directory = normalized(&directory);
        if !layout
            || (normalized_directory != normalized_base
                && !legacy_directory(
                    job,
                    stream,
                    &normalized_directory,
                    &normalized_base,
                    settings,
                ))
        {
            return directory;
        }
    }
    let mut directory = base;
    if settings.folder_name_platform {
        directory.push(sanitize(platform(job, stream)));
    }
    if settings.folder_name_author {
        directory.push(sanitize(anchor(job, stream)));
    }
    if settings.folder_name_time {
        directory.push(chrono::Local::now().format("%Y-%m-%d").to_string());
    }
    if settings.folder_name_title {
        if let Some(title) = stream.title.as_deref().filter(|s| !s.is_empty()) {
            directory.push(sanitize(title));
        }
    }
    directory
}
pub fn base_name(settings: &CoreSettings, job: &RecordingJob, stream: &StreamData) -> String {
    let time = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let anchor = sanitize(anchor(job, stream));
    let title = if settings.filename_includes_title {
        sanitize(stream.title.as_deref().unwrap_or(""))
    } else {
        String::new()
    };
    let template = nonempty(
        Some(&settings.custom_filename_template),
        "{anchor_name}_{title}_{time}",
    );
    let expanded = template
        .replace("{anchor_name}", &anchor)
        .replace("{title}", &title)
        .replace("{time}", &time)
        .replace("{platform}", &sanitize(platform(job, stream)));
    let result = expanded
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if result.is_empty() {
        format!("{anchor}_{time}")
    } else {
        result
    }
}
pub fn select_url(
    settings: &CoreSettings,
    job: &RecordingJob,
    stream: &StreamData,
) -> Result<String, String> {
    let hls = stream.m3u8_url.as_deref().unwrap_or("");
    let flv = stream.flv_url.as_deref().unwrap_or("");
    let record = stream.record_url.as_deref().unwrap_or("");
    let order = if settings.default_live_source.eq_ignore_ascii_case("HLS") {
        [hls, record, flv]
    } else {
        [flv, record, hls]
    };
    let mut url = order
        .into_iter()
        .find(|url| !url.is_empty())
        .ok_or("没有可用的直播流地址")?
        .to_owned();
    if settings.force_https_recording {
        if let Some(tail) = url.strip_prefix("http://") {
            url = format!("https://{tail}");
        }
    }
    if matches!(job.platform_key.as_str(), "shopee" | "migu") {
        url = url.replace("https://", "http://");
    }
    Ok(url)
}
pub fn ffmpeg_command(
    binary: &Path,
    url: &str,
    output: &Path,
    format: &str,
    segment: bool,
    segment_time: &str,
    proxy: Option<&str>,
) -> Result<Command, String> {
    let format = format.to_ascii_lowercase();
    let muxer = match format.as_str() {
        "ts" | "mpegts" => "mpegts",
        "flv" => "flv",
        "mp4" => "mp4",
        "mkv" => "matroska",
        "mov" => "mov",
        "mp3" => "mp3",
        "m4a" => "ipod",
        "wav" => "wav",
        "aac" => "adts",
        "wma" => "asf",
        _ => return Err(format!("不支持的录制格式：{format}")),
    };
    let mut command = Command::new(binary);
    command.arg("-y");
    if let Some(proxy) = proxy {
        command.args(["-http_proxy", proxy]);
    }
    command.args(["-v","error","-hide_banner","-user_agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36","-rw_timeout","15000000","-protocol_whitelist","rtmp,crypto,file,http,https,tcp,tls,udp,rtp,httpproxy","-thread_queue_size","1024","-analyzeduration","20000000","-probesize","10000000","-fflags","+discardcorrupt+igndts"]);
    if url.to_ascii_lowercase().starts_with("http://")
        || url.to_ascii_lowercase().starts_with("https://")
    {
        // 重试传输/连接故障与临时 HTTP 错误；正常 EOF 不重连，避免重播 HLS 分片。
        command.args([
            "-reconnect",
            "1",
            "-reconnect_streamed",
            "1",
            "-reconnect_on_network_error",
            "1",
            "-reconnect_on_http_error",
            "408,429,5xx",
            "-reconnect_delay_max",
            "15",
        ]);
    }
    command.args(["-i", url, "-sn", "-dn", "-map", "0"]);
    match format.as_str() {
        "mp3" => {
            command.args(["-vn", "-c:a", "libmp3lame"]);
        }
        "m4a" | "aac" => {
            command.args(["-vn", "-c:a", "aac"]);
        }
        "wav" => {
            command.args(["-vn", "-c:a", "pcm_s16le"]);
        }
        "wma" => {
            command.args(["-vn", "-c:a", "wmav2"]);
        }
        _ => {
            command.args(["-c:v", "copy", "-c:a", "copy"]);
            if format == "flv" {
                command.args(["-bsf:a", "aac_adtstoasc"]);
            }
        }
    }
    if segment {
        command.args([
            "-f",
            "segment",
            "-segment_time",
            segment_time,
            "-segment_format",
            muxer,
            "-reset_timestamps",
            "1",
        ]);
    } else {
        command.args(["-f", muxer]);
    }
    if muxer == "mpegts" {
        command.args([
            "-mpegts_flags",
            "+resend_headers",
            "-muxdelay",
            "0",
            "-muxpreload",
            "0",
        ]);
    }
    if muxer == "mp4" {
        // 提前写空 moov 会绕过 AAC 自动码流转换；保留关键帧分片，不强制音频为 AAC。
        command.args(["-movflags", "+faststart+frag_keyframe"]);
        if segment {
            command.args(["-flags", "global_header"]);
        }
    }
    command.arg(output);
    Ok(command)
}
pub fn find_ffmpeg() -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .flat_map(|directory| [directory.join("ffmpeg.exe"), directory.join("ffmpeg")])
        .find(|path| path.is_file())
}
pub fn prepare(
    paths: &ProjectPaths,
    settings: &CoreSettings,
    job: &RecordingJob,
    stream: &StreamData,
    proxy: Option<&str>,
) -> Result<RecordingPlan, String> {
    let binary = find_ffmpeg().ok_or("未检测到 ffmpeg，无法开始录制")?;
    let directory = output_directory(paths, settings, job, stream);
    fs::create_dir_all(&directory).map_err(|e| format!("无法创建录制目录：{e}"))?;
    let threshold = settings
        .recording_space_threshold
        .parse::<f64>()
        .unwrap_or(2.0);
    let wide: Vec<u16> = directory
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut available = 0u64;
    if unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(format!(
            "无法检查录制磁盘空间：{}",
            std::io::Error::last_os_error()
        ));
    }
    if (available as f64) / (1024f64.powi(3)) < threshold || threshold.is_nan() {
        return Err("存储空间不足，已跳过录制".into());
    }
    let format = nonempty(
        Some(&job.input.record_format),
        nonempty(Some(&settings.video_format), "TS"),
    )
    .to_ascii_lowercase();
    let suffix = if job.input.segment_record {
        format!("_%03d.{format}")
    } else {
        format!(".{format}")
    };
    let output = directory.join(format!("{}{suffix}", base_name(settings, job, stream)));
    let url = select_url(settings, job, stream)?;
    let command = ffmpeg_command(
        &binary,
        &url,
        &output,
        &format,
        job.input.segment_record,
        &job.input.segment_time,
        proxy,
    )?;
    Ok(RecordingPlan {
        output,
        url,
        command,
    })
}
pub fn outputs(pattern: &Path) -> Vec<PathBuf> {
    let Some(name) = pattern.file_name().and_then(|name| name.to_str()) else {
        return Vec::new();
    };
    let Some((prefix, suffix)) = name.split_once("%03d") else {
        return if pattern.is_file() {
            vec![pattern.to_owned()]
        } else {
            Vec::new()
        };
    };
    let Some(parent) = pattern.parent() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            (entry.file_name().to_str().is_some_and(|name| {
                name.starts_with(prefix) && name.ends_with(suffix)
            }) && path.is_file()).then_some(path)
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}
pub fn output_bytes(pattern: &Path) -> u64 {
    outputs(pattern)
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .map(|metadata| metadata.len())
        .sum()
}
async fn wait_cancellable(
    process: Arc<ManagedProcess>,
    cancel: &CancellationToken,
) -> Result<super::process::ProcessExit, String> {
    tokio::select! {result=process.wait()=>result,_=cancel.cancelled()=>{process.force();let _=tokio::time::timeout(std::time::Duration::from_secs(2),process.wait()).await;Err("后处理已取消".into())}}
}
pub async fn post_actions(
    settings: &CoreSettings,
    job: &RecordingJob,
    output: &Path,
    cancel: &CancellationToken,
) -> Result<(), String> {
    if cancel.is_cancelled() {
        return Err("后处理已取消".into());
    }
    if settings.convert_to_mp4
        && output
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("ts"))
    {
        let path = output.to_owned();
        let sources = tokio::task::spawn_blocking(move || outputs(&path))
            .await
            .map_err(|e| e.to_string())?;
        let binary = find_ffmpeg().ok_or("未检测到 ffmpeg，无法转封装")?;
        for source in sources {
            if cancel.is_cancelled() {
                return Err("后处理已取消".into());
            }
            let target = source.with_extension("mp4");
            let mut command = Command::new(&binary);
            command
                .arg("-y")
                .arg("-i")
                .arg(&source)
                .args(["-c", "copy"])
                .arg(&target);
            let process = ManagedProcess::spawn(&mut command).await?;
            let result = wait_cancellable(process, cancel).await?;
            if !result.success {
                return Err(format!("TS 转 MP4 失败：{}", result.stderr));
            }
            let verified = target.clone();
            tokio::task::spawn_blocking(move || fs::File::open(verified))
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| format!("转封装文件不可读：{e}"))?;
            if settings.delete_original {
                tokio::task::spawn_blocking(move || fs::remove_file(source))
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e| format!("无法删除已转换的 TS：{e}"))?;
            }
        }
    }
    if settings.execute_custom_script && !settings.custom_script_command.trim().is_empty() {
        if cancel.is_cancelled() {
            return Err("后处理已取消".into());
        }
        let mut command = Command::new("cmd.exe");
        command.args(["/D", "/S", "/C", &settings.custom_script_command]);
        command
            .env("STREAMRECORDER_JOB_ID", job.id())
            .env("STREAMRECORDER_STREAMER_NAME", &job.input.streamer_name)
            .env(
                "STREAMRECORDER_TITLE",
                if job.live_title.is_empty() {
                    &job.title
                } else {
                    &job.live_title
                },
            )
            .env("STREAMRECORDER_OUTPUT", output)
            .env("STREAMRECORDER_URL", &job.input.url);
        let result = wait_cancellable(ManagedProcess::spawn(&mut command).await?, cancel).await?;
        if !result.success {
            return Err(format!("自定义脚本失败：{}", result.stderr));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
