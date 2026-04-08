using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Windows;
using StreamRecorder.Desktop.Infrastructure;
using StreamRecorder.Desktop.Models;
using StreamRecorder.Desktop.Services;

namespace StreamRecorder.Desktop.ViewModels;

public sealed class MainViewModel : ObservableObject, IAsyncDisposable
{
    private static readonly (string Name, string Command, string VersionArguments)[] LocalDependencyDefinitions =
    {
        ("PYTHON", "python", "--version"),
        ("FFMPEG", "ffmpeg", "-version"),
        ("NODE", "node", "--version"),
    };

    private static readonly string[] CookiePlatforms =
    {
        "douyin", "tiktok", "kuaishou", "huya", "douyu", "yy", "bilibili", "xhs", "bigo", "blued",
        "soop", "netease", "qiandurebo", "pandalive", "maoerfm", "winktv", "flextv", "look", "popkontv",
        "twitcasting", "baidu", "weibo", "kugou", "twitch", "liveme", "huajiao", "liuxing", "showroom",
        "acfun", "changliao", "yinbo", "inke", "zhihu", "chzzk", "haixiu", "vvxq", "17live", "lang",
        "piaopiao", "6room", "lehai", "catshow", "shopee", "youtube", "taobao", "jd"
    };

    private static readonly string[] AccountKeys =
    {
        "sooplive_username",
        "sooplive_password",
        "flextv_username",
        "flextv_password",
        "popkontv_username",
        "popkontv_password",
        "twitcasting_account_type",
        "twitcasting_username",
        "twitcasting_password"
    };

    private readonly WorkerClient _workerClient = new();
    private readonly AutoStartService _autoStartService = new();
    private CancellationTokenSource? _settingsAutoSaveCts;
    private RecordingJob? _selectedJob;
    private StorageEntry? _selectedStorageEntry;
    private string _connectionStateText = "正在启动核心服务";
    private string _workerVersionText = "StreamRecorder 核心服务未连接";
    private string _jobCountText = "0 个任务";
    private string _monitoringCountText = "0 个监控中";
    private string _currentStoragePath = string.Empty;
    private string _dependencySummaryText = "尚未检查依赖";
    private bool _startWithWindows;
    private bool _settingsAutoSaveEnabled;
    private bool _suppressSettingsAutoSave;

    public MainViewModel()
    {
        _workerClient.EventReceived += HandleWorkerEventAsync;
        _workerClient.LogReceived += AppendLog;
        _startWithWindows = _autoStartService.IsEnabled();
        Settings.PropertyChanged += Settings_PropertyChanged;
        PropertyChanged += MainViewModel_PropertyChanged;
        SeedDefaultEntries();
        Settings.LiveSavePath = GetEffectiveSavePath();
        RefreshStorage();
    }

    public event Action<string, string>? DesktopNotificationRequested;

    public ObservableCollection<RecordingJob> Jobs { get; } = new();

    public ObservableCollection<string> Logs { get; } = new();

    public ObservableCollection<ConfigEntry> CookieEntries { get; } = new();

    public ObservableCollection<ConfigEntry> AccountEntries { get; } = new();

    public ObservableCollection<StorageEntry> StorageItems { get; } = new();

    public ObservableCollection<DependencyStatus> Dependencies { get; } = new();

    public CoreSettings Settings { get; } = new();

    public string RepositoryRoot => ProjectPaths.RepositoryRoot;

    public string WorkerRoot => ProjectPaths.WorkerRoot;

    public string WorkerDataRoot => ProjectPaths.WorkerDataRoot;

    public RecordingJob? SelectedJob
    {
        get => _selectedJob;
        set => SetProperty(ref _selectedJob, value);
    }

    public StorageEntry? SelectedStorageEntry
    {
        get => _selectedStorageEntry;
        set => SetProperty(ref _selectedStorageEntry, value);
    }

    public string ConnectionStateText
    {
        get => _connectionStateText;
        set => SetProperty(ref _connectionStateText, value);
    }

    public string WorkerVersionText
    {
        get => _workerVersionText;
        set => SetProperty(ref _workerVersionText, value);
    }

    public string JobCountText
    {
        get => _jobCountText;
        set => SetProperty(ref _jobCountText, value);
    }

