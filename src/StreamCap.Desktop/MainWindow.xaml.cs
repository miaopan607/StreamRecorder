using System.Collections.Specialized;
using System.ComponentModel;
using System.Diagnostics;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Media;
using StreamCap.Desktop.Models;
using StreamCap.Desktop.Services;
using StreamCap.Desktop.ViewModels;

namespace StreamCap.Desktop;

public partial class MainWindow : global::System.Windows.Window
{
    private readonly MainViewModel _viewModel = new();
    private readonly UiStateStore _uiStateStore = new();
    private TrayService? _trayService;
    private bool _allowExit;
    private string? _editingJobId;
    private ContextMenu? _columnMenu;

    public MainWindow()
    {
        InitializeComponent();
        DataContext = _viewModel;
        PreviewKeyDown += MainWindow_PreviewKeyDown;
        Loaded += MainWindow_Loaded;
        Closing += MainWindow_Closing;
        StateChanged += MainWindow_StateChanged;
        _viewModel.Jobs.CollectionChanged += Jobs_CollectionChanged;
        _viewModel.DesktopNotificationRequested += OnDesktopNotificationRequested;
    }

    private async void MainWindow_Loaded(object sender, global::System.Windows.RoutedEventArgs e)
    {
        try
        {
            _trayService = new TrayService(this, ExitFromTray);
            await _viewModel.InitializeAsync();
            ApplySavedColumnVisibility();
            HideEditorOverlay();
            UpdateToggleSelectAllButtonText();
        }
        catch (Exception ex)
        {
            global::System.Windows.MessageBox.Show(this, ex.Message, "核心服务启动失败", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Error);
        }
    }

    private async void MainWindow_Closing(object? sender, CancelEventArgs e)
    {
        if (!_allowExit && _viewModel.Settings.MinimizeToTrayOnClose)
        {
            e.Cancel = true;
            HideToTray("程序已缩到托盘。双击托盘图标可恢复窗口。", normalizeWindowState: false);
            return;
        }

        Loaded -= MainWindow_Loaded;
        Closing -= MainWindow_Closing;
        StateChanged -= MainWindow_StateChanged;
        PreviewKeyDown -= MainWindow_PreviewKeyDown;
        _viewModel.Jobs.CollectionChanged -= Jobs_CollectionChanged;
        _viewModel.DesktopNotificationRequested -= OnDesktopNotificationRequested;
        _trayService?.Dispose();
        await _viewModel.ShutdownAsync();
    }

    private void MainWindow_StateChanged(object? sender, EventArgs e)
    {
        if (WindowState == global::System.Windows.WindowState.Minimized && _viewModel.Settings.MinimizeToTrayOnMinimize)
        {
            HideToTray("程序已最小化到托盘。双击托盘图标可恢复窗口。", normalizeWindowState: false);
        }
    }

    private void OnDesktopNotificationRequested(string title, string message)
    {
        Dispatcher.Invoke(() => _trayService?.ShowNotification(title, message));
    }

    private void ExitFromTray()
    {
        Dispatcher.Invoke(() =>
        {
            _allowExit = true;
            Close();
        });
    }

    private void HideToTray(string message, bool normalizeWindowState)
    {
        if (normalizeWindowState && WindowState == global::System.Windows.WindowState.Minimized)
        {
            WindowState = global::System.Windows.WindowState.Normal;
        }

        _trayService?.MinimizeToTray(message);
    }

