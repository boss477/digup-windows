$ErrorActionPreference = 'Stop'
$destination = Join-Path $env:LOCALAPPDATA 'Programs\DigUpWindows'
$registry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\DigUpWindows'
$shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'DigUp Windows.lnk'
$marker = Join-Path $destination 'digup-installer-test-preserve.md'
if ((Test-Path -LiteralPath (Join-Path $destination 'digup-windows.exe')) -or (Test-Path $registry) -or (Test-Path -LiteralPath $shortcut) -or (Test-Path -LiteralPath $marker)) {
    throw 'Test requires no existing installation, shortcut or test marker; preserve them before testing.'
}
$dataRoot = Join-Path $env:LOCALAPPDATA 'DigUpWindows\data'
$before = @{}
if (Test-Path -LiteralPath $dataRoot) {
    Get-ChildItem -LiteralPath $dataRoot -File | ForEach-Object { $before[$_.FullName] = (Get-FileHash -LiteralPath $_.FullName).Hash }
}
& (Join-Path $PSScriptRoot 'install.ps1')
$installed = Join-Path $destination 'digup-windows.exe'
$source = Join-Path $PSScriptRoot 'target\release\digup-windows.exe'
if ((Get-FileHash -LiteralPath $installed).Hash -ne (Get-FileHash -LiteralPath $source).Hash) { throw 'Installed binary hash mismatch' }
$shell = New-Object -ComObject WScript.Shell
if ($shell.CreateShortcut($shortcut).TargetPath -ne $installed) { throw 'Shortcut target mismatch' }
if ((Get-ItemProperty $registry).DisplayVersion -ne '0.1.2') { throw 'Registration missing' }
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'README.md') -Destination $marker
& (Join-Path $PSScriptRoot 'install.ps1')
& (Join-Path $destination 'uninstall.ps1')
if ((Test-Path -LiteralPath $installed) -or (Test-Path -LiteralPath $shortcut) -or (Test-Path $registry)) { throw 'Uninstall incomplete' }
if (-not (Test-Path -LiteralPath $marker)) { throw 'Uninstaller removed unrelated content' }
foreach ($file in $before.Keys) {
    if ((Get-FileHash -LiteralPath $file).Hash -ne $before[$file]) { throw "User data changed: $file" }
}
# Only the marker created by this test is cleaned up; the directory is never recursively removed.
Remove-Item -LiteralPath $marker
Write-Host "PASS: install/reinstall/uninstall, binary hash, shortcut, registration, unrelated file preservation and $($before.Count) existing data-file hashes. Test marker removed; source and data retained."
