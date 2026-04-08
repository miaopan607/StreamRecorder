using System.Windows;
using System.Windows.Media;

namespace StreamRecorder.Desktop.Views;

public partial class AppDialogWindow : Window
{
    private AppDialogWindow(string title, string message, MessageBoxImage icon, bool showCancel)
    {
        InitializeComponent();
        Title = title;
        MessageTextBlock.Text = message;
        CancelButton.Visibility = showCancel ? Visibility.Visible : Visibility.Collapsed;
        SetIcon(icon);
        Loaded += (_, _) => ConfirmButton.Focus();
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
        var (background, foreground, text) = icon switch
        {
            MessageBoxImage.Error => (System.Windows.Media.Color.FromRgb(246, 221, 221), System.Windows.Media.Color.FromRgb(148, 37, 37), "!"),
            MessageBoxImage.Warning => (System.Windows.Media.Color.FromRgb(250, 236, 205), System.Windows.Media.Color.FromRgb(142, 95, 27), "!"),
            MessageBoxImage.Question => (System.Windows.Media.Color.FromRgb(221, 234, 244), System.Windows.Media.Color.FromRgb(29, 70, 84), "?"),
            _ => (System.Windows.Media.Color.FromRgb(221, 234, 244), System.Windows.Media.Color.FromRgb(29, 70, 84), "i"),
        };

        IconBadge.Background = new SolidColorBrush(background);
        IconTextBlock.Foreground = new SolidColorBrush(foreground);
        IconTextBlock.Text = text;
    }
}
