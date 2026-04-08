using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using Microsoft.Win32;

namespace StreamRecorder.Desktop.Services;

public sealed class AutoStartService
{
    private const string RunKeyPath = @"Software\Microsoft\Windows\CurrentVersion\Run";
    private const string ValueName = "StreamRecorder";

    public bool IsEnabled()
    {
        using var key = Registry.CurrentUser.OpenSubKey(RunKeyPath, writable: false);
        return key?.GetValue(ValueName) is string value && !string.IsNullOrWhiteSpace(value);
    }

    public void SetEnabled(bool enabled)
    {
        using var key = Registry.CurrentUser.CreateSubKey(RunKeyPath);
        if (key is null)
        {
            throw new InvalidOperationException("无法访问系统启动项注册表。");
        }

        if (enabled)
        {
            key.SetValue(ValueName, BuildLaunchCommand());
            return;
        }

        key.DeleteValue(ValueName, throwOnMissingValue: false);
    }

    private static string BuildLaunchCommand()
    {
        var processPath = Environment.ProcessPath;
        if (!string.IsNullOrWhiteSpace(processPath))
        {
            if (string.Equals(Path.GetFileName(processPath), "dotnet.exe", StringComparison.OrdinalIgnoreCase))
            {
                var entryAssemblyPath = Assembly.GetEntryAssembly()?.Location;
                if (string.IsNullOrWhiteSpace(entryAssemblyPath))
                {
                    throw new InvalidOperationException("无法确定程序入口路径，不能启用开机自启。");
                }

                return $"{Quote(processPath)} {Quote(entryAssemblyPath)} --startup";
            }

            return $"{Quote(processPath)} --startup";
        }

        var mainModulePath = Process.GetCurrentProcess().MainModule?.FileName;
        if (!string.IsNullOrWhiteSpace(mainModulePath))
        {
            return $"{Quote(mainModulePath)} --startup";
        }

        throw new InvalidOperationException("无法确定当前程序路径，不能启用开机自启。");
    }

    private static string Quote(string value)
    {
        return $"\"{value}\"";
    }
}
