$ErrorActionPreference = 'Stop'
$appPath = Join-Path $PSScriptRoot 'target\release\digup-windows.exe'
if (-not (Test-Path -LiteralPath $appPath)) { & (Join-Path $PSScriptRoot 'build.ps1') }
Start-Process -FilePath $appPath -WorkingDirectory $PSScriptRoot
