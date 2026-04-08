using System.Collections.Generic;
using System.Text.Json;
using StreamCap.Desktop.Infrastructure;

namespace StreamCap.Desktop.Models;

public sealed class CoreSettings : ObservableObject
{
    private string _liveSavePath = string.Empty;
    private bool _filenameIncludesTitle;
    private string _customFilenameTemplate = "{anchor_name}_{title}_{time}";
    private bool _removeEmojis;
    private bool _folderNamePlatform = true;
    private bool _folderNameAuthor = true;
    private bool _folderNameTime;
    private bool _folderNameTitle;
    private bool _enableProxy;
    private string _proxyAddress = string.Empty;
    private string _videoFormat = "TS";
    private string _recordQuality = "OD";
    private string _loopTimeSeconds = "180";
    private bool _segmentedRecordingEnabled = true;
    private bool _forceHttpsRecording = true;
    private string _defaultLiveSource = "FLV";
    private bool _flvUseDirectDownload;
    private string _recordingSpaceThreshold = "2.0";
    private string _videoSegmentTime = "1800";
    private bool _convertToMp4 = true;
    private bool _deleteOriginal;
    private bool _generateTimeSubtitleFile;
    private bool _executeCustomScript;
    private string _customScriptCommand = string.Empty;
    private string _defaultPlatformWithProxy = string.Empty;
    private bool _systemNotificationEnabled = true;
    private bool _streamStartNotificationEnabled;
    private bool _streamEndNotificationEnabled;
    private bool _onlyNotifyNoRecord;
    private string _customNotificationTitle = string.Empty;
    private string _customStreamStartContent = string.Empty;
    private string _customStreamEndContent = string.Empty;
    private bool _dingtalkEnabled;
    private bool _wechatEnabled;
    private bool _barkEnabled;
    private bool _ntfyEnabled;
    private bool _serverchanEnabled;
    private bool _telegramEnabled;
    private bool _emailEnabled;
    private string _dingtalkWebhookUrl = string.Empty;
    private string _dingtalkAtObjects = string.Empty;
    private bool _dingtalkAtAll;
    private string _wechatWebhookUrl = string.Empty;
    private string _barkWebhookUrl = string.Empty;
    private string _barkInterruptLevel = "active";
    private string _barkSound = string.Empty;
    private string _ntfyServerUrl = "https://ntfy.sh/xxxxx";
    private string _ntfyTags = "tada";
    private string _ntfyEmail = string.Empty;
    private string _ntfyActionUrl = string.Empty;
    private string _serverchanSendkey = string.Empty;
    private string _serverchanChannel = "9";
    private string _serverchanTags = "直播通知";
    private string _telegramApiToken = string.Empty;
    private string _telegramChatId = string.Empty;
    private string _smtpServer = "smtp.qq.com";
    private string _emailUsername = string.Empty;
    private string _emailPassword = string.Empty;
    private string _senderEmail = string.Empty;
    private string _senderName = string.Empty;
    private string _recipientEmail = string.Empty;
    private string _platformMaxConcurrentRequests = "3";
    private bool _minimizeToTrayOnMinimize;
    private bool _minimizeToTrayOnClose = true;

    public string LiveSavePath { get => _liveSavePath; set => SetProperty(ref _liveSavePath, value); }

    public bool FilenameIncludesTitle { get => _filenameIncludesTitle; set => SetProperty(ref _filenameIncludesTitle, value); }

    public string CustomFilenameTemplate { get => _customFilenameTemplate; set => SetProperty(ref _customFilenameTemplate, value); }

    public bool RemoveEmojis { get => _removeEmojis; set => SetProperty(ref _removeEmojis, value); }

    public bool FolderNamePlatform { get => _folderNamePlatform; set => SetProperty(ref _folderNamePlatform, value); }

    public bool FolderNameAuthor { get => _folderNameAuthor; set => SetProperty(ref _folderNameAuthor, value); }

    public bool FolderNameTime { get => _folderNameTime; set => SetProperty(ref _folderNameTime, value); }

    public bool FolderNameTitle { get => _folderNameTitle; set => SetProperty(ref _folderNameTitle, value); }

