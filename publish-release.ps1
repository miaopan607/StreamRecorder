$ErrorActionPreference = 'Stop'
$project = Join-Path $PSScriptRoot 'src\StreamRecorder.Tauri'
$output = Join-Path $PSScriptRoot 'artifacts\publish\win-x64'

Push-Location $project
try {
    & npm.cmd ci
    if ($LASTEXITCODE -ne 0) { throw '安装前端依赖失败。' }
    & npm.cmd run tauri -- build --no-bundle --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Tauri 发布构建失败。' }

    $executable = Join-Path $project 'src-tauri\target\x86_64-pc-windows-msvc\release\StreamRecorder.exe'
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) { throw '发布构建未生成 StreamRecorder.exe。' }
    New-Item -ItemType Directory -Force -Path $output | Out-Null
    # 只移除已知旧桌面构建副产物，保留所有配置、录制文件和未知用户文件。
    $oldFiles = @('StreamRecorder.dll', 'StreamRecorder.pdb', 'StreamRecorder.deps.json', 'StreamRecorder.runtimeconfig.json')
    foreach ($name in $oldFiles) {
        $path = Join-Path $output $name
        if (Test-Path -LiteralPath $path -PathType Leaf) { Remove-Item -LiteralPath $path }
    }
    $extraFiles = @(Get-ChildItem -LiteralPath $output -File | Where-Object { $_.Name -ne 'StreamRecorder.exe' })
    if ($extraFiles.Count -gt 0) {
        throw ('发布目录包含未知文件，请将这些文件移到其他目录后再发布：' + (($extraFiles | ForEach-Object { $_.Name }) -join '、'))
    }
    Copy-Item -LiteralPath $executable -Destination (Join-Path $output 'StreamRecorder.exe') -Force
    Write-Host ('已发布：' + (Join-Path $output 'StreamRecorder.exe'))
    Write-Host '首次运行自动生成 worker/ 和 runtime/。已有运行数据不会删除。'
}
finally {
    Pop-Location
}
