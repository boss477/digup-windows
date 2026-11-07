param([string]$Executable = $(if (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'digup-windows.exe')) { Join-Path $PSScriptRoot 'digup-windows.exe' } else { Join-Path $PSScriptRoot 'target\release\digup-windows.exe' }))
$ErrorActionPreference = 'Stop'
$root = Join-Path $PSScriptRoot ('vendor\gui-check-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root -Force | Out-Null
$report = Join-Path $root 'render.json'
$previousData = $env:LOCALAPPDATA
try {
    $env:LOCALAPPDATA = Join-Path $root 'isolated-data'
    $process = Start-Process -FilePath $Executable -ArgumentList @('--gui-check', ('"' + $report + '"')) -WindowStyle Hidden -PassThru -RedirectStandardError (Join-Path $root 'stderr.txt')
    if (-not $process.WaitForExit(30000)) { $process.Kill(); $process.WaitForExit(); throw 'Native GUI render check timed out' }
    if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $report)) { throw "GUI check failed; see $root" }
    $result = Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
    if ($result.status -ne 'ok') { throw "Invalid framebuffer; see $report" }
    Get-CimInstance Win32_OperatingSystem | Select-Object Caption,Version,BuildNumber
    $result | Format-List
    Write-Host "PASS: actual native GUI framebuffer rendered; evidence $report. No desktop automation or input testing."
} finally { $env:LOCALAPPDATA = $previousData }
