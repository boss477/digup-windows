param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
if (-not $SkipBuild) { & (Join-Path $PSScriptRoot 'build.ps1') }
$executable = Join-Path $PSScriptRoot 'target\release\digup-windows.exe'
if (-not (Test-Path -LiteralPath $executable)) { throw 'Release executable missing.' }
$dist = Join-Path $PSScriptRoot 'dist'
New-Item -ItemType Directory -Path $dist -Force | Out-Null
$archive = Join-Path $dist 'DigUpWindows-0.1.2-windows-x64-preview.zip'
if (Test-Path -LiteralPath $archive) { throw 'Package already exists; move it aside before repackaging.' }
$stage = Join-Path $PSScriptRoot ('vendor\package-stage-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage -Force | Out-Null
$notices = Join-Path $stage 'notices'
& (Join-Path $PSScriptRoot 'collect-notices.ps1') -OutputDirectory $notices
Copy-Item -LiteralPath (Join-Path $notices 'THIRD-PARTY-NOTICES.txt') -Destination $stage
Compress-Archive -LiteralPath @((Get-ChildItem -LiteralPath $notices).FullName) -DestinationPath (Join-Path $stage 'THIRD-PARTY.zip')
Copy-Item -LiteralPath $executable -Destination $stage
foreach ($name in @('install.ps1', 'uninstall.ps1', 'README.md', 'check-prerequisites.ps1', 'install-dependencies.ps1', 'setup-llama.ps1', 'start-llama.ps1')) { Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination $stage }
$files = @((Get-ChildItem -LiteralPath $stage -File).FullName)
Compress-Archive -LiteralPath $files -DestinationPath $archive
Get-FileHash -LiteralPath $archive -Algorithm SHA256
Write-Host 'Unsigned preview with active dependency notices/source archive. Packaging rejects unresolved license declarations. This automated inventory is not legal advice or code signing.'