    public bool EnableProxy { get => _enableProxy; set => SetProperty(ref _enableProxy, value); }

    public string ProxyAddress { get => _proxyAddress; set => SetProperty(ref _proxyAddress, value); }

    public string VideoFormat { get => _videoFormat; set => SetProperty(ref _videoFormat, value); }

    public string RecordQuality { get => _recordQuality; set => SetProperty(ref _recordQuality, value); }

    public string LoopTimeSeconds { get => _loopTimeSeconds; set => SetProperty(ref _loopTimeSeconds, value); }

    public bool SegmentedRecordingEnabled { get => _segmentedRecordingEnabled; set => SetProperty(ref _segmentedRecordingEnabled, value); }

    public bool ForceHttpsRecording { get => _forceHttpsRecording; set => SetProperty(ref _forceHttpsRecording, value); }

    public string DefaultLiveSource { get => _defaultLiveSource; set => SetProperty(ref _defaultLiveSource, value); }

    public bool FlvUseDirectDownload { get => _flvUseDirectDownload; set => SetProperty(ref _flvUseDirectDownload, value); }

    public string RecordingSpaceThreshold { get => _recordingSpaceThreshold; set => SetProperty(ref _recordingSpaceThreshold, value); }

    public string VideoSegmentTime { get => _videoSegmentTime; set => SetProperty(ref _videoSegmentTime, value); }

    public bool ConvertToMp4 { get => _convertToMp4; set => SetProperty(ref _convertToMp4, value); }

    public bool DeleteOriginal { get => _deleteOriginal; set => SetProperty(ref _deleteOriginal, value); }

    public bool GenerateTimeSubtitleFile { get => _generateTimeSubtitleFile; set => SetProperty(ref _generateTimeSubtitleFile, value); }

    public bool ExecuteCustomScript { get => _executeCustomScript; set => SetProperty(ref _executeCustomScript, value); }

    public string CustomScriptCommand { get => _customScriptCommand; set => SetProperty(ref _customScriptCommand, value); }

    public string DefaultPlatformWithProxy { get => _defaultPlatformWithProxy; set => SetProperty(ref _defaultPlatformWithProxy, value); }

    public bool SystemNotificationEnabled { get => _systemNotificationEnabled; set => SetProperty(ref _systemNotificationEnabled, value); }

    public bool StreamStartNotificationEnabled { get => _streamStartNotificationEnabled; set => SetProperty(ref _streamStartNotificationEnabled, value); }

    public bool StreamEndNotificationEnabled { get => _streamEndNotificationEnabled; set => SetProperty(ref _streamEndNotificationEnabled, value); }

    public bool OnlyNotifyNoRecord { get => _onlyNotifyNoRecord; set => SetProperty(ref _onlyNotifyNoRecord, value); }

    public string CustomNotificationTitle { get => _customNotificationTitle; set => SetProperty(ref _customNotificationTitle, value); }

    public string CustomStreamStartContent { get => _customStreamStartContent; set => SetProperty(ref _customStreamStartContent, value); }

    public string CustomStreamEndContent { get => _customStreamEndContent; set => SetProperty(ref _customStreamEndContent, value); }

    public bool DingtalkEnabled { get => _dingtalkEnabled; set => SetProperty(ref _dingtalkEnabled, value); }

    public bool WechatEnabled { get => _wechatEnabled; set => SetProperty(ref _wechatEnabled, value); }

    public bool BarkEnabled { get => _barkEnabled; set => SetProperty(ref _barkEnabled, value); }

    public bool NtfyEnabled { get => _ntfyEnabled; set => SetProperty(ref _ntfyEnabled, value); }

    public bool ServerchanEnabled { get => _serverchanEnabled; set => SetProperty(ref _serverchanEnabled, value); }

    public bool TelegramEnabled { get => _telegramEnabled; set => SetProperty(ref _telegramEnabled, value); }

    public bool EmailEnabled { get => _emailEnabled; set => SetProperty(ref _emailEnabled, value); }

    public string DingtalkWebhookUrl { get => _dingtalkWebhookUrl; set => SetProperty(ref _dingtalkWebhookUrl, value); }

