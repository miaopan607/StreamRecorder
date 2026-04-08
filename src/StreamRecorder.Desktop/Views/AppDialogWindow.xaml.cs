using System.Windows;
using System.Windows.Media;
using StreamRecorder.Desktop.Services;

namespace StreamRecorder.Desktop.Views;

public partial class AppDialogWindow : Window
{
    private readonly AppThemeService? _themeService;

    private AppDialogWindow(string title, string message, MessageBoxImage icon, bool showCancel)
    {
        InitializeComponent();
        Title = title;
        MessageTextBlock.Text = message;
        CancelButton.Visibility = showCancel ? Visibility.Visible : Visibility.Collapsed;
        SetIcon(icon);
        if (global::System.Windows.Application.Current is App app)
        {
            _themeService = app.ThemeService;
            _themeService.ThemeChanged += ThemeService_ThemeChanged;
            SourceInitialized += AppDialogWindow_SourceInitialized;
            Closed += AppDialogWindow_Closed;
        }

        Loaded += (_, _) => ConfirmButton.Focus();
    }

    private void AppDialogWindow_SourceInitialized(object? sender, EventArgs e)
    {
        if (_themeService is not null)
        {
            WindowFrameThemeService.Apply(this, _themeService.IsDarkThemeActive);
        }
    }

    private void AppDialogWindow_Closed(object? sender, EventArgs e)
    {
        SourceInitialized -= AppDialogWindow_SourceInitialized;
        Closed -= AppDialogWindow_Closed;
        if (_themeService is not null)
        {
            _themeService.ThemeChanged -= ThemeService_ThemeChanged;
        }
    }

    private void ThemeService_ThemeChanged(bool isDarkTheme)
    {
        Dispatcher.Invoke(() => WindowFrameThemeService.Apply(this, isDarkTheme));
    }

    public static void ShowMessage(Window? owner, string title, string message, MessageBoxImage icon)
    {
        Create(owner, title, message, icon, showCancel: false).ShowDialog();
    }

    public static bool ShowConfirmation(Window? owner, string title, string message, MessageBoxImage icon)
    {
        return Create(owner, title, message, icon, showCancel: true).ShowDialog() == true;
    }

    private static AppDialogWindow Create(Window? owner, string title, string message, MessageBoxImage icon, bool showCancel)
    {
        return new AppDialogWindow(title, message, icon, showCancel)
        {
            Owner = owner,
            WindowStartupLocation = owner is null ? WindowStartupLocation.CenterScreen : WindowStartupLocation.CenterOwner,
        };
    }

    private void ConfirmButton_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = true;
    }

    private void CancelButton_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
    }

    private void SetIcon(MessageBoxImage icon)
    {
        var (backgroundKey, foregroundKey, text) = icon switch
        {
            MessageBoxImage.Error => ("DialogErrorBackgroundBrush", "DialogErrorForegroundBrush", "!"),
            MessageBoxImage.Warning => ("DialogWarningBackgroundBrush", "DialogWarningForegroundBrush", "!"),
            MessageBoxImage.Question => ("DialogInfoBackgroundBrush", "DialogInfoForegroundBrush", "?"),
            _ => ("DialogInfoBackgroundBrush", "DialogInfoForegroundBrush", "i"),
        };

        IconBadge.SetResourceReference(BackgroundProperty, backgroundKey);
        IconTextBlock.SetResourceReference(global::System.Windows.Controls.TextBlock.ForegroundProperty, foregroundKey);
        IconTextBlock.Text = text;
    }
}
