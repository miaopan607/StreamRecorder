namespace StreamCap.Desktop.Models;

public sealed class DependencyStatus
{
    public string Name { get; set; } = string.Empty;

    public bool Available { get; set; }

    public string Version { get; set; } = string.Empty;

    public string Path { get; set; } = string.Empty;
}