    public string MonitoringCountText
    {
        get => _monitoringCountText;
        set => SetProperty(ref _monitoringCountText, value);
    }

    public string CurrentStoragePath
    {
        get => _currentStoragePath;
        set => SetProperty(ref _currentStoragePath, value);
    }

    public string DependencySummaryText
    {
        get => _dependencySummaryText;
        set => SetProperty(ref _dependencySummaryText, value);
    }

    public bool StartWithWindows
    {
        get => _startWithWindows;
        set => SetProperty(ref _startWithWindows, value);
    }

    public async Task InitializeAsync()
    {
        ConnectionStateText = "正在启动核心服务";
        _suppressSettingsAutoSave = true;
        StartWithWindows = _autoStartService.IsEnabled();
        try
        {
            await _workerClient.StartAsync();
            await ReloadSnapshotAsync();
            await ReloadConfigEditorsAsync();
            await RefreshDependenciesAsync();
            RefreshStorage();
            _settingsAutoSaveEnabled = true;
        }
        catch
        {
            WorkerVersionText = "StreamRecorder 核心服务未连接";
            ConnectionStateText = "未连接 | Python 核心服务不可用";
            await RefreshDependenciesAsync();
            RefreshStorage();
            throw;
        }
        finally
        {
            _suppressSettingsAutoSave = false;
        }
    }

    public async Task ReloadSnapshotAsync()
    {
        var snapshot = await _workerClient.CallAsync("get_snapshot");
        await ApplySnapshotAsync(snapshot);
    }

    public async Task ReloadConfigEditorsAsync()
    {
        await LoadCookiesAsync();
        await LoadAccountsAsync();
    }

    public async Task UpsertJobsAsync(IEnumerable<RecordingJob> jobs)
    {
        var payload = new Dictionary<string, object?>
        {
            ["jobs"] = jobs.Select(job => job.ToWorkerPayload()).ToArray(),
        };
        await _workerClient.CallAsync("jobs.upsert", payload);
    }

    public async Task DeleteSelectedJobAsync()
    {
        if (SelectedJob is null)
        {
            return;
        }

        await DeleteJobsAsync(new[] { SelectedJob });
    }

    public async Task DeleteJobsAsync(IEnumerable<RecordingJob> jobs)
    {
        var ids = jobs.Select(job => job.Id).Where(id => !string.IsNullOrWhiteSpace(id)).Distinct().ToArray();
        if (ids.Length == 0)
        {
            return;
        }

        await _workerClient.CallAsync("jobs.delete", new Dictionary<string, object?>
        {
            ["ids"] = ids,
        });
    }

    public async Task SetMonitoringAsync(bool enabled)
    {
        if (SelectedJob is null)
        {
            return;
        }

        await SetMonitoringAsync(new[] { SelectedJob }, enabled);
    }

    public async Task SetMonitoringAsync(IEnumerable<RecordingJob> jobs, bool enabled)
    {
        var ids = jobs.Select(job => job.Id).Where(id => !string.IsNullOrWhiteSpace(id)).Distinct().ToArray();
        if (ids.Length == 0)
        {
            return;
        }

        var method = enabled ? "jobs.start_monitoring" : "jobs.stop_monitoring";
        await _workerClient.CallAsync(method, new Dictionary<string, object?>
        {
            ["ids"] = ids,
        });
    }

    public async Task RecheckSelectedJobAsync()
    {
        if (SelectedJob is null)
        {
            return;
        }

        await RecheckJobsAsync(new[] { SelectedJob });
    }

    public async Task RecheckJobsAsync(IEnumerable<RecordingJob> jobs)
    {
        var ids = jobs.Select(job => job.Id).Where(id => !string.IsNullOrWhiteSpace(id)).Distinct().ToArray();
        if (ids.Length == 0)
        {
            return;
        }

        await _workerClient.CallAsync("jobs.recheck", new Dictionary<string, object?>
        {
            ["ids"] = ids,
        });
    }

    public async Task SaveSettingsAsync()
    {
        await _workerClient.CallAsync("settings.update", new Dictionary<string, object?>
        {
            ["settings"] = Settings.ToWorkerPayload(),
        });
        _autoStartService.SetEnabled(StartWithWindows);
        AppendLog("设置已保存");
        AppendLog(StartWithWindows ? "已启用开机自启" : "已关闭开机自启");
        RefreshStorage();
    }

