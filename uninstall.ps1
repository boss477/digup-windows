$ErrorActionPreference = 'Stop'
if (Get-Process -Name 'digup-windows' -ErrorAction SilentlyContinue) { throw 'Quit DigUp from its tray menu before uninstalling.' }
$destination = Join-Path $env:LOCALAPPDATA 'Programs\DigUpWindows'
# Delete only installer-owned files; preserve settings, vectors, models and unknown files.
foreach ($name in @('digup-windows.exe', 'uninstall.ps1', 'THIRD-PARTY-NOTICES.txt', 'THIRD-PARTY.zip', 'check-prerequisites.ps1', 'README.md')) {
    $target = Join-Path $destination $name
    if (Test-Path -LiteralPath $target) { Remove-Item -LiteralPath $target }
}
foreach ($folder in @('Programs', 'Desktop')) {
    $shortcutPath = Join-Path ([Environment]::GetFolderPath($folder)) 'DigUp Windows.lnk'
    if (Test-Path -LiteralPath $shortcutPath) {
        $shell = New-Object -ComObject WScript.Shell
        if ($shell.CreateShortcut($shortcutPath).TargetPath -eq (Join-Path $destination 'digup-windows.exe')) {
            Remove-Item -LiteralPath $shortcutPath
        }
    }
}
$registryPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\DigUpWindows'
if (Test-Path -LiteralPath $registryPath) { Remove-Item -LiteralPath $registryPath }
Write-Host 'Removed installed executable, uninstaller, owned shortcuts and uninstall entry. Settings, cached vectors, models and source files are preserved. Reinstall to restore the application.'
