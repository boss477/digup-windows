param([switch]$TestsOnly)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$cargoPath = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargoPath)) { throw 'Install Rust using https://rustup.rs first.' }
$vswherePath = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vsRoot = & $vswherePath -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vsRoot) { throw 'Install Visual Studio C++ Build Tools first.' }
$vcRoot = Get-ChildItem -LiteralPath (Join-Path $vsRoot 'VC\Tools\MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1 -ExpandProperty FullName
$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10'
$sdkVersion = Get-ChildItem -LiteralPath (Join-Path $sdkRoot 'Lib') -Directory | Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'um\x64\kernel32.lib') } | Sort-Object Name -Descending | Select-Object -First 1 -ExpandProperty Name
if (-not $sdkVersion) { throw 'Install the Windows SDK first.' }
$env:Path = "$vcRoot\bin\Hostx64\x64;$sdkRoot\bin\$sdkVersion\x64;" + $env:Path
$env:LIB = "$vcRoot\lib\x64;$sdkRoot\Lib\$sdkVersion\um\x64;$sdkRoot\Lib\$sdkVersion\ucrt\x64"
$env:INCLUDE = "$vcRoot\include;$sdkRoot\Include\$sdkVersion\ucrt;$sdkRoot\Include\$sdkVersion\shared;$sdkRoot\Include\$sdkVersion\um"
& $cargoPath fmt --check
if ($LASTEXITCODE -ne 0) { throw 'Formatting failed' }
& $cargoPath test --locked
if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
& $cargoPath clippy --locked --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'Clippy failed' }
if ($TestsOnly) { return }
& $cargoPath build --release --locked
if ($LASTEXITCODE -ne 0) { throw 'Build failed. Check Visual Studio C++ Build Tools and Windows SDK installation.' }
Write-Host "Built: $PSScriptRoot\target\release\digup-windows.exe"