    private void Settings_PropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        QueueSettingsAutoSave();
    }

    private void MainViewModel_PropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(StartWithWindows))
        {
            QueueSettingsAutoSave();
        }
    }

    private void QueueSettingsAutoSave()
    {
        if (!_settingsAutoSaveEnabled || _suppressSettingsAutoSave)
        {
            return;
        }

        _settingsAutoSaveCts?.Cancel();
        _settingsAutoSaveCts?.Dispose();
        var cts = new CancellationTokenSource();
        _settingsAutoSaveCts = cts;
        _ = AutoSaveSettingsAsync(cts.Token);
    }

    private async Task AutoSaveSettingsAsync(CancellationToken cancellationToken)
    {
        try
        {
            await Task.Delay(500, cancellationToken);
            await SaveSettingsAsync();
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception ex)
        {
            AppendLog($"自动保存设置失败：{ex.Message}");
        }
    }

    public async Task SaveCookiesAsync()
    {
        await _workerClient.CallAsync("cookies.update", new Dictionary<string, object?>
        {
            ["cookies"] = CookieEntries.ToDictionary(item => item.Key, item => item.Value),
        });
        AppendLog("Cookies 已保存");
    }

    public async Task SaveAccountsAsync()
    {
        await _workerClient.CallAsync("accounts.update", new Dictionary<string, object?>
        {
            ["accounts"] = AccountEntries.ToDictionary(item => item.Key, item => item.Value),
        });
        AppendLog("账号配置已保存");
    }

    public async Task LoadCookiesAsync()
    {
        var result = await _workerClient.CallAsync("cookies.get");
        var data = result.GetProperty("cookies");
        await global::System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
        {
            SeedCookieEntries(data);
        });
    }

    public async Task LoadAccountsAsync()
    {
        var result = await _workerClient.CallAsync("accounts.get");
        var data = result.GetProperty("accounts");
        await global::System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
        {
            SeedAccountEntries(data);
        });
    }

    public async Task RefreshDependenciesAsync()
    {
        if (!_workerClient.IsRunning)
        {
            await RefreshLocalDependenciesAsync();
            return;
        }

        var result = await _workerClient.CallAsync("dependencies.get");
        await global::System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
        {
            Dependencies.Clear();
            foreach (var property in result.EnumerateObject())
            {
                Dependencies.Add(new DependencyStatus
                {
                    Name = property.Name.ToUpperInvariant(),
                    Available = property.Value.GetProperty("available").GetBoolean(),
                    Version = property.Value.TryGetProperty("version", out var version) ? version.GetString() ?? string.Empty : string.Empty,
                    Path = property.Value.TryGetProperty("path", out var path) ? path.GetString() ?? string.Empty : string.Empty,
                });
            }

            var missing = Dependencies.Count(item => !item.Available);
            DependencySummaryText = missing == 0 ? "依赖检查通过" : $"有 {missing} 个依赖未就绪";
        });
    }

    private async Task RefreshLocalDependenciesAsync()
    {
        var dependencies = new List<DependencyStatus>();
        foreach (var definition in LocalDependencyDefinitions)
        {
            dependencies.Add(await ProbeDependencyAsync(definition.Name, definition.Command, definition.VersionArguments));
        }

        await global::System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
        {
            Dependencies.Clear();
            foreach (var dependency in dependencies)
            {
                Dependencies.Add(dependency);
            }

            var missing = Dependencies.Count(item => !item.Available);
            DependencySummaryText = missing == 0 ? "依赖检查通过（本地检测）" : $"有 {missing} 个依赖未就绪（本地检测）";
        });
    }

    private static async Task<DependencyStatus> ProbeDependencyAsync(string name, string command, string versionArguments)
    {
        var path = ResolveCommandPath(command);
        if (string.IsNullOrWhiteSpace(path))
        {
            return new DependencyStatus
            {
                Name = name,
                Available = false,
                Version = string.Empty,
                Path = string.Empty,
            };
        }

        var version = await ReadCommandVersionAsync(path, versionArguments, name);

        return new DependencyStatus
        {
            Name = name,
            Available = !string.IsNullOrWhiteSpace(version),
            Version = version,
            Path = path,
        };
    }

    private static async Task<string> ReadCommandVersionAsync(string executablePath, string arguments, string name)
    {
        try
        {
            using var process = new Process
            {
                StartInfo = new ProcessStartInfo
                {
                    FileName = executablePath,
                    Arguments = arguments,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    UseShellExecute = false,
                    CreateNoWindow = true,
                },
            };

            process.Start();
            var stdoutTask = process.StandardOutput.ReadToEndAsync();
            var stderrTask = process.StandardError.ReadToEndAsync();
            await process.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(3));
            var output = ((await stdoutTask) + "\n" + (await stderrTask)).Trim();

            if (process.ExitCode != 0 || LooksLikeMissingPythonAlias(output))
            {
                return string.Empty;
            }

            return SimplifyVersionText(output, name);
        }
        catch
        {
            return string.Empty;
        }
    }

    private static string SimplifyVersionText(string output, string name)
    {
        if (string.IsNullOrWhiteSpace(output))
        {
            return string.Empty;
        }

        var firstLine = output.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries).FirstOrDefault()?.Trim() ?? string.Empty;
        return name switch
        {
            "FFMPEG" when firstLine.StartsWith("ffmpeg version ", StringComparison.OrdinalIgnoreCase) => firstLine[15..].Split(' ').FirstOrDefault() ?? string.Empty,
            "NODE" when firstLine.StartsWith("v", StringComparison.OrdinalIgnoreCase) => firstLine[1..],
            "PYTHON" when firstLine.StartsWith("Python ", StringComparison.OrdinalIgnoreCase) => firstLine[7..],
            _ => firstLine,
        };
    }

    private static bool LooksLikeMissingPythonAlias(string output)
    {
        if (string.IsNullOrWhiteSpace(output))
        {
            return false;
        }

        return output.Contains("Python was not found", StringComparison.OrdinalIgnoreCase)
            || output.Contains("Microsoft Store", StringComparison.OrdinalIgnoreCase)
            || output.Contains("App Installer", StringComparison.OrdinalIgnoreCase);
    }

    private static string? ResolveCommandPath(string command)
    {
        var pathValue = Environment.GetEnvironmentVariable("PATH") ?? string.Empty;
        var searchExtensions = OperatingSystem.IsWindows()
            ? (Environment.GetEnvironmentVariable("PATHEXT") ?? ".EXE;.CMD;.BAT;.COM")
                .Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            : new[] { string.Empty };

        var hasExtension = Path.HasExtension(command);
        foreach (var directory in pathValue.Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            if (hasExtension)
            {
                var candidate = Path.Combine(directory, command);
                if (File.Exists(candidate))
                {
                    return candidate;
                }

                continue;
            }

            foreach (var extension in searchExtensions)
            {
                var candidate = Path.Combine(directory, command + extension.ToLowerInvariant());
                if (File.Exists(candidate))
                {
                    return candidate;
                }

                candidate = Path.Combine(directory, command + extension.ToUpperInvariant());
                if (File.Exists(candidate))
                {
                    return candidate;
                }
            }
        }

        return null;
    }

    public async Task PingAsync()
    {
        var result = await _workerClient.CallAsync("health.ping");
        AppendLog($"核心服务响应正常：{result.GetProperty("utc").GetString()}");
    }

    public void OpenDataFolder()
    {
        Directory.CreateDirectory(ProjectPaths.WorkerDataRoot);
        Process.Start(new ProcessStartInfo
        {
            FileName = "explorer.exe",
            Arguments = $"\"{ProjectPaths.WorkerDataRoot}\"",
            UseShellExecute = true,
        });
    }

    public void RefreshStorage()
    {
        var root = GetEffectiveSavePath();

        if (string.IsNullOrWhiteSpace(Settings.LiveSavePath))
        {
            Settings.LiveSavePath = root;
        }

        if (string.IsNullOrWhiteSpace(CurrentStoragePath) || !Directory.Exists(CurrentStoragePath))
        {
            CurrentStoragePath = root;
        }

        if (!Directory.Exists(CurrentStoragePath))
        {
            Directory.CreateDirectory(CurrentStoragePath);
        }

        StorageItems.Clear();
        var directoryInfo = new DirectoryInfo(CurrentStoragePath);
        foreach (var item in directoryInfo.GetDirectories().OrderBy(x => x.Name))
        {
            StorageItems.Add(new StorageEntry { Name = item.Name, FullPath = item.FullName, IsDirectory = true });
        }
        foreach (var item in directoryInfo.GetFiles().OrderBy(x => x.Name))
        {
            StorageItems.Add(new StorageEntry { Name = item.Name, FullPath = item.FullName, IsDirectory = false });
        }
    }

    public void OpenSelectedStorageEntry()
    {
        if (SelectedStorageEntry is null)
        {
            return;
        }

        if (SelectedStorageEntry.IsDirectory)
        {
            CurrentStoragePath = SelectedStorageEntry.FullPath;
            RefreshStorage();
            return;
        }

        Process.Start(new ProcessStartInfo
        {
            FileName = SelectedStorageEntry.FullPath,
            UseShellExecute = true,
        });
    }

    public void NavigateStorageParent()
    {
        var parent = Directory.GetParent(CurrentStoragePath);
        if (parent is null)
        {
            return;
        }

        CurrentStoragePath = parent.FullName;
        RefreshStorage();
    }

    public void OpenCurrentStorageFolder()
    {
        if (!Directory.Exists(CurrentStoragePath))
        {
            Directory.CreateDirectory(CurrentStoragePath);
        }

        Process.Start(new ProcessStartInfo
        {
            FileName = "explorer.exe",
            Arguments = $"\"{CurrentStoragePath}\"",
            UseShellExecute = true,
        });
    }

    public void OpenSelectedJobFolder()
    {
        if (SelectedJob is null)
        {
            return;
        }

        OpenJobFolder(SelectedJob);
    }

    public void OpenJobFolder(RecordingJob? job)
    {
        if (job is null || string.IsNullOrWhiteSpace(job.RecordingDir))
        {
            return;
        }

        Process.Start(new ProcessStartInfo
        {
            FileName = "explorer.exe",
            Arguments = $"\"{job.RecordingDir}\"",
            UseShellExecute = true,
        });
    }

    public void OpenSelectedJobFile()
    {
        if (SelectedJob is null)
        {
            return;
        }

        OpenJobFile(SelectedJob);
    }

    public void OpenJobFile(RecordingJob? job)
    {
        var path = ResolveLatestOutputPath(job);
        if (string.IsNullOrWhiteSpace(path))
        {
            return;
        }

        Process.Start(new ProcessStartInfo
        {
            FileName = path,
            UseShellExecute = true,
        });
    }

    public string GetEffectiveSavePath()
    {
        return string.IsNullOrWhiteSpace(Settings.LiveSavePath)
            ? Path.Combine(ProjectPaths.RepositoryRoot, "runtime", "recordings")
            : ProjectPaths.ResolveRepositoryPath(Settings.LiveSavePath);
    }

    public string? ResolveLatestOutputPath(RecordingJob? job)
    {
        if (job is null || string.IsNullOrWhiteSpace(job.LatestOutputPath))
        {
            return null;
        }

        if (File.Exists(job.LatestOutputPath))
        {
            return job.LatestOutputPath;
        }

        var outputPath = job.LatestOutputPath;
        if (!outputPath.Contains("%03d", StringComparison.OrdinalIgnoreCase))
        {
            return null;
        }

        var directory = Path.GetDirectoryName(outputPath);
        if (string.IsNullOrWhiteSpace(directory) || !Directory.Exists(directory))
        {
            return null;
        }

        var fileName = Path.GetFileName(outputPath);
        var pattern = fileName.Replace("%03d", "*");
        var latestFile = Directory.GetFiles(directory, pattern)
            .OrderByDescending(File.GetLastWriteTimeUtc)
            .FirstOrDefault();
        return latestFile;
    }

    private async Task HandleWorkerEventAsync(string name, JsonElement body)
    {
        switch (name)
        {
            case "snapshot_changed":
                await ApplySnapshotAsync(body);
                break;
            case "core_health":
                await global::System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
                {
                    WorkerVersionText = $"StreamRecorder 核心服务 {body.GetProperty("version").GetString()} | Python {body.GetProperty("python_version").GetString()}";
                    var ffmpegReady = body.GetProperty("ffmpeg_available").GetBoolean() ? "FFmpeg 已就绪" : "FFmpeg 未找到";
                    ConnectionStateText = $"已连接 | {ffmpegReady}";
                });
                break;
            case "desktop_notification":
                DesktopNotificationRequested?.Invoke(GetString(body, "title"), GetString(body, "message"));
                break;
        }
    }

    private async Task ApplySnapshotAsync(JsonElement snapshot)
    {
        await global::System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
        {
            if (snapshot.TryGetProperty("app", out var app))
            {
                WorkerVersionText = $"StreamRecorder 核心服务 {app.GetProperty("version").GetString()} | Python {app.GetProperty("python_version").GetString()}";
                var ffmpegReady = app.GetProperty("ffmpeg_available").GetBoolean() ? "FFmpeg 已就绪" : "FFmpeg 未找到";
                ConnectionStateText = $"已连接 | {ffmpegReady}";
            }

            if (snapshot.TryGetProperty("settings", out var settings))
            {
                _suppressSettingsAutoSave = true;
                try
                {
                    Settings.LoadFromJson(settings);
                }
                finally
                {
                    _suppressSettingsAutoSave = false;
                }
            }

            Jobs.Clear();
            if (snapshot.TryGetProperty("jobs", out var jobsElement))
            {
                foreach (var jobElement in jobsElement.EnumerateArray())
                {
                    Jobs.Add(RecordingJob.FromJson(jobElement));
                }
            }

            JobCountText = $"{Jobs.Count} 个任务";
            MonitoringCountText = $"{Jobs.Count(job => job.MonitorStatus)} 个监控中";
            RefreshStorage();
        });
    }

    private void SeedDefaultEntries()
    {
        SeedCookieEntries(default);
        SeedAccountEntries(default);
    }

    private void SeedCookieEntries(JsonElement? source)
    {
        CookieEntries.Clear();
        var values = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        if (source.HasValue && source.Value.ValueKind == JsonValueKind.Object)
        {
            foreach (var property in source.Value.EnumerateObject())
            {
                values[property.Name] = property.Value.GetString() ?? string.Empty;
            }
        }

        foreach (var key in CookiePlatforms.OrderBy(x => x))
        {
            CookieEntries.Add(new ConfigEntry { Key = key, Value = values.TryGetValue(key, out var value) ? value : string.Empty });
        }
    }

    private void SeedAccountEntries(JsonElement? source)
    {
        AccountEntries.Clear();
        var values = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        if (source.HasValue && source.Value.ValueKind == JsonValueKind.Object)
        {
            foreach (var property in source.Value.EnumerateObject())
            {
                values[property.Name] = property.Value.ValueKind == JsonValueKind.String
                    ? property.Value.GetString() ?? string.Empty
                    : property.Value.ToString();
            }
        }

        foreach (var key in AccountKeys)
        {
            AccountEntries.Add(new ConfigEntry { Key = key, Value = values.TryGetValue(key, out var value) ? value : string.Empty });
        }
    }

    private void AppendLog(string line)
    {
        global::System.Windows.Application.Current.Dispatcher.Invoke(() =>
        {
            Logs.Insert(0, $"[{DateTime.Now:HH:mm:ss}] {line}");
            while (Logs.Count > 300)
            {
                Logs.RemoveAt(Logs.Count - 1);
            }
        });
    }

    private static string GetString(JsonElement element, string propertyName)
    {
        return element.TryGetProperty(propertyName, out var property) ? property.GetString() ?? string.Empty : string.Empty;
    }

    private static bool GetBool(JsonElement element, string propertyName)
    {
        return element.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.True;
    }

    public async Task ShutdownAsync()
    {
        _settingsAutoSaveCts?.Cancel();
        _settingsAutoSaveCts?.Dispose();
        _settingsAutoSaveCts = null;
        await _workerClient.StopAsync();
    }

    public async ValueTask DisposeAsync()
    {
        await _workerClient.DisposeAsync();
    }
}
