using System.Linq;
using StreamRecorder.Desktop.Models;

namespace StreamRecorder.Desktop.Views;

public partial class JobEditorWindow : global::System.Windows.Window
{
    private readonly CoreSettings _settings;
    private readonly RecordingJob? _editingJob;
    private readonly List<RecordingJob> _resultJobs = new();

    public JobEditorWindow(CoreSettings settings, RecordingJob? editingJob = null)
    {
        InitializeComponent();
        _settings = settings;
        _editingJob = editingJob?.Clone();
        SeedDefaults();
        if (_editingJob is not null)
        {
            Title = "编辑任务";
            BatchTab.IsEnabled = false;
            LoadExistingJob(_editingJob);
        }
    }

    public IReadOnlyList<RecordingJob> ResultJobs => _resultJobs;

    private void SeedDefaults()
    {
        SelectComboValue(QualityComboBox, _settings.RecordQuality);
        SelectComboValue(FormatComboBox, _settings.VideoFormat);
        SegmentCheckBox.IsChecked = _settings.SegmentedRecordingEnabled;
        SegmentTimeTextBox.Text = _settings.VideoSegmentTime;
        RecordingDirTextBox.Text = string.Empty;
    }

    private void LoadExistingJob(RecordingJob job)
    {
        UrlTextBox.Text = job.Url;
        StreamerNameTextBox.Text = job.StreamerName;
        SelectComboValue(QualityComboBox, job.Quality);
        SelectComboValue(FormatComboBox, job.RecordFormat);
        SegmentCheckBox.IsChecked = job.SegmentRecord;
        SegmentTimeTextBox.Text = job.SegmentTime;
        MonitorCheckBox.IsChecked = job.MonitorStatus;
        ScheduledCheckBox.IsChecked = job.ScheduledRecording;
        ScheduledStartTextBox.Text = job.ScheduledStartTime;
        MonitorHoursTextBox.Text = job.MonitorHours;
        RecordingDirTextBox.Text = job.RecordingDir;
        MessagePushCheckBox.IsChecked = job.EnabledMessagePush;
        OnlyNotifyCheckBox.IsChecked = job.OnlyNotifyNoRecord;
        DirectDownloadCheckBox.IsChecked = job.FlvUseDirectDownload;
    }

    private void SaveButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        try
        {
            _resultJobs.Clear();
            if (_editingJob is not null || ModeTabs.SelectedIndex == 0)
            {
                _resultJobs.Add(BuildSingleJob());
            }
            else
            {
                var jobs = ParseBatchJobs();
                if (jobs.Count == 0)
                {
                    throw new InvalidOperationException("没有找到有效的批量任务。\n请检查每行是否包含直播链接。\n支持格式：画质编号,链接,主播名");
                }

                _resultJobs.AddRange(jobs);
            }

            DialogResult = true;
        }
        catch (Exception ex)
        {
            global::System.Windows.MessageBox.Show(this, ex.Message, "任务输入有误", global::System.Windows.MessageBoxButton.OK, global::System.Windows.MessageBoxImage.Warning);
        }
    }

    private RecordingJob BuildSingleJob()
    {
        var url = UrlTextBox.Text.Trim();
        if (string.IsNullOrWhiteSpace(url))
        {
            throw new InvalidOperationException("直播间链接不能为空。");
        }

        return new RecordingJob
        {
            Id = _editingJob?.Id ?? string.Empty,
            Url = url,
            StreamerName = string.IsNullOrWhiteSpace(StreamerNameTextBox.Text) ? "直播间" : StreamerNameTextBox.Text.Trim(),
            Quality = SelectedComboText(QualityComboBox, "OD"),
            RecordFormat = SelectedComboText(FormatComboBox, "TS"),
            SegmentRecord = SegmentCheckBox.IsChecked == true,
            SegmentTime = string.IsNullOrWhiteSpace(SegmentTimeTextBox.Text) ? _settings.VideoSegmentTime : SegmentTimeTextBox.Text.Trim(),
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
                RecordFormat = _settings.VideoFormat,
                SegmentRecord = _settings.SegmentedRecordingEnabled,
                SegmentTime = _settings.VideoSegmentTime,
                MonitorStatus = true,
                ScheduledRecording = false,
                MonitorHours = "5",
                RecordingDir = string.Empty,
                EnabledMessagePush = true,
                OnlyNotifyNoRecord = false,
                FlvUseDirectDownload = _settings.FlvUseDirectDownload,
            });
        }

        return jobs;
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

    private static string SelectedComboText(global::System.Windows.Controls.ComboBox comboBox, string fallback)
    {
        return comboBox.SelectedItem is global::System.Windows.Controls.ComboBoxItem item && item.Content is string value ? value : fallback;
    }

    private static void SelectComboValue(global::System.Windows.Controls.ComboBox comboBox, string? value)
    {
        foreach (var item in comboBox.Items.OfType<global::System.Windows.Controls.ComboBoxItem>())
        {
            if (string.Equals(item.Content?.ToString(), value, StringComparison.OrdinalIgnoreCase))
            {
                comboBox.SelectedItem = item;
                return;
            }
        }

        comboBox.SelectedIndex = 0;
    }

    private void CancelButton_Click(object sender, global::System.Windows.RoutedEventArgs e)
    {
        DialogResult = false;
    }
}
