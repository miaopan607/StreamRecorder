using System.Collections.Generic;
using System.Text.Json;
using StreamCap.Desktop.Infrastructure;

namespace StreamCap.Desktop.Models;

public sealed class RecordingJob : ObservableObject
{
    private bool _isBatchSelected;

    public string Id { get; set; } = string.Empty;

    public string Url { get; set; } = string.Empty;

    public string StreamerName { get; set; } = "直播间";

    public string Quality { get; set; } = "OD";

    public string RecordFormat { get; set; } = "TS";

    public bool SegmentRecord { get; set; }

    public string SegmentTime { get; set; } = "1800";

    public bool MonitorStatus { get; set; } = true;

    public bool ScheduledRecording { get; set; }

    public string ScheduledStartTime { get; set; } = string.Empty;

    public string MonitorHours { get; set; } = "5";

    public string RecordingDir { get; set; } = string.Empty;

    public bool EnabledMessagePush { get; set; } = true;

    public bool OnlyNotifyNoRecord { get; set; }

    public bool FlvUseDirectDownload { get; set; }

    public string Platform { get; set; } = "未知平台";

    public string PlatformKey { get; set; } = "unknown";

    public string StatusInfo { get; set; } = "已停止";

    public string DisplayTitle { get; set; } = string.Empty;

    public string Title { get; set; } = string.Empty;

    public string ErrorMessage { get; set; } = string.Empty;

    public string CreatedAt { get; set; } = string.Empty;

    public string UpdatedAt { get; set; } = string.Empty;

    public string LastCheckedAt { get; set; } = string.Empty;

    public string LiveTitle { get; set; } = string.Empty;

    public string RecordUrl { get; set; } = string.Empty;

    public string LatestOutputPath { get; set; } = string.Empty;

    public bool IsBatchSelected
    {
        get => _isBatchSelected;
        set => SetProperty(ref _isBatchSelected, value);
    }

    public string DisplayQuality => Quality switch
    {
        "OD" => "原画",
        "UHD" => "超清",
        "HD" => "高清",
        "SD" => "标清",
        "LD" => "流畅",
        _ => Quality,
    };

    public RecordingJob Clone()
    {
        return (RecordingJob)MemberwiseClone();
    }

    public Dictionary<string, object?> ToWorkerPayload()
    {
        return new Dictionary<string, object?>
        {
            ["id"] = string.IsNullOrWhiteSpace(Id) ? null : Id,
            ["url"] = Url.Trim(),
            ["streamer_name"] = string.IsNullOrWhiteSpace(StreamerName) ? "直播间" : StreamerName.Trim(),
            ["quality"] = Quality,
            ["record_format"] = RecordFormat,
            ["segment_record"] = SegmentRecord,
            ["segment_time"] = SegmentTime,
            ["monitor_status"] = MonitorStatus,
            ["scheduled_recording"] = ScheduledRecording,
            ["scheduled_start_time"] = ScheduledStartTime,
            ["monitor_hours"] = MonitorHours,
            ["recording_dir"] = RecordingDir,
            ["enabled_message_push"] = EnabledMessagePush,
            ["only_notify_no_record"] = OnlyNotifyNoRecord,
            ["flv_use_direct_download"] = FlvUseDirectDownload,
        };
    }

    public static RecordingJob FromJson(JsonElement element)
    {
        return new RecordingJob
        {
            Id = GetString(element, "id"),
            Url = GetString(element, "url"),
            StreamerName = GetString(element, "streamer_name", "直播间"),
            Quality = GetString(element, "quality", "OD"),
            RecordFormat = GetString(element, "record_format", "TS"),
            SegmentRecord = GetBool(element, "segment_record"),
            SegmentTime = GetString(element, "segment_time", "1800"),
            MonitorStatus = GetBool(element, "monitor_status", true),
            ScheduledRecording = GetBool(element, "scheduled_recording"),
            ScheduledStartTime = GetString(element, "scheduled_start_time"),
            MonitorHours = GetString(element, "monitor_hours", "5"),
            RecordingDir = GetString(element, "recording_dir"),
            EnabledMessagePush = GetBool(element, "enabled_message_push", true),
            OnlyNotifyNoRecord = GetBool(element, "only_notify_no_record"),
            FlvUseDirectDownload = GetBool(element, "flv_use_direct_download"),
            Platform = GetString(element, "platform", "未知平台"),
            PlatformKey = GetString(element, "platform_key", "unknown"),
            StatusInfo = GetString(element, "status_info", "已停止"),
            DisplayTitle = GetString(element, "display_title"),
            Title = GetString(element, "title"),
            ErrorMessage = GetString(element, "error_message"),
            CreatedAt = GetString(element, "created_at"),
            UpdatedAt = GetString(element, "updated_at"),
            LastCheckedAt = GetString(element, "last_checked_at"),
            LiveTitle = GetString(element, "live_title"),
            RecordUrl = GetString(element, "record_url"),
            LatestOutputPath = GetString(element, "latest_output_path"),
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
