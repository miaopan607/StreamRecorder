using System.IO;
using System.Text.Json;

namespace StreamRecorder.Desktop.Services;

public sealed class UiStateStore
{
    private readonly string _statePath = Path.Combine(ProjectPaths.WorkerDataRoot, "desktop_ui_state.json");

    public Dictionary<string, bool> LoadVisibleColumns()
    {
        return LoadPayload().VisibleColumns;
    }

    public void SaveVisibleColumns(Dictionary<string, bool> visibleColumns)
    {
        var payload = LoadPayload();
        payload.VisibleColumns = visibleColumns;
        SavePayload(payload);
    }

    public bool LoadTaskLayout(bool defaultValue)
    {
        return LoadPayload().IsCardLayout ?? defaultValue;
    }

    public void SaveTaskLayout(bool isCardLayout)
    {
        var payload = LoadPayload();
        payload.IsCardLayout = isCardLayout;
        SavePayload(payload);
    }

    private UiStatePayload LoadPayload()
    {
        try
        {
            if (!File.Exists(_statePath))
            {
                return new UiStatePayload();
            }

            var json = File.ReadAllText(_statePath);
            return JsonSerializer.Deserialize<UiStatePayload>(json) ?? new UiStatePayload();
        }
        catch
        {
            return new UiStatePayload();
        }
    }

    private void SavePayload(UiStatePayload payload)
    {
        Directory.CreateDirectory(ProjectPaths.WorkerDataRoot);
        var json = JsonSerializer.Serialize(payload, new JsonSerializerOptions { WriteIndented = true });
        File.WriteAllText(_statePath, json);
    }

    private sealed class UiStatePayload
    {
        public Dictionary<string, bool> VisibleColumns { get; set; } = new();

        public bool? IsCardLayout { get; set; }
    }
}
