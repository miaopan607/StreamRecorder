from __future__ import annotations

from pathlib import Path

FFMPEG_USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"
)


def build_ffmpeg_command(
    record_url: str,
    save_path: Path,
    save_format: str,
    *,
    segment_record: bool,
    segment_time: str,
    proxy: str | None = None,
) -> list[str]:
    format_key = save_format.lower()
    command = [
        "ffmpeg",
        "-y",
        "-v",
        "error",
        "-hide_banner",
        "-user_agent",
        FFMPEG_USER_AGENT,
        "-rw_timeout",
        "15000000",
        "-protocol_whitelist",
        "rtmp,crypto,file,http,https,tcp,tls,udp,rtp,httpproxy",
        "-thread_queue_size",
        "1024",
        "-analyzeduration",
        "20000000",
        "-probesize",
        "10000000",
        "-fflags",
        "+discardcorrupt+igndts",
    ]

    if record_url.lower().startswith(("http://", "https://")):
        command.extend(
            [
                "-reconnect",
                "1",
                "-reconnect_at_eof",
                "1",
                "-reconnect_streamed",
                "1",
                "-reconnect_delay_max",
                "15",
            ]
        )

    command.extend(
        [
            "-i",
            record_url,
            "-sn",
            "-dn",
            "-map",
            "0",
        ]
    )

    if proxy:
        command[1:1] = ["-http_proxy", proxy]

    if format_key in {"ts", "mpegts"}:
        codec_args = ["-c:v", "copy", "-c:a", "copy"]
        if segment_record:
            mux_args = [
                "-f",
                "segment",
                "-segment_time",
                str(segment_time),
                "-segment_format",
                "mpegts",
                "-reset_timestamps",
                "1",
                "-mpegts_flags",
                "+resend_headers",
                "-muxdelay",
                "0",
                "-muxpreload",
                "0",
            ]
        else:
            mux_args = [
                "-f",
                "mpegts",
                "-mpegts_flags",
                "+resend_headers",
                "-muxdelay",
                "0",
                "-muxpreload",
                "0",
            ]
    elif format_key == "flv":
        codec_args = ["-c:v", "copy", "-c:a", "copy", "-bsf:a", "aac_adtstoasc"]
        if segment_record:
            mux_args = [
                "-f",
                "segment",
                "-segment_time",
                str(segment_time),
                "-segment_format",
                "flv",
                "-reset_timestamps",
                "1",
            ]
        else:
            mux_args = ["-f", "flv"]
    elif format_key == "mp4":
        codec_args = ["-c:v", "copy", "-c:a", "copy"]
        if segment_record:
            mux_args = [
                "-f",
                "segment",
                "-segment_time",
                str(segment_time),
                "-segment_format",
                "mp4",
                "-reset_timestamps",
                "1",
                "-movflags",
                "+frag_keyframe+empty_moov+faststart+delay_moov",
                "-flags",
                "global_header",
            ]
        else:
            mux_args = [
                "-f",
                "mp4",
                "-movflags",
                "+faststart+frag_keyframe+empty_moov+delay_moov",
            ]
    elif format_key in {"mkv", "mov"}:
        codec_args = ["-c:v", "copy", "-c:a", "copy"]
        if segment_record:
            mux_args = [
                "-f",
                "segment",
                "-segment_time",
                str(segment_time),
                "-segment_format",
                format_key,
                "-reset_timestamps",
                "1",
            ]
        else:
            mux_args = ["-f", format_key]
    elif format_key in {"mp3", "m4a", "wav", "aac", "wma"}:
        codec_args = ["-vn"]
        if format_key == "mp3":
            codec_args += ["-c:a", "libmp3lame"]
        elif format_key == "aac":
            codec_args += ["-c:a", "aac"]
        elif format_key == "m4a":
            codec_args += ["-c:a", "aac"]
        elif format_key == "wav":
            codec_args += ["-c:a", "pcm_s16le"]
        else:
            codec_args += ["-c:a", "wmav2"]
        mux_args = ["-f", format_key]
    else:
        raise ValueError(f"不支持的录制格式: {save_format}")

    command.extend(codec_args)
    command.extend(mux_args)
    command.append(str(save_path))
    return command


def build_save_path(
    base_name: str, directory: Path, save_format: str, segment_record: bool
) -> Path:
    extension = save_format.lower()
    suffix = f"_%03d.{extension}" if segment_record else f".{extension}"
    return directory / f"{base_name}{suffix}"
