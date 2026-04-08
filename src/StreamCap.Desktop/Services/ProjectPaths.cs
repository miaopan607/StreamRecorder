using System.IO;

namespace StreamCap.Desktop.Services;

public static class ProjectPaths
{
    private static string? _repositoryRoot;

    public static string RepositoryRoot => _repositoryRoot ??= ResolveRepositoryRoot();

    public static string WorkerRoot => Path.Combine(RepositoryRoot, "worker");

    public static string WorkerDataRoot => Path.Combine(RepositoryRoot, "runtime");

    public static string WorkerEntryPath => Path.Combine(WorkerRoot, "streamcap_worker", "__main__.py");

    private static string ResolveRepositoryRoot()
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            var solutionPath = Path.Combine(current.FullName, "StreamCapRe2.sln");
            var workerPath = Path.Combine(current.FullName, "worker", "streamcap_worker", "__main__.py");
            if (File.Exists(solutionPath) || File.Exists(workerPath))
            {
                return current.FullName;
            }

            current = current.Parent;
        }

        return Directory.GetCurrentDirectory();
    }
}
