param([switch]$DesktopShortcut)
$ErrorActionPreference = 'Stop'
$source = Join-Path $PSScriptRoot 'digup-windows.exe'
if (-not (Test-Path -LiteralPath $source)) {
    $source = Join-Path $PSScriptRoot 'target\release\digup-windows.exe'
}
if (-not (Test-Path -LiteralPath $source)) { throw 'Build first using build.ps1, or extract the release package.' }
if (Get-Process -Name 'digup-windows' -ErrorAction SilentlyContinue) { throw 'Quit DigUp from its tray menu before installing.' }
$destination = Join-Path $env:LOCALAPPDATA 'Programs\DigUpWindows'
$shell = New-Object -ComObject WScript.Shell
$shortcutPaths = @((Join-Path ([Environment]::GetFolderPath('Programs')) 'DigUp Windows.lnk'))
if ($DesktopShortcut) { $shortcutPaths += Join-Path ([Environment]::GetFolderPath('Desktop')) 'DigUp Windows.lnk' }
foreach ($shortcutPath in $shortcutPaths) {
    if ((Test-Path -LiteralPath $shortcutPath) -and ($shell.CreateShortcut($shortcutPath).TargetPath -ne (Join-Path $destination 'digup-windows.exe'))) {
        throw "An unrelated shortcut already exists: $shortcutPath. Move it aside before installation."
    }
}
New-Item -ItemType Directory -Path $destination -Force | Out-Null
Copy-Item -LiteralPath $source -Destination (Join-Path $destination 'digup-windows.exe') -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'uninstall.ps1') -Destination $destination -Force
foreach ($name in @('THIRD-PARTY-NOTICES.txt','THIRD-PARTY.zip','check-prerequisites.ps1','README.md')) {
    $file = Join-Path $PSScriptRoot $name
    if (Test-Path -LiteralPath $file) { Copy-Item -LiteralPath $file -Destination $destination -Force }
}
foreach ($shortcutPath in $shortcutPaths) {
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = Join-Path $destination 'digup-windows.exe'
    $shortcut.WorkingDirectory = $destination
    $shortcut.Save()
}
$registryPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\DigUpWindows'
New-Item -Path $registryPath -Force | Out-Null
$values = @{
    DisplayName = 'DigUp Windows (preview)'
    DisplayVersion = '0.1.2'
    InstallLocation = $destination
    UninstallString = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$destination\uninstall.ps1`""
}
foreach ($key in $values.Keys) { New-ItemProperty -Path $registryPath -Name $key -Value $values[$key] -PropertyType String -Force | Out-Null }
Write-Host "Installed to $destination. Models, inference servers and FFmpeg are separate prerequisites."