    public string DingtalkAtObjects { get => _dingtalkAtObjects; set => SetProperty(ref _dingtalkAtObjects, value); }

    public bool DingtalkAtAll { get => _dingtalkAtAll; set => SetProperty(ref _dingtalkAtAll, value); }

    public string WechatWebhookUrl { get => _wechatWebhookUrl; set => SetProperty(ref _wechatWebhookUrl, value); }

    public string BarkWebhookUrl { get => _barkWebhookUrl; set => SetProperty(ref _barkWebhookUrl, value); }

    public string BarkInterruptLevel { get => _barkInterruptLevel; set => SetProperty(ref _barkInterruptLevel, value); }

    public string BarkSound { get => _barkSound; set => SetProperty(ref _barkSound, value); }

    public string NtfyServerUrl { get => _ntfyServerUrl; set => SetProperty(ref _ntfyServerUrl, value); }

    public string NtfyTags { get => _ntfyTags; set => SetProperty(ref _ntfyTags, value); }

    public string NtfyEmail { get => _ntfyEmail; set => SetProperty(ref _ntfyEmail, value); }

    public string NtfyActionUrl { get => _ntfyActionUrl; set => SetProperty(ref _ntfyActionUrl, value); }

    public string ServerchanSendkey { get => _serverchanSendkey; set => SetProperty(ref _serverchanSendkey, value); }

    public string ServerchanChannel { get => _serverchanChannel; set => SetProperty(ref _serverchanChannel, value); }

    public string ServerchanTags { get => _serverchanTags; set => SetProperty(ref _serverchanTags, value); }

    public string TelegramApiToken { get => _telegramApiToken; set => SetProperty(ref _telegramApiToken, value); }

    public string TelegramChatId { get => _telegramChatId; set => SetProperty(ref _telegramChatId, value); }

    public string SmtpServer { get => _smtpServer; set => SetProperty(ref _smtpServer, value); }

    public string EmailUsername { get => _emailUsername; set => SetProperty(ref _emailUsername, value); }

    public string EmailPassword { get => _emailPassword; set => SetProperty(ref _emailPassword, value); }

    public string SenderEmail { get => _senderEmail; set => SetProperty(ref _senderEmail, value); }

    public string SenderName { get => _senderName; set => SetProperty(ref _senderName, value); }

    public string RecipientEmail { get => _recipientEmail; set => SetProperty(ref _recipientEmail, value); }

    public string PlatformMaxConcurrentRequests { get => _platformMaxConcurrentRequests; set => SetProperty(ref _platformMaxConcurrentRequests, value); }

    public bool MinimizeToTrayOnMinimize { get => _minimizeToTrayOnMinimize; set => SetProperty(ref _minimizeToTrayOnMinimize, value); }

    public bool MinimizeToTrayOnClose { get => _minimizeToTrayOnClose; set => SetProperty(ref _minimizeToTrayOnClose, value); }

