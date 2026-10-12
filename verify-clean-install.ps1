param([Parameter(Mandatory)][string]$Archive)
$ErrorActionPreference = 'Stop'
$root = Join-Path $PSScriptRoot ('vendor\clean-install-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root -Force | Out-Null
Expand-Archive -LiteralPath $Archive -DestinationPath $root
$installed = Join-Path $env:LOCALAPPDATA 'Programs\DigUpWindows'
if (Test-Path -LiteralPath (Join-Path $installed 'digup-windows.exe')) { throw 'Refusing to replace an existing installation' }
& (Join-Path $root 'install.ps1')
foreach ($name in @('digup-windows.exe','THIRD-PARTY.zip','THIRD-PARTY-NOTICES.txt')) {
    if ((Get-FileHash -LiteralPath (Join-Path $root $name)).Hash -ne (Get-FileHash -LiteralPath (Join-Path $installed $name)).Hash) { throw "Installed payload differs: $name" }
}
& (Join-Path $root 'install.ps1')
& (Join-Path $installed 'uninstall.ps1')
foreach ($name in @('digup-windows.exe','THIRD-PARTY.zip','THIRD-PARTY-NOTICES.txt','uninstall.ps1')) {
    if (Test-Path -LiteralPath (Join-Path $installed $name)) { throw "Uninstaller left owned file: $name" }
}
if (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\DigUpWindows') { throw 'Uninstall registration remains' }
Write-Host 'PASS: extracted-package install, payload hashes, reinstall and uninstall on this Windows host. No desktop automation.'
