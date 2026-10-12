param([switch]$Ocr, [switch]$Media, [switch]$VideoSeeking)
$ErrorActionPreference = 'Stop'
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) { throw 'Install Microsoft App Installer (winget) first.' }
$packages = @('Microsoft.VCRedist.2015+.x64')
if ($Ocr) { $packages += @('UB-Mannheim.TesseractOCR','oschwartz10612.Poppler') }
if ($Media) { $packages += 'Gyan.FFmpeg' }
if ($VideoSeeking) { $packages += 'VideoLAN.VLC' }
foreach ($package in $packages) {
    & winget list --id $package --exact --disable-interactivity | Out-Null
    if ($LASTEXITCODE -eq 0) { Write-Host "$package is already installed."; continue }
    & winget install --id $package --exact --silent --accept-package-agreements --accept-source-agreements --disable-interactivity
    if ($LASTEXITCODE -ne 0) { throw "Installation failed for $package (exit $LASTEXITCODE)" }
}
Write-Host 'Restart DigUp (and your terminal) to refresh PATH after installing optional tools. Each external package retains its own license.'
