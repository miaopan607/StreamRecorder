using System.Collections.Specialized;
using System.ComponentModel;
using System.Diagnostics;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Media;
using StreamRecorder.Desktop.Models;
using StreamRecorder.Desktop.Services;
using StreamRecorder.Desktop.ViewModels;

namespace StreamRecorder.Desktop;

public partial class MainWindow : global::System.Windows.Window
{
    private readonly MainViewModel _viewModel = new();
    private readonly UiStateStore _uiStateStore = new();
    private readonly bool _launchToTrayOnStartup;
    private TrayService? _trayService;
    private bool _allowExit;
    private bool _isCardLayout = true;
    private string? _editingJobId;
    private ContextMenu? _columnMenu;

    public MainWindow(bool launchToTrayOnStartup = false)
    {
        _launchToTrayOnStartup = launchToTrayOnStartup;
        _isCardLayout = _uiStateStore.LoadTaskLayout(defaultValue: true);
        InitializeComponent();
        ApplyInitialWindowBounds();
        DataContext = _viewModel;
        UpdateLayoutState();
        PreviewKeyDown += MainWindow_PreviewKeyDown;
        Loaded += MainWindow_Loaded;
        Closing += MainWindow_Closing;
        StateChanged += MainWindow_StateChanged;
        _viewModel.Jobs.CollectionChanged += Jobs_CollectionChanged;
        _viewModel.DesktopNotificationRequested += OnDesktopNotificationRequested;
    }

