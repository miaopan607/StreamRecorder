use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
pub const STOPPED: &str = "已停止";
pub const MONITORING: &str = "监控中";
pub const CHECKING: &str = "检测中";
pub const RECORDING: &str = "录制中";
pub const ERROR: &str = "错误";
pub const WAITING: &str = "未开播";
pub fn utc_now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CoreSettings {
    pub language: String,
    pub live_save_path: String,
    pub filename_includes_title: bool,
    pub custom_filename_template: String,
    pub remove_emojis: bool,
    pub folder_name_platform: bool,
    pub folder_name_author: bool,
    pub folder_name_time: bool,
    pub folder_name_title: bool,
    pub enable_proxy: bool,
    pub proxy_address: String,
    pub video_format: String,
    pub record_quality: String,
    pub loop_time_seconds: String,
    pub segmented_recording_enabled: bool,
    pub force_https_recording: bool,
    pub default_live_source: String,
    pub flv_use_direct_download: bool,
    pub recording_space_threshold: String,
    pub video_segment_time: String,
    pub convert_to_mp4: bool,
    pub delete_original: bool,
    pub generate_time_subtitle_file: bool,
    pub execute_custom_script: bool,
    pub custom_script_command: String,
    pub default_platform_with_proxy: String,
    pub system_notification_enabled: bool,
    pub system_stream_start_notification_enabled: bool,
    pub system_stream_end_notification_enabled: bool,
    pub system_error_notification_enabled: bool,
    pub system_minimize_to_tray_notification_enabled: bool,
    pub system_close_to_tray_notification_enabled: bool,
    pub stream_start_notification_enabled: bool,
    pub stream_end_notification_enabled: bool,
    pub stream_error_notification_enabled: bool,
    pub only_notify_no_record: bool,
    pub custom_notification_title: String,
    pub custom_stream_start_content: String,
    pub custom_stream_end_content: String,
    pub custom_stream_error_content: String,
    pub dingtalk_enabled: bool,
    pub wechat_enabled: bool,
    pub bark_enabled: bool,
    pub ntfy_enabled: bool,
    pub serverchan_enabled: bool,
    pub telegram_enabled: bool,
    pub email_enabled: bool,
    pub dingtalk_webhook_url: String,
    pub dingtalk_at_objects: String,
    pub dingtalk_at_all: bool,
    pub wechat_webhook_url: String,
    pub bark_webhook_url: String,
    pub bark_interrupt_level: String,
    pub bark_sound: String,
    pub ntfy_server_url: String,
    pub ntfy_tags: String,
    pub ntfy_email: String,
    pub ntfy_action_url: String,
    pub serverchan_sendkey: String,
    pub serverchan_channel: String,
    pub serverchan_tags: String,
    pub telegram_api_token: String,
    pub telegram_chat_id: String,
    pub smtp_server: String,
    pub email_username: String,
    pub email_password: String,
    pub sender_email: String,
    pub sender_name: String,
    pub recipient_email: String,
    pub theme_color: String,
    pub is_grid_view: bool,
    pub theme_mode: String,
    pub platform_max_concurrent_requests: String,
    pub minimize_to_tray_on_minimize: bool,
    pub minimize_to_tray_on_close: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
impl Default for CoreSettings {
    fn default() -> Self {
        Self {
            language: "Chinese".into(),
            live_save_path: "".into(),
            filename_includes_title: false,
            custom_filename_template: "{anchor_name}_{title}_{time}".into(),
            remove_emojis: false,
            folder_name_platform: true,
            folder_name_author: true,
            folder_name_time: false,
            folder_name_title: false,
            enable_proxy: false,
            proxy_address: "".into(),
            video_format: "TS".into(),
            record_quality: "OD".into(),
            loop_time_seconds: "180".into(),
            segmented_recording_enabled: true,
            force_https_recording: true,
            default_live_source: "FLV".into(),
            flv_use_direct_download: false,
            recording_space_threshold: "2.0".into(),
            video_segment_time: "1800".into(),
            convert_to_mp4: true,
            delete_original: false,
            generate_time_subtitle_file: false,
            execute_custom_script: false,
            custom_script_command: "".into(),
            default_platform_with_proxy: "".into(),
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
            custom_notification_title: "".into(),
            custom_stream_start_content: "".into(),
            custom_stream_end_content: "".into(),
            custom_stream_error_content: "".into(),
            dingtalk_enabled: false,
            wechat_enabled: false,
            bark_enabled: false,
            ntfy_enabled: false,
            serverchan_enabled: false,
            telegram_enabled: false,
            email_enabled: false,
            dingtalk_webhook_url: "".into(),
            dingtalk_at_objects: "".into(),
            dingtalk_at_all: false,
            wechat_webhook_url: "".into(),
            bark_webhook_url: "".into(),
            bark_interrupt_level: "active".into(),
            bark_sound: "".into(),
            ntfy_server_url: "https://ntfy.sh/xxxxx".into(),
            ntfy_tags: "tada".into(),
            ntfy_email: "".into(),
            ntfy_action_url: "".into(),
            serverchan_sendkey: "".into(),
            serverchan_channel: "9".into(),
            serverchan_tags: "直播通知".into(),
            telegram_api_token: "".into(),
            telegram_chat_id: "".into(),
            smtp_server: "smtp.qq.com".into(),
            email_username: "".into(),
            email_password: "".into(),
            sender_email: "".into(),
            sender_name: "".into(),
            recipient_email: "".into(),
            theme_color: "blue".into(),
            is_grid_view: true,
            theme_mode: "light".into(),
            platform_max_concurrent_requests: "3".into(),
            minimize_to_tray_on_minimize: false,
            minimize_to_tray_on_close: true,
            extra: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct JobInput {
    #[serde(alias = "rec_id")]
    pub id: Option<String>,
    pub url: String,
    #[serde(alias = "streamerName")]
    pub streamer_name: String,
    pub quality: String,
    #[serde(alias = "recordFormat")]
    pub record_format: String,
    #[serde(alias = "segmentRecord")]
    pub segment_record: bool,
    #[serde(alias = "segmentTime")]
    pub segment_time: String,
    #[serde(alias = "monitorStatus")]
    pub monitor_status: bool,
    #[serde(alias = "scheduledRecording")]
    pub scheduled_recording: bool,
    #[serde(alias = "scheduledStartTime")]
    pub scheduled_start_time: String,
    #[serde(alias = "monitorHours")]
    pub monitor_hours: String,
    #[serde(alias = "recordingDir")]
    pub recording_dir: String,
    #[serde(alias = "enabledMessagePush")]
    pub enabled_message_push: bool,
    #[serde(alias = "onlyNotifyNoRecord")]
    pub only_notify_no_record: bool,
    #[serde(alias = "flvUseDirectDownload")]
    pub flv_use_direct_download: bool,
    #[serde(alias = "createdAt")]
    pub created_at: String,
    #[serde(alias = "updatedAt")]
    pub updated_at: String,
}
impl Default for JobInput {
    fn default() -> Self {
        Self {
            id: None,
            url: "".into(),
            streamer_name: "直播间".into(),
            quality: "OD".into(),
            record_format: "TS".into(),
            segment_record: false,
            segment_time: "1800".into(),
            monitor_status: true,
            scheduled_recording: false,
            scheduled_start_time: "".into(),
            monitor_hours: "5".into(),
            recording_dir: "".into(),
            enabled_message_push: true,
            only_notify_no_record: false,
            flv_use_direct_download: false,
            created_at: utc_now(),
            updated_at: utc_now(),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct RecordingJob {
    #[serde(flatten)]
    pub input: JobInput,
    pub platform: String,
    pub platform_key: String,
    pub status_info: String,
    pub display_title: String,
    pub title: String,
    pub error_message: String,
    pub last_checked_at: String,
    pub live_title: String,
    pub record_url: String,
    pub latest_output_path: String,
    pub duration_text: String,
    pub speed_text: String,
    pub recording_started_at: String,
}
impl RecordingJob {
    pub fn new(mut input: JobInput) -> Self {
        if input.id.as_deref().is_none_or(str::is_empty) {
            input.id = Some(uuid::Uuid::new_v4().simple().to_string());
        }
        input.url = input.url.trim().to_owned();
        input.streamer_name = input.streamer_name.trim().to_owned();
        if input.streamer_name.is_empty() {
            input.streamer_name = "直播间".into();
        }
        input.quality = if input.quality.is_empty() {
            "OD".into()
        } else {
            input.quality.to_ascii_uppercase()
        };
        input.record_format = if input.record_format.is_empty() {
            "TS".into()
        } else {
            input.record_format.to_ascii_uppercase()
        };
        let mut job = Self {
            input,
            platform: "未知平台".into(),
            platform_key: "unknown".into(),
            status_info: "监控中".into(),
            display_title: "".into(),
            title: "".into(),
            error_message: "".into(),
            last_checked_at: "".into(),
            live_title: "".into(),
            record_url: "".into(),
            latest_output_path: "".into(),
            duration_text: "00:00:00".into(),
            speed_text: "X KB/s".into(),
            recording_started_at: "".into(),
        };
        job.refresh(false);
        job
    }
    pub fn id(&self) -> &str {
        self.input.id.as_deref().expect("任务 ID 在构造时确定")
    }
    pub fn refresh(&mut self, touch: bool) {
        let (name, key) = super::platforms::identify(&self.input.url);
        self.platform = name.into();
        self.platform_key = key.into();
        let quality = match self.input.quality.as_str() {
            "OD" => "原画",
            "UHD" => "超清",
            "HD" => "高清",
            "SD" => "标清",
            "LD" => "流畅",
            other => other,
        };
        self.title = format!("{} - {quality}", self.input.streamer_name);
        self.display_title.clone_from(&self.title);
        if touch {
            self.input.updated_at = utc_now();
        }
        if self.recording_started_at.is_empty() {
            self.status_info = if self.input.monitor_status {
                MONITORING
            } else {
                STOPPED
            }
            .into();
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct CoreInfo {
    pub name: String,
    pub version: String,
    pub ffmpeg_available: bool,
    pub updated_at: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub revision: u64,
    pub app: CoreInfo,
    pub settings: CoreSettings,
    pub jobs: Vec<RecordingJob>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CoreState {
    pub status: String,
    pub error: String,
    pub revision: u64,
}
#[derive(Serialize)]
pub struct PingReply {
    pub utc: String,
}
#[derive(Serialize)]
pub struct SettingsReply {
    pub settings: CoreSettings,
}
#[derive(Serialize)]
pub struct CookiesReply {
    pub cookies: BTreeMap<String, String>,
}
#[derive(Serialize)]
pub struct AccountsReply {
    pub accounts: BTreeMap<String, Value>,
}
#[derive(Serialize)]
pub struct UpsertReply {
    pub changed_ids: Vec<String>,
}
#[derive(Serialize)]
pub struct DeleteReply {
    pub deleted: usize,
}
#[derive(Serialize)]
pub struct MonitoringReply {
    pub changed_ids: Vec<String>,
    pub monitoring: bool,
}
#[derive(Serialize)]
pub struct RecheckReply {
    pub queued_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct CoreBootstrap {
    pub snapshot: Snapshot,
    pub core_state: CoreState,
    pub cookies: BTreeMap<String, String>,
    pub accounts: BTreeMap<String, Value>,
    pub logs: Vec<String>,
}
