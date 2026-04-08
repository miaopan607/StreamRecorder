using System.Runtime.InteropServices;

namespace StreamRecorder.Desktop.Services;

public static class AppIconFactory
{
    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool DestroyIcon(IntPtr hIcon);

    public static global::System.Windows.Media.ImageSource CreateWindowIcon()
    {
        using var icon = CreateTrayIcon();
        var bitmap = global::System.Windows.Interop.Imaging.CreateBitmapSourceFromHIcon(
            icon.Handle,
            global::System.Windows.Int32Rect.Empty,
            global::System.Windows.Media.Imaging.BitmapSizeOptions.FromWidthAndHeight(64, 64));
        bitmap.Freeze();
        return bitmap;
    }

    public static global::System.Drawing.Icon CreateTrayIcon()
    {
        using var bitmap = new global::System.Drawing.Bitmap(64, 64);
        using var graphics = global::System.Drawing.Graphics.FromImage(bitmap);
        graphics.SmoothingMode = global::System.Drawing.Drawing2D.SmoothingMode.AntiAlias;
        graphics.Clear(global::System.Drawing.Color.Transparent);

        using var path = new global::System.Drawing.Drawing2D.GraphicsPath();
        path.AddArc(4, 4, 16, 16, 180, 90);
        path.AddArc(44, 4, 16, 16, 270, 90);
        path.AddArc(44, 44, 16, 16, 0, 90);
        path.AddArc(4, 44, 16, 16, 90, 90);
        path.CloseFigure();

        using var bgBrush = new global::System.Drawing.SolidBrush(global::System.Drawing.Color.FromArgb(31, 82, 96));
        using var accentBrush = new global::System.Drawing.SolidBrush(global::System.Drawing.Color.FromArgb(231, 135, 65));
        using var barBrush = new global::System.Drawing.SolidBrush(global::System.Drawing.Color.White);

        graphics.FillPath(bgBrush, path);
        graphics.FillEllipse(accentBrush, 41, 9, 12, 12);
        graphics.FillRectangle(barBrush, 16, 18, 8, 28);
        graphics.FillRectangle(barBrush, 28, 12, 8, 34);
        graphics.FillRectangle(barBrush, 40, 24, 8, 22);

        var handle = bitmap.GetHicon();
        try
        {
            using var icon = global::System.Drawing.Icon.FromHandle(handle);
            return (global::System.Drawing.Icon)icon.Clone();
        }
        finally
        {
            DestroyIcon(handle);
        }
    }
}