    private async void AddJobButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        ResetEditorForNewJob();
        await Task.CompletedTask;
    }

    private async void EditJobButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        var job = _viewModel.SelectedJob ?? GetSelectedJobs().FirstOrDefault();
        if (job is null)
        {
            global::System.Windows.MessageBox.Show(this, "请先选择一个任务。", "未选择任务", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Information);
            return;
        }

        LoadJobIntoEditor(job);
        await Task.CompletedTask;
    }

    private async void DeleteJobButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        var jobs = GetSelectedJobs();
        if (jobs.Count == 0)
        {
            return;
        }

        var answer = global::System.Windows.MessageBox.Show(this, $"确定删除选中的 {jobs.Count} 个任务吗？", "确认删除", global::System.Windows.MessageBoxButton.YesNo, global::System.Windows.MessageBoxImage.Warning);
        if (answer != global::System.Windows.MessageBoxResult.Yes)
        {
            return;
        }

        await RunUiActionAsync(() => _viewModel.DeleteJobsAsync(jobs));
    }

    private async void StartMonitoringButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        var jobs = GetSelectedJobsOrAll();
        await RunUiActionAsync(() => _viewModel.SetMonitoringAsync(jobs, true));
    }

    private async void StopMonitoringButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        var jobs = GetSelectedJobsOrAll();
        await RunUiActionAsync(() => _viewModel.SetMonitoringAsync(jobs, false));
    }

    private async void RecheckJobButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        var jobs = GetSelectedJobsOrAll();
        await RunUiActionAsync(() => _viewModel.RecheckJobsAsync(jobs));
    }

    private async void RefreshButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        await RunUiActionAsync(_viewModel.ReloadSnapshotAsync);
    }

    private async void SaveSettingsButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        await RunUiActionAsync(_viewModel.SaveSettingsAsync);
    }

    private async void SaveCookiesButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        await RunUiActionAsync(_viewModel.SaveCookiesAsync);
    }

    private async void SaveAccountsButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        await RunUiActionAsync(_viewModel.SaveAccountsAsync);
    }

    private async void ReloadCookiesButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        await RunUiActionAsync(_viewModel.LoadCookiesAsync);
    }

    private async void ReloadAccountsButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        await RunUiActionAsync(_viewModel.LoadAccountsAsync);
    }

    private async void PingButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        await RunUiActionAsync(_viewModel.PingAsync);
    }

    private async void RefreshDependenciesButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        await RunUiActionAsync(_viewModel.RefreshDependenciesAsync);
    }

    private async void CheckUpdatesButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        try
        {
            var result = await _viewModel.CheckUpdatesAsync();
            if (!string.IsNullOrWhiteSpace(result.Error))
            {
                global::System.Windows.MessageBox.Show(this, result.Error, "检查更新失败", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Warning);
                return;
            }

            if (!result.HasUpdate)
            {
                global::System.Windows.MessageBox.Show(this, "当前已经是最新版本。", "检查更新", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Information);
                return;
            }

            var notes = string.IsNullOrWhiteSpace(result.ReleaseNotes) ? "无更新说明" : result.ReleaseNotes;
            var answer = global::System.Windows.MessageBox.Show(
                this,
                $"发现新版本 {result.LatestVersion}\n\n当前版本：{result.CurrentVersion}\n来源：{result.Source}\n\n更新说明：\n{notes}\n\n是否打开下载页面？",
                "发现更新",
                global::System.Windows.MessageBoxButton.YesNo,
                global::System.Windows.MessageBoxImage.Information);

            if (answer == global::System.Windows.MessageBoxResult.Yes && !string.IsNullOrWhiteSpace(result.DownloadUrl))
            {
                Process.Start(new ProcessStartInfo { FileName = result.DownloadUrl, UseShellExecute = true });
            }
        }
        catch (Exception ex)
        {
            global::System.Windows.MessageBox.Show(this, ex.Message, "检查更新失败", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Error);
        }
    }

    private async void ImportLegacyButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        var answer = global::System.Windows.MessageBox.Show(this, "导入旧版配置会覆盖当前任务、Cookies、账号和设置，是否继续？", "确认导入", global::System.Windows.MessageBoxButton.YesNo, global::System.Windows.MessageBoxImage.Warning);
        if (answer != global::System.Windows.MessageBoxResult.Yes)
        {
            return;
        }

        await RunUiActionAsync(_viewModel.ImportLegacyAsync);
        ResetEditorForNewJob();
    }

    private void PickLegacyFolderButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        using var dialog = new global::System.Windows.Forms.FolderBrowserDialog
        {
            Description = "选择旧版 StreamCap 项目根目录",
            UseDescriptionForTitle = true,
            InitialDirectory = _viewModel.LegacyImportPath,
        };

        if (dialog.ShowDialog() == global::System.Windows.Forms.DialogResult.OK)
        {
            _viewModel.LegacyImportPath = dialog.SelectedPath;
        }
    }

    private void OpenDataFolderButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        try
        {
            _viewModel.OpenDataFolder();
        }
        catch (Exception ex)
        {
            global::System.Windows.MessageBox.Show(this, ex.Message, "打开目录失败", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Error);
        }
    }

    private void OpenJobFolderButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        try
        {
            _viewModel.OpenJobFolder(GetSelectedJobs().FirstOrDefault());
        }
        catch (Exception ex)
        {
            global::System.Windows.MessageBox.Show(this, ex.Message, "打开目录失败", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Error);
        }
    }

    private void OpenLatestFileButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        try
        {
            var job = GetSelectedJobs().FirstOrDefault();
            if (string.IsNullOrWhiteSpace(_viewModel.ResolveLatestOutputPath(job)))
            {
                global::System.Windows.MessageBox.Show(this, "当前任务还没有可播放的录制文件。", "没有可播放文件", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Information);
                return;
            }

            _viewModel.OpenJobFile(job);
        }
        catch (Exception ex)
        {
            global::System.Windows.MessageBox.Show(this, ex.Message, "打开文件失败", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Error);
        }
    }

    private void RowEditJobButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        if (sender is global::System.Windows.Controls.Button { Tag: RecordingJob job })
        {
            _viewModel.SelectedJob = job;
            LoadJobIntoEditor(job);
        }
    }

    private async void RowRecheckJobButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        if (sender is global::System.Windows.Controls.Button { Tag: RecordingJob job })
        {
            await RunUiActionAsync(() => _viewModel.RecheckJobsAsync(new[] { job }));
        }
    }

    private async void RowStartJobButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        if (sender is global::System.Windows.Controls.Button { Tag: RecordingJob job })
        {
            await RunUiActionAsync(() => _viewModel.SetMonitoringAsync(new[] { job }, true));
        }
    }

    private async void RowStopJobButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        if (sender is global::System.Windows.Controls.Button { Tag: RecordingJob job })
        {
            await RunUiActionAsync(() => _viewModel.SetMonitoringAsync(new[] { job }, false));
        }
    }

    private void RowOpenFolderButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        if (sender is global::System.Windows.Controls.Button { Tag: RecordingJob job })
        {
            _viewModel.SelectedJob = job;
            _viewModel.OpenJobFolder(job);
        }
    }

    private void RowOpenFileButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        if (sender is global::System.Windows.Controls.Button { Tag: RecordingJob job })
        {
            _viewModel.SelectedJob = job;
            if (string.IsNullOrWhiteSpace(_viewModel.ResolveLatestOutputPath(job)))
            {
                global::System.Windows.MessageBox.Show(this, "当前任务还没有可播放的录制文件。", "没有可播放文件", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Information);
                return;
            }

            _viewModel.OpenJobFile(job);
        }
    }

    private void ToggleSelectAllJobsButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        var shouldSelectAll = _viewModel.Jobs.Any(job => !job.IsBatchSelected);
        foreach (var job in _viewModel.Jobs)
        {
            job.IsBatchSelected = shouldSelectAll;
        }

        if (!shouldSelectAll)
        {
            JobGrid.UnselectAll();
            _viewModel.SelectedJob = null;
        }

        UpdateToggleSelectAllButtonText();
    }

    private void BrowseRecordingDirButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        using var dialog = new global::System.Windows.Forms.FolderBrowserDialog
        {
            Description = "选择任务保存目录",
            UseDescriptionForTitle = true,
            InitialDirectory = string.IsNullOrWhiteSpace(RecordingDirTextBox.Text)
                ? _viewModel.GetEffectiveSavePath()
                : RecordingDirTextBox.Text,
        };

        if (dialog.ShowDialog() == global::System.Windows.Forms.DialogResult.OK)
        {
            RecordingDirTextBox.Text = dialog.SelectedPath;
        }
    }

    private void ApplyColumnOverlayButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        ApplyColumnVisibility();
        SaveColumnVisibility();
        HideColumnOverlay();
    }

    private void CancelColumnOverlayButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        HideColumnOverlay();
    }

    private void StorageParentButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        _viewModel.NavigateStorageParent();
    }

    private void StorageOpenButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        _viewModel.OpenSelectedStorageEntry();
    }

    private void StorageOpenFolderButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        _viewModel.OpenCurrentStorageFolder();
    }

    private void StorageRefreshButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        _viewModel.RefreshStorage();
    }

    private void StorageGrid_MouseDoubleClick(object sender, global::System.Windows.Input.MouseButtonEventArgs e)
    {
        _viewModel.OpenSelectedStorageEntry();
    }

    private void JobGrid_MouseDoubleClick(object sender, global::System.Windows.Input.MouseButtonEventArgs e)
    {
        if (e.ChangedButton != MouseButton.Left)
        {
            return;
        }

        if (FindVisualParent<global::System.Windows.Controls.DataGridRow>(e.OriginalSource as DependencyObject) is null)
        {
            e.Handled = true;
        }
    }

    private async void SaveEditorButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        try
        {
            if (EditorTabs.SelectedIndex == 1)
            {
                var batchJobs = ParseBatchJobs();
                if (batchJobs.Count == 0)
                {
                    throw new InvalidOperationException("没有找到有效的批量任务。\n请检查每行是否包含直播链接。\n支持格式：画质编号,链接,主播名");
                }

                await _viewModel.UpsertJobsAsync(batchJobs);
                BatchTextBox.Clear();
                HideEditorOverlay();
                return;
            }

            var job = BuildSingleJobFromEditor();
            await _viewModel.UpsertJobsAsync(new[] { job });
            HideEditorOverlay();
        }
        catch (Exception ex)
        {
            global::System.Windows.MessageBox.Show(this, ex.Message, "任务输入有误", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Warning);
        }
    }

    private void ClearEditorButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        HideEditorOverlay();
    }

    private void ResetEditorForNewJob()
    {
        _editingJobId = null;
        ShowEditorOverlay();
        EditorTabs.SelectedIndex = 0;
        EditorTitleTextBlock.Text = "添加任务";
        UrlTextBox.Text = string.Empty;
        StreamerNameTextBox.Text = string.Empty;
        RecordingDirTextBox.Text = _viewModel.Settings.LiveSavePath;
        SegmentTimeTextBox.Text = _viewModel.Settings.VideoSegmentTime;
        ScheduledStartTextBox.Text = string.Empty;
        MonitorHoursTextBox.Text = "5";
        MonitorCheckBox.IsChecked = true;
        SegmentCheckBox.IsChecked = _viewModel.Settings.SegmentedRecordingEnabled;
        ScheduledCheckBox.IsChecked = false;
        MessagePushCheckBox.IsChecked = true;
        OnlyNotifyCheckBox.IsChecked = false;
        DirectDownloadCheckBox.IsChecked = _viewModel.Settings.FlvUseDirectDownload;
        SelectComboByTag(QualityComboBox, _viewModel.Settings.RecordQuality);
        SelectComboByTag(FormatComboBox, _viewModel.Settings.VideoFormat);
    }

    private void LoadJobIntoEditor(RecordingJob job)
    {
        _editingJobId = job.Id;
        ShowEditorOverlay();
        EditorTabs.SelectedIndex = 0;
        EditorTitleTextBlock.Text = $"编辑任务：{job.DisplayTitle}";
        UrlTextBox.Text = job.Url;
        StreamerNameTextBox.Text = job.StreamerName;
        RecordingDirTextBox.Text = job.RecordingDir;
        SegmentTimeTextBox.Text = string.IsNullOrWhiteSpace(job.SegmentTime) ? _viewModel.Settings.VideoSegmentTime : job.SegmentTime;
        ScheduledStartTextBox.Text = job.ScheduledStartTime;
        MonitorHoursTextBox.Text = string.IsNullOrWhiteSpace(job.MonitorHours) ? "5" : job.MonitorHours;
        MonitorCheckBox.IsChecked = job.MonitorStatus;
        SegmentCheckBox.IsChecked = job.SegmentRecord;
        ScheduledCheckBox.IsChecked = job.ScheduledRecording;
        MessagePushCheckBox.IsChecked = job.EnabledMessagePush;
        OnlyNotifyCheckBox.IsChecked = job.OnlyNotifyNoRecord;
        DirectDownloadCheckBox.IsChecked = job.FlvUseDirectDownload;
        SelectComboByTag(QualityComboBox, job.Quality);
        SelectComboByTag(FormatComboBox, job.RecordFormat);
    }

    private RecordingJob BuildSingleJobFromEditor()
    {
        var url = UrlTextBox.Text.Trim();
        if (string.IsNullOrWhiteSpace(url))
        {
            throw new InvalidOperationException("直播间链接不能为空。");
        }

        return new RecordingJob
        {
            Id = _editingJobId ?? string.Empty,
            Url = url,
            StreamerName = string.IsNullOrWhiteSpace(StreamerNameTextBox.Text) ? "直播间" : StreamerNameTextBox.Text.Trim(),
            Quality = GetComboTag(QualityComboBox, "OD"),
            RecordFormat = GetComboTag(FormatComboBox, "TS"),
            SegmentRecord = SegmentCheckBox.IsChecked == true,
            SegmentTime = string.IsNullOrWhiteSpace(SegmentTimeTextBox.Text) ? _viewModel.Settings.VideoSegmentTime : SegmentTimeTextBox.Text.Trim(),
            MonitorStatus = MonitorCheckBox.IsChecked != false,
            ScheduledRecording = ScheduledCheckBox.IsChecked == true,
            ScheduledStartTime = ScheduledStartTextBox.Text.Trim(),
            MonitorHours = string.IsNullOrWhiteSpace(MonitorHoursTextBox.Text) ? "5" : MonitorHoursTextBox.Text.Trim(),
            RecordingDir = RecordingDirTextBox.Text.Trim(),
            EnabledMessagePush = MessagePushCheckBox.IsChecked != false,
            OnlyNotifyNoRecord = OnlyNotifyCheckBox.IsChecked == true,
            FlvUseDirectDownload = DirectDownloadCheckBox.IsChecked == true,
        };
    }

    private List<RecordingJob> ParseBatchJobs()
    {
        var jobs = new List<RecordingJob>();
        var seenUrls = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var lines = BatchTextBox.Text.Split(new[] { "\r\n", "\n" }, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        foreach (var line in lines)
        {
            if (!line.Contains("http", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var parts = line.Replace('，', ',').Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
            var qualityCode = "0";
            var url = string.Empty;
            var streamerName = "直播间";

            if (parts.Length >= 3)
            {
                qualityCode = parts[0];
                url = parts[1];
                streamerName = parts[2];
            }
            else if (parts.Length == 2)
            {
                if (parts[1].StartsWith("http", StringComparison.OrdinalIgnoreCase))
                {
                    qualityCode = parts[0];
                    url = parts[1];
                }
                else
                {
                    url = parts[0];
                    streamerName = parts[1];
                }
            }
            else
            {
                url = parts[0];
            }

            if (string.IsNullOrWhiteSpace(url) || !seenUrls.Add(url))
            {
                continue;
            }

            jobs.Add(new RecordingJob
            {
                Url = url,
                StreamerName = string.IsNullOrWhiteSpace(streamerName) ? "直播间" : streamerName,
                Quality = MapQualityCode(qualityCode),
                RecordFormat = _viewModel.Settings.VideoFormat,
                SegmentRecord = _viewModel.Settings.SegmentedRecordingEnabled,
                SegmentTime = _viewModel.Settings.VideoSegmentTime,
                MonitorStatus = true,
                ScheduledRecording = false,
                MonitorHours = "5",
                RecordingDir = _viewModel.Settings.LiveSavePath,
                EnabledMessagePush = true,
                OnlyNotifyNoRecord = false,
                FlvUseDirectDownload = _viewModel.Settings.FlvUseDirectDownload,
            });
        }

        return jobs;
    }

    private IReadOnlyList<RecordingJob> GetSelectedJobs()
    {
        var jobs = _viewModel.Jobs.Where(job => job.IsBatchSelected).ToList();
        if (jobs.Count > 0)
        {
            return jobs;
        }

        return _viewModel.SelectedJob is null ? Array.Empty<RecordingJob>() : new[] { _viewModel.SelectedJob };
    }

    private IReadOnlyList<RecordingJob> GetSelectedJobsOrAll()
    {
        var selected = GetSelectedJobs();
        return selected.Count > 0 ? selected : _viewModel.Jobs.ToList();
    }

    private void ShowEditorOverlay()
    {
        EditorOverlay.Visibility = global::System.Windows.Visibility.Visible;
    }

    private void HideEditorOverlay()
    {
        EditorOverlay.Visibility = global::System.Windows.Visibility.Collapsed;
        _editingJobId = null;
        BatchTextBox.Text = string.Empty;
    }

    private void ShowColumnOverlay()
    {
        ColumnTitleCheckBox.IsChecked = TitleColumn.Visibility == Visibility.Visible;
        ColumnPlatformCheckBox.IsChecked = PlatformColumn.Visibility == Visibility.Visible;
        ColumnUrlCheckBox.IsChecked = UrlColumn.Visibility == Visibility.Visible;
        ColumnStatusCheckBox.IsChecked = StatusColumn.Visibility == Visibility.Visible;
        ColumnQualityCheckBox.IsChecked = QualityColumn.Visibility == Visibility.Visible;
        ColumnFormatCheckBox.IsChecked = FormatColumn.Visibility == Visibility.Visible;
        ColumnUpdatedCheckBox.IsChecked = UpdatedAtColumn.Visibility == Visibility.Visible;
        ColumnActionsCheckBox.IsChecked = ActionsColumn.Visibility == Visibility.Visible;
        ColumnOverlay.Visibility = Visibility.Visible;
    }

    private void HideColumnOverlay()
    {
        ColumnOverlay.Visibility = Visibility.Collapsed;
    }

    private void ApplyColumnVisibility()
    {
        TitleColumn.Visibility = Visibility.Visible;
        ActionsColumn.Visibility = Visibility.Visible;
        PlatformColumn.Visibility = ColumnPlatformCheckBox.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        UrlColumn.Visibility = ColumnUrlCheckBox.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        StatusColumn.Visibility = ColumnStatusCheckBox.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        QualityColumn.Visibility = ColumnQualityCheckBox.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        FormatColumn.Visibility = ColumnFormatCheckBox.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        UpdatedAtColumn.Visibility = ColumnUpdatedCheckBox.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
    }

    private void ApplySavedColumnVisibility()
    {
        var visibleColumns = _uiStateStore.LoadVisibleColumns();
        if (visibleColumns.Count == 0)
        {
            ColumnPlatformCheckBox.IsChecked = true;
            ColumnUrlCheckBox.IsChecked = true;
            ColumnStatusCheckBox.IsChecked = true;
            ColumnQualityCheckBox.IsChecked = true;
            ColumnFormatCheckBox.IsChecked = true;
            ColumnUpdatedCheckBox.IsChecked = true;
            ApplyColumnVisibility();
            return;
        }

        ColumnPlatformCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(PlatformColumn), true);
        ColumnUrlCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(UrlColumn), true);
        ColumnStatusCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(StatusColumn), true);
        ColumnQualityCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(QualityColumn), true);
        ColumnFormatCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(FormatColumn), true);
        ColumnUpdatedCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(UpdatedAtColumn), true);
        ApplyColumnVisibility();
    }

    private void SaveColumnVisibility()
    {
        _uiStateStore.SaveVisibleColumns(new Dictionary<string, bool>
        {
            [nameof(PlatformColumn)] = PlatformColumn.Visibility == Visibility.Visible,
            [nameof(UrlColumn)] = UrlColumn.Visibility == Visibility.Visible,
            [nameof(StatusColumn)] = StatusColumn.Visibility == Visibility.Visible,
            [nameof(QualityColumn)] = QualityColumn.Visibility == Visibility.Visible,
            [nameof(FormatColumn)] = FormatColumn.Visibility == Visibility.Visible,
            [nameof(UpdatedAtColumn)] = UpdatedAtColumn.Visibility == Visibility.Visible,
        });
    }

    private void AutoSizeAllColumns()
    {
        foreach (var column in JobGrid.Columns)
        {
            if (column.Visibility != Visibility.Visible)
            {
                continue;
            }

            column.Width = new DataGridLength(1, DataGridLengthUnitType.Auto);
        }
    }

    private void JobGrid_PreviewMouseLeftButtonDown(object sender, global::System.Windows.Input.MouseButtonEventArgs e)
    {
        if (e.OriginalSource is DependencyObject source)
        {
            if (FindVisualParent<global::System.Windows.Controls.Button>(source) is not null
                || FindVisualParent<global::System.Windows.Controls.CheckBox>(source) is not null)
            {
                return;
            }

            var row = FindVisualParent<global::System.Windows.Controls.DataGridRow>(source);
            if (row?.Item is RecordingJob job)
            {
                if (e.ChangedButton == MouseButton.Left && e.ClickCount >= 2)
                {
                    _viewModel.SelectedJob = job;
                    JobGrid.SelectedItem = job;
                    job.IsBatchSelected = true;
                    LoadJobIntoEditor(job);
                    UpdateToggleSelectAllButtonText();
                    e.Handled = true;
                    return;
                }

                job.IsBatchSelected = !job.IsBatchSelected;
                if (job.IsBatchSelected)
                {
                    _viewModel.SelectedJob = job;
                    JobGrid.SelectedItem = job;
                }
                else
                {
                    if (ReferenceEquals(_viewModel.SelectedJob, job))
                    {
                        _viewModel.SelectedJob = null;
                    }
                    Focus();
                    JobGrid.UnselectAll();
                }

                UpdateToggleSelectAllButtonText();
                e.Handled = true;
            }
        }
    }

    private void JobGrid_PreviewMouseRightButtonUp(object sender, MouseButtonEventArgs e)
    {
        if (FindVisualParent<DataGridColumnHeader>(e.OriginalSource as DependencyObject) is not DataGridColumnHeader header)
        {
            return;
        }

        _columnMenu ??= BuildColumnMenu();
        _columnMenu.PlacementTarget = header;
        _columnMenu.Placement = PlacementMode.Bottom;
        _columnMenu.IsOpen = true;
        e.Handled = true;
    }

    private ContextMenu BuildColumnMenu()
    {
        var menu = new ContextMenu();
        var chooseColumnsItem = new MenuItem { Header = "选择列..." };
        chooseColumnsItem.Click += (_, _) => ShowColumnOverlay();
        var autoSizeItem = new MenuItem { Header = "自动调整全部列宽" };
        autoSizeItem.Click += (_, _) => AutoSizeAllColumns();
        menu.Items.Add(chooseColumnsItem);
        menu.Items.Add(autoSizeItem);
        return menu;
    }

    private void UpdateToggleSelectAllButtonText()
    {
        var allSelected = _viewModel.Jobs.Count > 0 && _viewModel.Jobs.All(job => job.IsBatchSelected);
        ToggleSelectAllTextBlock.Text = allSelected ? "取消全选" : "全选";
        ToggleSelectAllIconTextBlock.Text = allSelected ? new string((char)0xE894, 1) : new string((char)0xE73E, 1);
    }

    private static T? FindVisualParent<T>(DependencyObject? child) where T : DependencyObject
    {
        while (child is not null)
        {
            if (child is T match)
            {
                return match;
            }

            child = VisualTreeHelper.GetParent(child);
        }

        return null;
    }

    private static string MapQualityCode(string qualityCode)
    {
        return qualityCode.Trim() switch
        {
            "0" => "OD",
            "1" => "UHD",
            "2" => "HD",
            "3" => "SD",
            "4" => "LD",
            _ => "OD",
        };
    }

    private static void SelectComboByTag(global::System.Windows.Controls.ComboBox comboBox, string? value)
    {
        foreach (var item in comboBox.Items.OfType<global::System.Windows.Controls.ComboBoxItem>())
        {
            if (string.Equals(item.Tag?.ToString(), value, StringComparison.OrdinalIgnoreCase))
            {
                comboBox.SelectedItem = item;
                return;
            }
        }

        comboBox.SelectedIndex = 0;
    }

    private static string GetComboTag(global::System.Windows.Controls.ComboBox comboBox, string fallback)
    {
        return comboBox.SelectedItem is global::System.Windows.Controls.ComboBoxItem item
            ? item.Tag?.ToString() ?? fallback
            : fallback;
    }

    private async Task RunUiActionAsync(Func<Task> action)
    {
        try
        {
            await action();
        }
        catch (Exception ex)
        {
            global::System.Windows.MessageBox.Show(this, ex.Message, "操作失败", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Error);
        }
    }

    private void Jobs_CollectionChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        if (e.OldItems is not null)
        {
            foreach (var item in e.OldItems.OfType<RecordingJob>())
            {
                item.PropertyChanged -= Job_PropertyChanged;
            }
        }

        if (e.NewItems is not null)
        {
            foreach (var item in e.NewItems.OfType<RecordingJob>())
            {
                item.PropertyChanged += Job_PropertyChanged;
            }
        }

        UpdateToggleSelectAllButtonText();
    }

    private void Job_PropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(RecordingJob.IsBatchSelected))
        {
            UpdateToggleSelectAllButtonText();
        }
    }

    private void MainWindow_PreviewKeyDown(object sender, global::System.Windows.Input.KeyEventArgs e)
    {
        if (e.Key == Key.Escape && EditorOverlay.Visibility == Visibility.Visible)
        {
            HideEditorOverlay();
            e.Handled = true;
            return;
        }

        if (e.Key == Key.Escape && ColumnOverlay.Visibility == Visibility.Visible)
        {
            HideColumnOverlay();
            e.Handled = true;
        }
    }
}
