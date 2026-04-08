namespace StreamCap.Desktop.Models;

public sealed class UpdateCheckResult
{
    public bool HasUpdate { get; set; }

    public string LatestVersion { get; set; } = string.Empty;

    public string CurrentVersion { get; set; } = string.Empty;

    public string ReleaseNotes { get; set; } = string.Empty;

    public string DownloadUrl { get; set; } = string.Empty;

    public string Source { get; set; } = string.Empty;

    public string Error { get; set; } = string.Empty;
}
