import type { CoreSettings } from "./types";
import { formats, qualities } from "./jobs";
export interface SettingField {
  key: keyof CoreSettings & string;
  label: string;
  options?: Record<string, string>;
  secret?: boolean;
  hint?: string;
}
export interface SettingSection {
  title: string;
  description?: string;
  fields: SettingField[];
}
const field = (
  key: SettingField["key"],
  label: string,
  extra: Omit<SettingField, "key" | "label"> = {},
): SettingField => ({ key, label, ...extra });
export const recordingSections: SettingSection[] = [
  {
    title: "存储与命名",
    description: "相对路径按程序所在目录解析。",
    fields: [
      field("live_save_path", "默认保存路径", {
        hint: "留空保存到 runtime/recordings",
      }),
      field("custom_filename_template", "文件名模板", {
        hint: "可使用 {anchor_name}、{title}、{time}、{platform}",
      }),
      field("filename_includes_title", "文件名包含标题"),
      field("remove_emojis", "移除表情"),
      field("folder_name_platform", "目录含平台"),
      field("folder_name_author", "目录含主播"),
      field("folder_name_time", "目录含日期"),
      field("folder_name_title", "目录含标题"),
    ],
  },
  {
    title: "网络与监控",
    fields: [
      field("enable_proxy", "启用代理"),
      field("proxy_address", "代理地址"),
      field("loop_time_seconds", "轮询间隔（秒）"),
      field("default_platform_with_proxy", "默认代理平台（逗号分隔）"),
      field("platform_max_concurrent_requests", "平台并发数"),
    ],
  },
  {
    title: "格式与处理",
    fields: [
      field("video_format", "默认录制格式", {
        options: Object.fromEntries(formats.map((format) => [format, format])),
      }),
      field("record_quality", "默认画质", { options: qualities }),
      field("segmented_recording_enabled", "启用分段录制"),
      field("force_https_recording", "强制 HTTPS"),
      field("flv_use_direct_download", "允许 FLV 直连下载"),
      field("generate_time_subtitle_file", "生成时间字幕文件"),
      field("convert_to_mp4", "录后 TS 转 MP4"),
      field("delete_original", "转码后删除原文件"),
      field("video_segment_time", "分段时长（秒）"),
      field("default_live_source", "默认直播源", {
        options: { FLV: "FLV", HLS: "HLS" },
      }),
      field("recording_space_threshold", "最低剩余磁盘空间（GB）", {
        hint: "低于阈值时不启动新录制",
      }),
      field("execute_custom_script", "执行自定义脚本"),
      field("custom_script_command", "自定义脚本命令"),
    ],
  },
];
export const appearanceSections: SettingSection[] = [
  {
    title: "外观",
    fields: [
      field("theme_mode", "主题", {
        options: { light: "浅色", dark: "深色", system: "跟随系统" },
      }),
    ],
  },
  {
    title: "托盘",
    description: "关闭到托盘后，核心服务仍继续运行。",
    fields: [
      field("minimize_to_tray_on_minimize", "点击最小化时缩到托盘"),
      field("minimize_to_tray_on_close", "点击关闭时缩到托盘"),
    ],
  },
];
export const notificationSections: SettingSection[] = [
  {
    title: "系统通知",
    fields: [
      field("system_notification_enabled", "启用系统通知"),
      field("system_stream_start_notification_enabled", "开播通知"),
      field("system_stream_end_notification_enabled", "下播通知"),
      field("system_error_notification_enabled", "异常通知"),
      field("system_minimize_to_tray_notification_enabled", "最小化到托盘提示"),
      field("system_close_to_tray_notification_enabled", "关闭到托盘提示"),
      field("only_notify_no_record", "开播时仅通知，不录制"),
    ],
  },
  {
    title: "通知模板",
    description: "内容支持 {streamer_name}、{title}、{error}。",
    fields: [
      field("custom_notification_title", "自定义通知标题"),
      field("custom_stream_start_content", "开播通知内容"),
      field("custom_stream_end_content", "下播通知内容"),
      field("custom_stream_error_content", "异常通知内容"),
    ],
  },
  {
    title: "远程推送",
    fields: [
      field("stream_start_notification_enabled", "开播推送"),
      field("stream_end_notification_enabled", "下播推送"),
      field("stream_error_notification_enabled", "异常推送"),
    ],
  },
  {
    title: "钉钉",
    fields: [
      field("dingtalk_enabled", "启用钉钉"),
      field("dingtalk_webhook_url", "钉钉 Webhook", { secret: true }),
      field("dingtalk_at_objects", "钉钉 @对象"),
      field("dingtalk_at_all", "钉钉 @所有人"),
    ],
  },
  {
    title: "企业微信",
    fields: [
      field("wechat_enabled", "启用企业微信"),
      field("wechat_webhook_url", "企业微信 Webhook", { secret: true }),
    ],
  },
  {
    title: "Bark",
    fields: [
      field("bark_enabled", "启用 Bark"),
      field("bark_webhook_url", "Bark 地址", { secret: true }),
      field("bark_interrupt_level", "Bark 中断级别"),
      field("bark_sound", "Bark 声音"),
    ],
  },
  {
    title: "Ntfy",
    fields: [
      field("ntfy_enabled", "启用 Ntfy"),
      field("ntfy_server_url", "Ntfy 地址"),
      field("ntfy_tags", "Ntfy 标签"),
      field("ntfy_email", "Ntfy 邮箱"),
      field("ntfy_action_url", "Ntfy 动作链接"),
    ],
  },
  {
    title: "Telegram",
    fields: [
      field("telegram_enabled", "启用 Telegram"),
      field("telegram_api_token", "Telegram Token", { secret: true }),
      field("telegram_chat_id", "Telegram Chat ID"),
    ],
  },
  {
    title: "ServerChan",
    fields: [
      field("serverchan_enabled", "启用 ServerChan"),
      field("serverchan_sendkey", "ServerChan SendKey", { secret: true }),
      field("serverchan_channel", "ServerChan Channel"),
      field("serverchan_tags", "ServerChan 标签"),
    ],
  },
  {
    title: "邮件",
    fields: [
      field("email_enabled", "启用邮件"),
      field("smtp_server", "SMTP 服务器"),
      field("email_username", "邮箱用户名"),
      field("email_password", "邮箱密码", { secret: true }),
      field("sender_email", "发件邮箱"),
      field("sender_name", "发件人名称"),
      field("recipient_email", "收件邮箱"),
    ],
  },
];
export const cookiePlatforms = [
  "douyin",
  "tiktok",
  "kuaishou",
  "huya",
  "douyu",
  "yy",
  "bilibili",
  "xhs",
  "bigo",
  "blued",
  "soop",
  "netease",
  "qiandurebo",
  "pandalive",
  "maoerfm",
  "winktv",
  "flextv",
  "look",
  "popkontv",
  "twitcasting",
  "baidu",
  "weibo",
  "kugou",
  "twitch",
  "liveme",
  "huajiao",
  "liuxing",
  "showroom",
  "acfun",
  "changliao",
  "yinbo",
  "inke",
  "zhihu",
  "chzzk",
  "haixiu",
  "vvxq",
  "17live",
  "lang",
  "piaopiao",
  "6room",
  "lehai",
  "catshow",
  "shopee",
  "youtube",
  "taobao",
  "jd",
];
export const accountKeys = [
  "sooplive_username",
  "sooplive_password",
  "flextv_username",
  "flextv_password",
  "popkontv_username",
  "popkontv_password",
  "twitcasting_account_type",
  "twitcasting_username",
  "twitcasting_password",
];
