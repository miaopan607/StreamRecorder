using System.IO;
using System.Text.Json;
using System.Windows;
using Microsoft.Win32;

namespace StreamRecorder.Desktop.Services;

public sealed class AppThemeService : IDisposable
{
    private const string LightThemePath = "Themes/Light.xaml";
    private const string DarkThemePath = "Themes/Dark.xaml";
    private const string ThemeModeRegistryPath = @"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    private const string ThemeModeRegistryValue = "AppsUseLightTheme";
    private const string LightMode = "light";
    private const string DarkMode = "dark";
    private const string SystemMode = "system";

    private ResourceDictionary? _activeThemeDictionary;
    private string _requestedThemeMode = LightMode;

    public bool IsDarkThemeActive { get; private set; }

    public event Action<bool>? ThemeChanged;

    public void Initialize()
    {
        SystemEvents.UserPreferenceChanged += OnUserPreferenceChanged;
    }

    public string LoadStoredThemeMode()
    {
        try
        {
            var settingsPath = Path.Combine(ProjectPaths.WorkerDataRoot, "core_settings.json");
            if (!File.Exists(settingsPath))
            {
                return LightMode;
            }

            using var stream = File.OpenRead(settingsPath);
            using var document = JsonDocument.Parse(stream);
            if (!document.RootElement.TryGetProperty("theme_mode", out var themeModeProperty))
            {
                return LightMode;
            }

            return NormalizeThemeMode(themeModeProperty.GetString());
        }
        catch
        {
            return LightMode;
        }
    }

    public void ApplyTheme(string? themeMode)
    {
        _requestedThemeMode = NormalizeThemeMode(themeMode);

        var effectiveThemeMode = ResolveEffectiveThemeMode(_requestedThemeMode);
        var themePath = effectiveThemeMode == DarkMode ? DarkThemePath : LightThemePath;
        var resources = global::System.Windows.Application.Current.Resources.MergedDictionaries;
        if (_activeThemeDictionary is not null)
        {
            resources.Remove(_activeThemeDictionary);
        }

        _activeThemeDictionary = new ResourceDictionary
        {
            Source = new Uri(themePath, UriKind.Relative),
        };

        resources.Insert(0, _activeThemeDictionary);
        IsDarkThemeActive = string.Equals(effectiveThemeMode, DarkMode, StringComparison.OrdinalIgnoreCase);
        ThemeChanged?.Invoke(IsDarkThemeActive);
    }

    public void Dispose()
    {
        SystemEvents.UserPreferenceChanged -= OnUserPreferenceChanged;
    }

    private void OnUserPreferenceChanged(object? sender, UserPreferenceChangedEventArgs e)
    {
        if (!string.Equals(_requestedThemeMode, SystemMode, StringComparison.OrdinalIgnoreCase))
        {
            return;
        }

        global::System.Windows.Application.Current.Dispatcher.BeginInvoke(() => ApplyTheme(_requestedThemeMode));
    }

    private static string ResolveEffectiveThemeMode(string themeMode)
    {
        return themeMode switch
        {
            DarkMode => DarkMode,
            SystemMode => IsSystemLightTheme() ? LightMode : DarkMode,
            _ => LightMode,
        };
    }

    private static string NormalizeThemeMode(string? themeMode)
    {
        return themeMode?.Trim().ToLowerInvariant() switch
        {
            DarkMode => DarkMode,
            SystemMode => SystemMode,
            _ => LightMode,
        };
    }

    private static bool IsSystemLightTheme()
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(ThemeModeRegistryPath);
            var value = key?.GetValue(ThemeModeRegistryValue);
            return value is int intValue ? intValue != 0 : true;
        }
        catch
        {
            return true;
        }
    }
}
