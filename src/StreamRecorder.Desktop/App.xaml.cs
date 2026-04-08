using System;
using System.Linq;
using StreamRecorder.Desktop.Services;

namespace StreamRecorder.Desktop;

public partial class App : global::System.Windows.Application
{
    private const string StartupArgument = "--startup";
    private readonly SingleInstanceService _singleInstanceService = new();
    private bool _pendingActivationRequest;

    protected override void OnStartup(global::System.Windows.StartupEventArgs e)
    {
        if (!_singleInstanceService.TryRegisterPrimaryInstance(HandleActivationRequest))
        {
            SingleInstanceService.TryActivatePrimaryInstance();
            Shutdown();
            return;
        }

        base.OnStartup(e);

        var launchToTray = e.Args.Any(arg => string.Equals(arg, StartupArgument, StringComparison.OrdinalIgnoreCase));
        var mainWindow = new MainWindow(launchToTray);
        MainWindow = mainWindow;

        if (launchToTray)
        {
            mainWindow.Opacity = 0;
            mainWindow.ShowInTaskbar = false;
            mainWindow.ShowActivated = false;
            mainWindow.WindowState = global::System.Windows.WindowState.Minimized;
        }

        mainWindow.Show();

        if (_pendingActivationRequest)
        {
            _pendingActivationRequest = false;
            mainWindow.BringToFront();
        }
    }

    protected override void OnExit(global::System.Windows.ExitEventArgs e)
    {
        _singleInstanceService.Dispose();
        base.OnExit(e);
    }

    private void HandleActivationRequest()
    {
        Dispatcher.BeginInvoke(() =>
        {
            if (MainWindow is global::StreamRecorder.Desktop.MainWindow window)
            {
                window.BringToFront();
                return;
            }

            _pendingActivationRequest = true;
        });
    }
}

