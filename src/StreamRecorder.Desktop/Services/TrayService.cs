using Forms = global::System.Windows.Forms;

namespace StreamRecorder.Desktop.Services;

public sealed class TrayService : IDisposable
{
    private readonly global::System.Windows.Window _window;
    private readonly Forms.NotifyIcon _notifyIcon;
    private readonly Action _exitAction;

    public TrayService(global::System.Windows.Window window, Action exitAction)
    {
        _window = window;
        _exitAction = exitAction;
        _notifyIcon = new Forms.NotifyIcon
        {
            Text = "StreamRecorder",
            Visible = true,
            ContextMenuStrip = BuildMenu(),
            Icon = AppIconFactory.CreateTrayIcon(),
        };
        _notifyIcon.MouseClick += (_, e) =>
        {
            if (e.Button == Forms.MouseButtons.Left)
            {
                RestoreWindow();
            }
        };
    }

    public void MinimizeToTray(string? message)
    {
        _window.ShowInTaskbar = false;
        _window.Hide();
        if (!string.IsNullOrWhiteSpace(message))
        {
            ShowBalloon("StreamRecorder", message, Forms.ToolTipIcon.Info);
        }
    }

    public void ShowNotification(string title, string message)
    {
        if (_window.IsVisible && _window.WindowState != global::System.Windows.WindowState.Minimized)
        {
            return;
        }

        ShowBalloon(title, message, Forms.ToolTipIcon.Info);
    }

    public void RestoreWindow()
    {
        _window.ShowInTaskbar = true;
        _window.Opacity = 1;
        _window.Show();
        if (_window.WindowState == global::System.Windows.WindowState.Minimized)
        {
            _window.WindowState = global::System.Windows.WindowState.Normal;
        }
        _window.Activate();
    }

    private Forms.ContextMenuStrip BuildMenu()
    {
        var menu = new Forms.ContextMenuStrip();
        menu.Items.Add("恢复窗口", null, (_, _) => RestoreWindow());
        menu.Items.Add("退出程序", null, (_, _) => _exitAction());
        return menu;
    }

    private void ShowBalloon(string title, string message, Forms.ToolTipIcon icon)
    {
        _notifyIcon.BalloonTipTitle = title;
        _notifyIcon.BalloonTipText = message;
        _notifyIcon.BalloonTipIcon = icon;
        _notifyIcon.ShowBalloonTip(4000);
    }

    public void Dispose()
    {
        _notifyIcon.Visible = false;
        _notifyIcon.Dispose();
    }
}
