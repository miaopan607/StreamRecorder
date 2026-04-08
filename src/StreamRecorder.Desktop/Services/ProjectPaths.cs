using System.IO;

namespace StreamRecorder.Desktop.Services;

public static class ProjectPaths
{
    public static string RepositoryRoot => AppContext.BaseDirectory;

    public static string WorkerRoot => Path.Combine(RepositoryRoot, "worker");

    public static string WorkerPackageRoot => Path.Combine(WorkerRoot, "streamrecorder_worker");

    public static string WorkerDataRoot => Path.Combine(RepositoryRoot, "runtime");

    public static string WorkerEntryPath => Path.Combine(WorkerPackageRoot, "__main__.py");

    public static string ResolveRepositoryPath(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            return RepositoryRoot;
        }

        return Path.IsPathRooted(path)
            ? Path.GetFullPath(path)
            : Path.GetFullPath(Path.Combine(RepositoryRoot, path));
    }

}
