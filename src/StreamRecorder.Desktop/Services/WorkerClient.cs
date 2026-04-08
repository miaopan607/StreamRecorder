using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Text.Json;

namespace StreamRecorder.Desktop.Services;

public sealed class WorkerClient : IAsyncDisposable
{
    private readonly ConcurrentDictionary<string, TaskCompletionSource<JsonElement>> _pending = new();
    private readonly SemaphoreSlim _stdinLock = new(1, 1);
    private Process? _process;
    private Task? _stdoutPump;
    private Task? _stderrPump;

    public event Func<string, JsonElement, Task>? EventReceived;

    public event Action<string>? LogReceived;

    public bool IsRunning => _process is { HasExited: false };

    public async Task StartAsync(CancellationToken cancellationToken = default)
    {
        if (IsRunning)
        {
            return;
        }

        EmbeddedWorkerService.EnsureExtracted();
        Directory.CreateDirectory(ProjectPaths.WorkerDataRoot);

        var startInfo = new ProcessStartInfo
        {
            FileName = "python",
            Arguments = $"-m streamrecorder_worker --stdio --data-root \"{ProjectPaths.WorkerDataRoot}\"",
            WorkingDirectory = ProjectPaths.WorkerRoot,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            StandardInputEncoding = Encoding.UTF8,
            StandardOutputEncoding = Encoding.UTF8,
            StandardErrorEncoding = Encoding.UTF8,
            CreateNoWindow = true,
            UseShellExecute = false,
        };

        var existingPythonPath = startInfo.Environment.ContainsKey("PYTHONPATH")
            ? startInfo.Environment["PYTHONPATH"]
            : string.Empty;
        startInfo.Environment["PYTHONPATH"] = string.IsNullOrWhiteSpace(existingPythonPath)
            ? ProjectPaths.WorkerRoot
            : ProjectPaths.WorkerRoot + Path.PathSeparator + existingPythonPath;
        startInfo.Environment["PYTHONUTF8"] = "1";
        startInfo.Environment["PYTHONIOENCODING"] = "utf-8";

        _process = new Process { StartInfo = startInfo, EnableRaisingEvents = true };
        _process.Exited += (_, _) => LogReceived?.Invoke("核心服务进程已退出");
        _process.Start();

        _stdoutPump = Task.Run(ReadStdoutLoopAsync, cancellationToken);
        _stderrPump = Task.Run(ReadStderrLoopAsync, cancellationToken);

        await Task.Delay(250, cancellationToken);
    }

    public async Task<JsonElement> CallAsync(string method, object? body = null, CancellationToken cancellationToken = default)
    {
        if (_process is null || _process.HasExited)
        {
            throw new InvalidOperationException("核心服务当前未运行。");
        }

        var id = Guid.NewGuid().ToString("N");
        var tcs = new TaskCompletionSource<JsonElement>(TaskCreationOptions.RunContinuationsAsynchronously);
        if (!_pending.TryAdd(id, tcs))
        {
            throw new InvalidOperationException("注册核心服务请求失败。");
        }

        var envelope = new Dictionary<string, object?>
        {
            ["kind"] = "cmd",
            ["id"] = id,
            ["method"] = method,
            ["body"] = body ?? new { },
        };
        var json = JsonSerializer.Serialize(envelope);

        await _stdinLock.WaitAsync(cancellationToken);
        try
        {
            await _process.StandardInput.WriteLineAsync(json);
            await _process.StandardInput.FlushAsync();
        }
        finally
        {
            _stdinLock.Release();
        }

        try
        {
            return await tcs.Task.WaitAsync(TimeSpan.FromSeconds(8), cancellationToken);
        }
        finally
        {
            _pending.TryRemove(id, out _);
        }
    }

    public async Task StopAsync()
    {
        if (_process is null)
        {
            return;
        }

        try
        {
            if (!_process.HasExited)
            {
                await CallAsync("core.shutdown");
                if (!await WaitForExitAsync(_process, TimeSpan.FromSeconds(2)))
                {
                    _process.Kill(entireProcessTree: true);
                }
            }
        }
        catch
        {
            if (!_process.HasExited)
            {
                _process.Kill(entireProcessTree: true);
            }
        }

        if (_stdoutPump is not null)
        {
            await _stdoutPump;
        }
        if (_stderrPump is not null)
        {
            await _stderrPump;
        }
    }

    private async Task ReadStdoutLoopAsync()
    {
        if (_process is null)
        {
            return;
        }

        while (!_process.HasExited)
        {
            var line = await _process.StandardOutput.ReadLineAsync();
            if (line is null)
            {
                break;
            }

            if (string.IsNullOrWhiteSpace(line))
            {
                continue;
            }

            using var document = JsonDocument.Parse(line);
            var root = document.RootElement;
            var kind = root.GetProperty("kind").GetString();

            if (string.Equals(kind, "event", StringComparison.OrdinalIgnoreCase))
            {
                var name = root.GetProperty("name").GetString() ?? "unknown";
                var body = root.TryGetProperty("body", out var bodyElement)
                    ? bodyElement.Clone()
                    : JsonDocument.Parse("{}").RootElement.Clone();
                if (EventReceived is not null)
                {
                    await EventReceived.Invoke(name, body);
                }
                continue;
            }

            var id = root.GetProperty("id").GetString() ?? string.Empty;
            if (!_pending.TryGetValue(id, out var pending))
            {
                continue;
            }

            if (string.Equals(kind, "result", StringComparison.OrdinalIgnoreCase))
            {
                var body = root.TryGetProperty("body", out var bodyElement)
                    ? bodyElement.Clone()
                    : JsonDocument.Parse("{}").RootElement.Clone();
                pending.TrySetResult(body);
            }
            else
            {
                var message = root.GetProperty("error").GetProperty("message").GetString() ?? "核心服务调用失败。";
                pending.TrySetException(new InvalidOperationException(message));
            }
        }
    }

    private async Task ReadStderrLoopAsync()
    {
        if (_process is null)
        {
            return;
        }

        while (!_process.HasExited)
        {
            var line = await _process.StandardError.ReadLineAsync();
            if (line is null)
            {
                break;
            }

            if (!string.IsNullOrWhiteSpace(line))
            {
                LogReceived?.Invoke(line);
            }
        }
    }

    private static async Task<bool> WaitForExitAsync(Process process, TimeSpan timeout)
    {
        using var cts = new CancellationTokenSource(timeout);
        try
        {
            await process.WaitForExitAsync(cts.Token);
            return true;
        }
        catch (OperationCanceledException)
        {
            return false;
        }
    }

    public async ValueTask DisposeAsync()
    {
        await StopAsync();
        _stdinLock.Dispose();
    }
}
