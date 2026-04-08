using System;
using System.Linq;

namespace StreamRecorder.Desktop;

public partial class App : global::System.Windows.Application
{
    private const string StartupArgument = "--startup";

    protected override void OnStartup(global::System.Windows.StartupEventArgs e)
    {
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
    }
}

