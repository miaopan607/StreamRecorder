using System.Runtime.InteropServices;
using System.Windows.Interop;

namespace StreamRecorder.Desktop.Services;

public static class WindowFrameThemeService
{
    private const int UseImmersiveDarkModeAttribute = 20;
    private const int LegacyUseImmersiveDarkModeAttribute = 19;
    private const int BorderColorAttribute = 34;
    private const int CaptionColorAttribute = 35;
    private const int TextColorAttribute = 36;
    private const int DefaultColor = unchecked((int)0xFFFFFFFF);

    public static void Apply(global::System.Windows.Window window, bool darkMode)
    {
        var handle = new WindowInteropHelper(window).Handle;
        if (handle == IntPtr.Zero)
        {
            return;
        }

        var useDarkMode = darkMode ? 1 : 0;
        _ = DwmSetWindowAttribute(handle, UseImmersiveDarkModeAttribute, ref useDarkMode, sizeof(int));
        _ = DwmSetWindowAttribute(handle, LegacyUseImmersiveDarkModeAttribute, ref useDarkMode, sizeof(int));

        if (!darkMode)
        {
            SetDefaultColor(handle, BorderColorAttribute);
            SetDefaultColor(handle, CaptionColorAttribute);
            SetDefaultColor(handle, TextColorAttribute);
            return;
        }

        SetColor(handle, BorderColorAttribute, GetColorRef("BorderBrush", global::System.Windows.Media.Color.FromRgb(54, 64, 76)));
        SetColor(handle, CaptionColorAttribute, GetColorRef("WindowBackgroundBrush", global::System.Windows.Media.Color.FromRgb(22, 26, 32)));
        SetColor(handle, TextColorAttribute, GetColorRef("TextPrimaryBrush", global::System.Windows.Media.Color.FromRgb(231, 237, 244)));
    }

    private static void SetDefaultColor(IntPtr handle, int attribute)
    {
        var color = DefaultColor;
        _ = DwmSetWindowAttribute(handle, attribute, ref color, sizeof(int));
    }

    private static void SetColor(IntPtr handle, int attribute, int colorRef)
    {
        _ = DwmSetWindowAttribute(handle, attribute, ref colorRef, sizeof(int));
    }

    private static int GetColorRef(string resourceKey, global::System.Windows.Media.Color fallback)
    {
        if (global::System.Windows.Application.Current.Resources[resourceKey] is global::System.Windows.Media.SolidColorBrush brush)
        {
            fallback = brush.Color;
        }

        return fallback.R | (fallback.G << 8) | (fallback.B << 16);
    }

    [DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(IntPtr hwnd, int dwAttribute, ref int pvAttribute, int cbAttribute);
}
