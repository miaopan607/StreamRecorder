namespace StreamCap.Desktop.Models;

public sealed class StorageEntry
{
    public string Name { get; set; } = string.Empty;

    public string FullPath { get; set; } = string.Empty;

    public bool IsDirectory { get; set; }

    public string Type => IsDirectory ? "目录" : "文件";
}