    public void LoadFromJson(JsonElement element)
    {
        LiveSavePath = GetString(element, "live_save_path");
        FilenameIncludesTitle = GetBool(element, "filename_includes_title");
        CustomFilenameTemplate = GetString(element, "custom_filename_template", "{anchor_name}_{title}_{time}");
        RemoveEmojis = GetBool(element, "remove_emojis");
        FolderNamePlatform = GetBool(element, "folder_name_platform", true);
        FolderNameAuthor = GetBool(element, "folder_name_author", true);
        FolderNameTime = GetBool(element, "folder_name_time");
        FolderNameTitle = GetBool(element, "folder_name_title");
        EnableProxy = GetBool(element, "enable_proxy");
        ProxyAddress = GetString(element, "proxy_address");
        VideoFormat = GetString(element, "video_format", "TS");
        RecordQuality = GetString(element, "record_quality", "OD");
        LoopTimeSeconds = GetString(element, "loop_time_seconds", "180");
        SegmentedRecordingEnabled = GetBool(element, "segmented_recording_enabled", true);
        ForceHttpsRecording = GetBool(element, "force_https_recording", true);
        DefaultLiveSource = GetString(element, "default_live_source", "FLV");
        FlvUseDirectDownload = GetBool(element, "flv_use_direct_download");
        RecordingSpaceThreshold = GetString(element, "recording_space_threshold", "2.0");
        VideoSegmentTime = GetString(element, "video_segment_time", "1800");
        ConvertToMp4 = GetBool(element, "convert_to_mp4", true);
        DeleteOriginal = GetBool(element, "delete_original");
        GenerateTimeSubtitleFile = GetBool(element, "generate_time_subtitle_file");
        ExecuteCustomScript = GetBool(element, "execute_custom_script");
        CustomScriptCommand = GetString(element, "custom_script_command");
        DefaultPlatformWithProxy = GetString(element, "default_platform_with_proxy");
        SystemNotificationEnabled = GetBool(element, "system_notification_enabled", true);
        StreamStartNotificationEnabled = GetBool(element, "stream_start_notification_enabled");
        StreamEndNotificationEnabled = GetBool(element, "stream_end_notification_enabled");
        OnlyNotifyNoRecord = GetBool(element, "only_notify_no_record");
        CustomNotificationTitle = GetString(element, "custom_notification_title");
        CustomStreamStartContent = GetString(element, "custom_stream_start_content");
        CustomStreamEndContent = GetString(element, "custom_stream_end_content");
        DingtalkEnabled = GetBool(element, "dingtalk_enabled");
        WechatEnabled = GetBool(element, "wechat_enabled");
        BarkEnabled = GetBool(element, "bark_enabled");
        NtfyEnabled = GetBool(element, "ntfy_enabled");
        ServerchanEnabled = GetBool(element, "serverchan_enabled");
        TelegramEnabled = GetBool(element, "telegram_enabled");
        EmailEnabled = GetBool(element, "email_enabled");
        DingtalkWebhookUrl = GetString(element, "dingtalk_webhook_url");
        DingtalkAtObjects = GetString(element, "dingtalk_at_objects");
        DingtalkAtAll = GetBool(element, "dingtalk_at_all");
        WechatWebhookUrl = GetString(element, "wechat_webhook_url");
        BarkWebhookUrl = GetString(element, "bark_webhook_url");
        BarkInterruptLevel = GetString(element, "bark_interrupt_level", "active");
        BarkSound = GetString(element, "bark_sound");
        NtfyServerUrl = GetString(element, "ntfy_server_url", "https://ntfy.sh/xxxxx");
        NtfyTags = GetString(element, "ntfy_tags", "tada");
        NtfyEmail = GetString(element, "ntfy_email");
        NtfyActionUrl = GetString(element, "ntfy_action_url");
        ServerchanSendkey = GetString(element, "serverchan_sendkey");
        ServerchanChannel = GetString(element, "serverchan_channel", "9");
        ServerchanTags = GetString(element, "serverchan_tags", "直播通知");
        TelegramApiToken = GetString(element, "telegram_api_token");
        TelegramChatId = GetString(element, "telegram_chat_id");
        SmtpServer = GetString(element, "smtp_server", "smtp.qq.com");
        EmailUsername = GetString(element, "email_username");
        EmailPassword = GetString(element, "email_password");
        SenderEmail = GetString(element, "sender_email");
        SenderName = GetString(element, "sender_name");
        RecipientEmail = GetString(element, "recipient_email");
        PlatformMaxConcurrentRequests = GetString(element, "platform_max_concurrent_requests", "3");
        MinimizeToTrayOnMinimize = GetBool(element, "minimize_to_tray_on_minimize");
        MinimizeToTrayOnClose = GetBool(element, "minimize_to_tray_on_close", true);
    }

