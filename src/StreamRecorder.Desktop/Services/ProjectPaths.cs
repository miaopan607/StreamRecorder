using System.IO;

namespace StreamRecorder.Desktop.Services;

public static class ProjectPaths
{
    private static string? _repositoryRoot;

    public static string RepositoryRoot => _repositoryRoot ??= ResolveRepositoryRoot();

    public static string WorkerRoot => Path.Combine(RepositoryRoot, "worker");

    public static string WorkerDataRoot => Path.Combine(RepositoryRoot, "runtime");

    public static string WorkerEntryPath => Path.Combine(WorkerRoot, "streamrecorder_worker", "__main__.py");

    private static string ResolveRepositoryRoot()
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            var solutionPath = Path.Combine(current.FullName, "StreamRecorder.sln");
            var workerPath = Path.Combine(current.FullName, "worker", "streamrecorder_worker", "__main__.py");
            if (File.Exists(solutionPath) || File.Exists(workerPath))
            {
                return current.FullName;
            }

            current = current.Parent;
        }

        return Directory.GetCurrentDirectory();
    }
}
