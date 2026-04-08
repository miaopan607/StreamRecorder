using System.IO;
using System.Reflection;

namespace StreamRecorder.Desktop.Services;

public static class EmbeddedWorkerService
{
    private static readonly string[] WorkerFiles =
    {
        "__init__.py",
        "__main__.py",
        "app.py",
        "config.py",
        "ffmpeg.py",
        "models.py",
        "notification_service.py",
        "platforms.py",
        "probe.py",
        "protocol.py",
    };

    public static void EnsureExtracted()
    {
        Directory.CreateDirectory(ProjectPaths.WorkerPackageRoot);

        var assembly = Assembly.GetExecutingAssembly();
        foreach (var fileName in WorkerFiles)
        {
            var resourceName = $"StreamRecorder.WorkerAssets.{fileName}";
            using var resourceStream = assembly.GetManifestResourceStream(resourceName);
            if (resourceStream is null)
            {
                throw new InvalidOperationException($"缺少内置核心服务文件：{fileName}");
            }

            var outputPath = Path.Combine(ProjectPaths.WorkerPackageRoot, fileName);
            using var outputStream = File.Create(outputPath);
            resourceStream.CopyTo(outputStream);
        }
    }
}
