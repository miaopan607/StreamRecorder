export type Json =
  null | boolean | number | string | Json[] | { [key: string]: Json };
export interface Job {
  id: string;
  url: string;
  streamer_name: string;
  quality: string;
  record_format: string;
  segment_record: boolean;
  segment_time: string;
  monitor_status: boolean;
  scheduled_recording: boolean;
  scheduled_start_time: string;
  monitor_hours: string;
  recording_dir: string;
  enabled_message_push: boolean;
  only_notify_no_record: boolean;
  flv_use_direct_download: boolean;
  platform: string;
  platform_key: string;
  status_info: string;
  display_title: string;
  title: string;
  error_message: string;
  created_at: string;
  updated_at: string;
  last_checked_at: string;
  live_title: string;
  record_url: string;
  latest_output_path: string;
  duration_text: string;
  speed_text: string;
  recording_started_at: string;
}
export type JobDraft = Pick<
  Job,
  | "url"
  | "streamer_name"
  | "quality"
  | "record_format"
  | "segment_record"
  | "segment_time"
  | "monitor_status"
  | "scheduled_recording"
  | "scheduled_start_time"
  | "monitor_hours"
  | "recording_dir"
  | "enabled_message_push"
  | "only_notify_no_record"
  | "flv_use_direct_download"
> & { id: string | null };
export interface CoreSettings {
  language: string;
  live_save_path: string;
  filename_includes_title: boolean;
  custom_filename_template: string;
  remove_emojis: boolean;
  folder_name_platform: boolean;
  folder_name_author: boolean;
  folder_name_time: boolean;
  folder_name_title: boolean;
  enable_proxy: boolean;
  proxy_address: string;
  video_format: string;
  record_quality: string;
  loop_time_seconds: string;
  segmented_recording_enabled: boolean;
  force_https_recording: boolean;
  default_live_source: string;
  flv_use_direct_download: boolean;
  recording_space_threshold: string;
  video_segment_time: string;
  convert_to_mp4: boolean;
  delete_original: boolean;
  generate_time_subtitle_file: boolean;
  execute_custom_script: boolean;
  custom_script_command: string;
  default_platform_with_proxy: string;
  system_notification_enabled: boolean;
  system_stream_start_notification_enabled: boolean;
  system_stream_end_notification_enabled: boolean;
  system_error_notification_enabled: boolean;
  system_minimize_to_tray_notification_enabled: boolean;
  system_close_to_tray_notification_enabled: boolean;
  stream_start_notification_enabled: boolean;
  stream_end_notification_enabled: boolean;
  stream_error_notification_enabled: boolean;
  only_notify_no_record: boolean;
  custom_notification_title: string;
  custom_stream_start_content: string;
  custom_stream_end_content: string;
  custom_stream_error_content: string;
  dingtalk_enabled: boolean;
  wechat_enabled: boolean;
  bark_enabled: boolean;
  ntfy_enabled: boolean;
  serverchan_enabled: boolean;
  telegram_enabled: boolean;
  email_enabled: boolean;
  dingtalk_webhook_url: string;
  dingtalk_at_objects: string;
  dingtalk_at_all: boolean;
  wechat_webhook_url: string;
  bark_webhook_url: string;
  bark_interrupt_level: string;
  bark_sound: string;
  ntfy_server_url: string;
  ntfy_tags: string;
  ntfy_email: string;
  ntfy_action_url: string;
  serverchan_sendkey: string;
  serverchan_channel: string;
  serverchan_tags: string;
  telegram_api_token: string;
  telegram_chat_id: string;
  smtp_server: string;
  email_username: string;
  email_password: string;
  sender_email: string;
  sender_name: string;
  recipient_email: string;
  theme_color: string;
  is_grid_view: boolean;
  theme_mode: string;
  platform_max_concurrent_requests: string;
  minimize_to_tray_on_minimize: boolean;
  minimize_to_tray_on_close: boolean;
  [key: string]: Json;
}
export interface CoreInfo {
  name: string;
  version: string;
  ffmpeg_available: boolean;
  updated_at: string;
}
export interface Snapshot {
  revision: number;
  app: CoreInfo;
  settings: CoreSettings;
  jobs: Job[];
}
export interface CoreState {
  status: "starting" | "connected" | "disconnected";
  error: string;
  revision: number;
}
export interface Dependency {
  available: boolean;
  version: string;
  path: string;
  error?: string;
}
export interface UiState {
  IsCardLayout: boolean;
  VisibleColumns: Record<string, boolean>;
}
export interface DesktopBootstrap {
  snapshot: Snapshot;
  cookies: Record<string, string>;
  accounts: Record<string, Json>;
  dependencies: Record<string, Dependency>;
  ui_state: UiState;
  autostart_enabled: boolean;
  core_state: CoreState;
  logs: string[];
  app_root: string;
  data_root: string;
}
export interface CoreEvent {
  name: "snapshot_changed";
  body: Snapshot;
}
export const defaultSettings: CoreSettings = {
  language: "Chinese",
  live_save_path: "",
  filename_includes_title: false,
  custom_filename_template: "{anchor_name}_{title}_{time}",
  remove_emojis: false,
  folder_name_platform: true,
  folder_name_author: true,
  folder_name_time: false,
  folder_name_title: false,
  enable_proxy: false,
  proxy_address: "",
  video_format: "TS",
  record_quality: "OD",
  loop_time_seconds: "180",
  segmented_recording_enabled: true,
  force_https_recording: true,
  default_live_source: "FLV",
  flv_use_direct_download: false,
  recording_space_threshold: "2.0",
  video_segment_time: "1800",
  convert_to_mp4: true,
  delete_original: false,
  generate_time_subtitle_file: false,
  execute_custom_script: false,
  custom_script_command: "",
  default_platform_with_proxy: "",
  system_notification_enabled: true,
  system_stream_start_notification_enabled: true,
  system_stream_end_notification_enabled: false,
  system_error_notification_enabled: false,
  system_minimize_to_tray_notification_enabled: false,
  system_close_to_tray_notification_enabled: false,
  stream_start_notification_enabled: false,
  stream_end_notification_enabled: false,
  stream_error_notification_enabled: false,
  only_notify_no_record: false,
  custom_notification_title: "",
  custom_stream_start_content: "",
  custom_stream_end_content: "",
  custom_stream_error_content: "",
  dingtalk_enabled: false,
  wechat_enabled: false,
  bark_enabled: false,
  ntfy_enabled: false,
  serverchan_enabled: false,
  telegram_enabled: false,
  email_enabled: false,
  dingtalk_webhook_url: "",
  dingtalk_at_objects: "",
  dingtalk_at_all: false,
  wechat_webhook_url: "",
  bark_webhook_url: "",
  bark_interrupt_level: "active",
  bark_sound: "",
  ntfy_server_url: "https://ntfy.sh/xxxxx",
  ntfy_tags: "tada",
  ntfy_email: "",
  ntfy_action_url: "",
  serverchan_sendkey: "",
  serverchan_channel: "9",
  serverchan_tags: "直播通知",
  telegram_api_token: "",
  telegram_chat_id: "",
  smtp_server: "smtp.qq.com",
  email_username: "",
  email_password: "",
  sender_email: "",
  sender_name: "",
  recipient_email: "",
  theme_color: "blue",
  is_grid_view: true,
  theme_mode: "light",
  platform_max_concurrent_requests: "3",
  minimize_to_tray_on_minimize: false,
  minimize_to_tray_on_close: true,
};
