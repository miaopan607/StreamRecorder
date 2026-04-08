using System.IO;
using System.Text.Json;

namespace StreamCap.Desktop.Services;

public sealed class UiStateStore
{
    private readonly string _statePath = Path.Combine(ProjectPaths.WorkerDataRoot, "desktop_ui_state.json");

    public Dictionary<string, bool> LoadVisibleColumns()
    {
        try
        {
            if (!File.Exists(_statePath))
            {
                return new Dictionary<string, bool>();
            }

            var json = File.ReadAllText(_statePath);
            var payload = JsonSerializer.Deserialize<UiStatePayload>(json);
            return payload?.VisibleColumns ?? new Dictionary<string, bool>();
        }
        catch
        {
            return new Dictionary<string, bool>();
        }
    }

    public void SaveVisibleColumns(Dictionary<string, bool> visibleColumns)
    {
        Directory.CreateDirectory(ProjectPaths.WorkerDataRoot);
        var payload = new UiStatePayload
        {
            VisibleColumns = visibleColumns,
        };
        var json = JsonSerializer.Serialize(payload, new JsonSerializerOptions { WriteIndented = true });
        File.WriteAllText(_statePath, json);
    }

    private sealed class UiStatePayload
    {
        public Dictionary<string, bool> VisibleColumns { get; set; } = new();
    }
}
