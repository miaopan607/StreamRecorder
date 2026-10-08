import type { CoreSettings, Job, JobDraft } from "./types";
export const qualities: Record<string, string> = {
  OD: "原画",
  UHD: "超清",
  HD: "高清",
  SD: "标清",
  LD: "流畅",
};
export const formats = ["TS", "FLV", "MKV", "MOV", "MP4", "MP3", "M4A"];
export function newJob(settings: CoreSettings): JobDraft {
  return {
    id: null,
    url: "",
    streamer_name: "直播间",
    quality: settings.record_quality,
    record_format: settings.video_format,
    segment_record: settings.segmented_recording_enabled,
    segment_time: settings.video_segment_time,
    monitor_status: true,
    scheduled_recording: false,
    scheduled_start_time: "",
    monitor_hours: "5",
    recording_dir: "",
    enabled_message_push: true,
    only_notify_no_record: false,
    flv_use_direct_download: settings.flv_use_direct_download,
  };
}
export function jobDraft(job: Job): JobDraft {
  return {
    id: job.id,
    url: job.url,
    streamer_name: job.streamer_name,
    quality: job.quality,
    record_format: job.record_format,
    segment_record: job.segment_record,
    segment_time: job.segment_time,
    monitor_status: job.monitor_status,
    scheduled_recording: job.scheduled_recording,
    scheduled_start_time: job.scheduled_start_time,
    monitor_hours: job.monitor_hours,
    recording_dir: job.recording_dir,
    enabled_message_push: job.enabled_message_push,
    only_notify_no_record: job.only_notify_no_record,
    flv_use_direct_download: job.flv_use_direct_download,
  };
}
export function parseBatch(text: string, settings: CoreSettings) {
  const seen = new Set<string>();
  const jobs: JobDraft[] = [];
  const codes: Record<string, string> = {
    "0": "OD",
    "1": "UHD",
    "2": "HD",
    "3": "SD",
    "4": "LD",
  };
  for (const line of text
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter(Boolean)) {
    if (!/http/i.test(line)) continue;
    const parts = line
      .replaceAll("，", ",")
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    let code = "0",
      url = "",
      streamer = "直播间";
    if (parts.length >= 3) {
      [code, url, streamer] = parts;
    } else if (parts.length === 2) {
      if (/^http/i.test(parts[1])) [code, url] = parts;
      else [url, streamer] = parts;
    } else url = parts[0] ?? "";
    const key = url.toLowerCase();
    if (!url || seen.has(key)) continue;
    seen.add(key);
    jobs.push({
      ...newJob(settings),
      url,
      streamer_name: streamer || "直播间",
      quality: codes[code] ?? "OD",
    });
  }
  return jobs;
}
export function selectedTargets(
  jobs: Job[],
  selected: Set<string>,
  active: string | null,
  allWhenEmpty: boolean,
) {
  const checked = jobs.filter((job) => selected.has(job.id));
  if (checked.length) return checked;
  const current = jobs.find((job) => job.id === active);
  return current ? [current] : allWhenEmpty ? jobs : [];
}
export const columns = [
  ["PlatformColumn", "平台"],
  ["UrlColumn", "直播间链接"],
  ["StatusColumn", "状态"],
  ["QualityColumn", "画质"],
  ["FormatColumn", "格式"],
  ["DurationColumn", "时长"],
  ["SpeedColumn", "速度"],
  ["UpdatedAtColumn", "更新时间"],
] as const;
export function detailText(job: Job) {
  return `标题：${job.display_title}
平台：${job.platform}
状态：${job.status_info}
直播间：${job.url}
保存目录：${job.recording_dir || "默认目录"}
最新文件：${job.latest_output_path}
录制流地址：${job.record_url}
录制时长：${job.duration_text}
录制速度：${job.speed_text}`;
}