    public Dictionary<string, object?> ToWorkerPayload()
    {
        return new Dictionary<string, object?>
        {
            ["live_save_path"] = LiveSavePath,
            ["filename_includes_title"] = FilenameIncludesTitle,
            ["custom_filename_template"] = CustomFilenameTemplate,
            ["remove_emojis"] = RemoveEmojis,
            ["folder_name_platform"] = FolderNamePlatform,
            ["folder_name_author"] = FolderNameAuthor,
            ["folder_name_time"] = FolderNameTime,
            ["folder_name_title"] = FolderNameTitle,
            ["enable_proxy"] = EnableProxy,
            ["proxy_address"] = ProxyAddress,
            ["video_format"] = VideoFormat,
            ["record_quality"] = RecordQuality,
            ["loop_time_seconds"] = LoopTimeSeconds,
            ["segmented_recording_enabled"] = SegmentedRecordingEnabled,
            ["force_https_recording"] = ForceHttpsRecording,
            ["default_live_source"] = DefaultLiveSource,
            ["flv_use_direct_download"] = FlvUseDirectDownload,
            ["recording_space_threshold"] = RecordingSpaceThreshold,
            ["video_segment_time"] = VideoSegmentTime,
            ["convert_to_mp4"] = ConvertToMp4,
            ["delete_original"] = DeleteOriginal,
            ["generate_time_subtitle_file"] = GenerateTimeSubtitleFile,
            ["execute_custom_script"] = ExecuteCustomScript,
            ["custom_script_command"] = CustomScriptCommand,
            ["default_platform_with_proxy"] = DefaultPlatformWithProxy,
            ["system_notification_enabled"] = SystemNotificationEnabled,
            ["stream_start_notification_enabled"] = StreamStartNotificationEnabled,
            ["stream_end_notification_enabled"] = StreamEndNotificationEnabled,
            ["only_notify_no_record"] = OnlyNotifyNoRecord,
            ["custom_notification_title"] = CustomNotificationTitle,
            ["custom_stream_start_content"] = CustomStreamStartContent,
            ["custom_stream_end_content"] = CustomStreamEndContent,
            ["dingtalk_enabled"] = DingtalkEnabled,
            ["wechat_enabled"] = WechatEnabled,
            ["bark_enabled"] = BarkEnabled,
            ["ntfy_enabled"] = NtfyEnabled,
            ["serverchan_enabled"] = ServerchanEnabled,
            ["telegram_enabled"] = TelegramEnabled,
            ["email_enabled"] = EmailEnabled,
            ["dingtalk_webhook_url"] = DingtalkWebhookUrl,
            ["dingtalk_at_objects"] = DingtalkAtObjects,
            ["dingtalk_at_all"] = DingtalkAtAll,
            ["wechat_webhook_url"] = WechatWebhookUrl,
            ["bark_webhook_url"] = BarkWebhookUrl,
            ["bark_interrupt_level"] = BarkInterruptLevel,
            ["bark_sound"] = BarkSound,
            ["ntfy_server_url"] = NtfyServerUrl,
            ["ntfy_tags"] = NtfyTags,
            ["ntfy_email"] = NtfyEmail,
            ["ntfy_action_url"] = NtfyActionUrl,
            ["serverchan_sendkey"] = ServerchanSendkey,
            ["serverchan_channel"] = ServerchanChannel,
            ["serverchan_tags"] = ServerchanTags,
            ["telegram_api_token"] = TelegramApiToken,
            ["telegram_chat_id"] = TelegramChatId,
            ["smtp_server"] = SmtpServer,
            ["email_username"] = EmailUsername,
            ["email_password"] = EmailPassword,
            ["sender_email"] = SenderEmail,
            ["sender_name"] = SenderName,
            ["recipient_email"] = RecipientEmail,
            ["platform_max_concurrent_requests"] = PlatformMaxConcurrentRequests,
            ["minimize_to_tray_on_minimize"] = MinimizeToTrayOnMinimize,
            ["minimize_to_tray_on_close"] = MinimizeToTrayOnClose,
        };
    }

    private static string GetString(JsonElement element, string propertyName, string fallback = "")
    {
        return element.TryGetProperty(propertyName, out var property) && property.ValueKind != JsonValueKind.Null
            ? property.GetString() ?? fallback
            : fallback;
    }

    private static bool GetBool(JsonElement element, string propertyName, bool fallback = false)
    {
        if (!element.TryGetProperty(propertyName, out var property))
        {
            return fallback;
        }

        return property.ValueKind switch
        {
            JsonValueKind.True => true,
            JsonValueKind.False => false,
            JsonValueKind.String => bool.TryParse(property.GetString(), out var parsed) ? parsed : fallback,
            _ => fallback,
        };
    }
}