    private async void MainWindow_Loaded(object sender, global::System.Windows.RoutedEventArgs e)
    {
        _trayService = new TrayService(this, ExitFromTray);
        ApplySavedColumnVisibility();
        HideEditorOverlay();
        UpdateLayoutState();
        UpdateToggleSelectAllButtonText();

        try
        {
            await _viewModel.InitializeAsync();
            if (_launchToTrayOnStartup)
            {
                HideToTray(null, normalizeWindowState: false);
            }
        }
        catch (Exception ex)
        {
            if (_launchToTrayOnStartup)
            {
                RestoreWindowVisibility();
            }

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

    private void HideToTray(string? message, bool normalizeWindowState)
    {
        if (normalizeWindowState && WindowState == global::System.Windows.WindowState.Minimized)
        {
            WindowState = global::System.Windows.WindowState.Normal;
        }

        _trayService?.MinimizeToTray(message);
    }

    private void RestoreWindowVisibility()
    {
        Opacity = 1;
        ShowInTaskbar = true;
        if (WindowState == global::System.Windows.WindowState.Minimized)
        {
            WindowState = global::System.Windows.WindowState.Normal;
        }
    }

    private void ApplyInitialWindowBounds()
    {
        const double edgePadding = 48;
        const double fallbackMinWidth = 720;
        const double fallbackMinHeight = 480;

        var workArea = SystemParameters.WorkArea;
        var availableWidth = Math.Max(fallbackMinWidth, workArea.Width - edgePadding);
        var availableHeight = Math.Max(fallbackMinHeight, workArea.Height - edgePadding);

        MinWidth = Math.Min(MinWidth, availableWidth);
        MinHeight = Math.Min(MinHeight, availableHeight);
        MaxWidth = availableWidth;
        MaxHeight = availableHeight;
        Width = Math.Min(Math.Max(Width, MinWidth), availableWidth);
        Height = Math.Min(Math.Max(Height, MinHeight), availableHeight);
    }

    private async void AddJobButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        ResetEditorForNewJob();
        await Task.CompletedTask;
    }

    private void ToggleLayoutButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        _isCardLayout = !_isCardLayout;
        UpdateLayoutState();
        _uiStateStore.SaveTaskLayout(_isCardLayout);
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

    private void ComboBox_PreviewMouseWheel(object sender, MouseWheelEventArgs e)
    {
        if (sender is not global::System.Windows.Controls.ComboBox comboBox || comboBox.IsDropDownOpen)
        {
            return;
        }

        e.Handled = true;
        if (VisualTreeHelper.GetParent(comboBox) is not UIElement parent)
        {
            return;
        }

        var forwardedEvent = new MouseWheelEventArgs(e.MouseDevice, e.Timestamp, e.Delta)
        {
            RoutedEvent = UIElement.MouseWheelEvent,
            Source = comboBox,
        };
        parent.RaiseEvent(forwardedEvent);
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

    private void CopyLogsButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        try
        {
            var text = string.Join(Environment.NewLine, _viewModel.Logs);
            if (string.IsNullOrWhiteSpace(text))
            {
                global::System.Windows.MessageBox.Show(this, "当前没有可复制的日志。", "复制日志", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Information);
                return;
            }

            global::System.Windows.Clipboard.SetText(text);
        }
        catch (Exception ex)
        {
            global::System.Windows.MessageBox.Show(this, ex.Message, "复制日志失败", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Error);
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

    private void RowInfoJobButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        if (sender is global::System.Windows.Controls.Button { Tag: RecordingJob job })
        {
            _viewModel.SelectedJob = job;
            ShowJobDetailsWindow(job);
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

    private async void RowToggleMonitorButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        if (sender is global::System.Windows.Controls.Button { Tag: RecordingJob job })
        {
            await RunUiActionAsync(() => _viewModel.SetMonitoringAsync(new[] { job }, !job.MonitorStatus));
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
        BatchImportTab.Visibility = Visibility.Visible;
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
        BatchImportTab.Visibility = Visibility.Collapsed;
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

    private void UpdateLayoutState()
    {
        JobGrid.Visibility = _isCardLayout ? Visibility.Collapsed : Visibility.Visible;
        CardLayoutScrollViewer.Visibility = _isCardLayout ? Visibility.Visible : Visibility.Collapsed;
        ToggleLayoutTextBlock.Text = _isCardLayout ? "列表布局" : "卡片布局";
        ToggleLayoutIconTextBlock.Text = _isCardLayout ? new string((char)0xE8A5, 1) : new string((char)0xE8A5, 1);
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
        ColumnDurationCheckBox.IsChecked = DurationColumn.Visibility == Visibility.Visible;
        ColumnSpeedCheckBox.IsChecked = SpeedColumn.Visibility == Visibility.Visible;
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
        DurationColumn.Visibility = ColumnDurationCheckBox.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        SpeedColumn.Visibility = ColumnSpeedCheckBox.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
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
            ColumnDurationCheckBox.IsChecked = true;
            ColumnSpeedCheckBox.IsChecked = true;
            ColumnUpdatedCheckBox.IsChecked = true;
            ApplyColumnVisibility();
            return;
        }

        ColumnPlatformCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(PlatformColumn), true);
        ColumnUrlCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(UrlColumn), true);
        ColumnStatusCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(StatusColumn), true);
        ColumnQualityCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(QualityColumn), true);
        ColumnFormatCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(FormatColumn), true);
        ColumnDurationCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(DurationColumn), true);
        ColumnSpeedCheckBox.IsChecked = visibleColumns.GetValueOrDefault(nameof(SpeedColumn), true);
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
            [nameof(DurationColumn)] = DurationColumn.Visibility == Visibility.Visible,
            [nameof(SpeedColumn)] = SpeedColumn.Visibility == Visibility.Visible,
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

    private void TaskCard_MouseLeftButtonDown(object sender, MouseButtonEventArgs e)
    {
        if (sender is not Border { Tag: RecordingJob job })
        {
            return;
        }

        if (e.OriginalSource is DependencyObject source
            && (FindVisualParent<global::System.Windows.Controls.Button>(source) is not null
                || FindVisualParent<global::System.Windows.Controls.CheckBox>(source) is not null))
        {
            return;
        }

        if (e.ChangedButton == MouseButton.Left && e.ClickCount >= 2)
        {
            _viewModel.SelectedJob = job;
            job.IsBatchSelected = true;
            LoadJobIntoEditor(job);
            UpdateToggleSelectAllButtonText();
            e.Handled = true;
            return;
        }

        job.IsBatchSelected = !job.IsBatchSelected;
        _viewModel.SelectedJob = job.IsBatchSelected ? job : null;
        UpdateToggleSelectAllButtonText();
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

    private void ShowJobDetailsWindow(RecordingJob job)
    {
        var detailsTextBox = new global::System.Windows.Controls.TextBox
        {
            Text = job.DetailSummary,
            IsReadOnly = true,
            IsReadOnlyCaretVisible = true,
            AcceptsReturn = true,
            TextWrapping = TextWrapping.Wrap,
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
            HorizontalScrollBarVisibility = ScrollBarVisibility.Auto,
            FontFamily = new global::System.Windows.Media.FontFamily("Consolas"),
            FontSize = 13,
            Margin = new Thickness(0, 0, 0, 14),
        };

        var copyButton = new global::System.Windows.Controls.Button
        {
            Content = "复制全部",
            Width = 100,
            Margin = new Thickness(0, 0, 10, 0),
        };

        copyButton.Click += (_, _) =>
        {
            global::System.Windows.Clipboard.SetText(detailsTextBox.Text);
            detailsTextBox.Focus();
            detailsTextBox.SelectAll();
        };

        var closeButton = new global::System.Windows.Controls.Button
        {
            Content = "关闭",
            Width = 100,
            IsCancel = true,
        };

        var buttonPanel = new StackPanel
        {
            Orientation = global::System.Windows.Controls.Orientation.Horizontal,
            HorizontalAlignment = global::System.Windows.HorizontalAlignment.Right,
        };
        buttonPanel.Children.Add(copyButton);
        buttonPanel.Children.Add(closeButton);

        var layoutRoot = new Grid
        {
            Margin = new Thickness(18),
        };
        layoutRoot.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        layoutRoot.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        Grid.SetRow(detailsTextBox, 0);
        Grid.SetRow(buttonPanel, 1);
        layoutRoot.Children.Add(detailsTextBox);
        layoutRoot.Children.Add(buttonPanel);

        var window = new global::System.Windows.Window
        {
            Owner = this,
            Title = $"任务详细信息 - {job.DisplayTitle}",
            Width = 760,
            Height = 560,
            MinWidth = 560,
            MinHeight = 420,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            Background = new SolidColorBrush(global::System.Windows.Media.Color.FromRgb(247, 242, 232)),
            Content = layoutRoot,
        };

        closeButton.Click += (_, _) => window.Close();
        window.ShowDialog();
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
