$ErrorActionPreference = 'Stop'
Write-Host "Windows architecture: $([Runtime.InteropServices.RuntimeInformation]::OSArchitecture)"
if ([Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne 'X64') { throw 'This package targets Windows x64.' }
$runtime = Join-Path $env:WINDIR 'System32\vcruntime140.dll'
Write-Host "Microsoft VC++ x64 runtime present: $(Test-Path -LiteralPath $runtime)"
if (-not (Test-Path -LiteralPath $runtime)) { Write-Warning 'Install Microsoft.VCRedist.2015+.x64 using winget, or the official Microsoft VC++ x64 redistributable.' }
foreach ($name in @('ffmpeg','ffprobe','pdftoppm')) {
    $tool = Get-Command $name -ErrorAction SilentlyContinue
    Write-Host "$name : $(if ($tool) { $tool.Source } else { 'missing (optional feature requires it)' })"
}
$tesseract = if ($env:DIGUP_TESSERACT) { $env:DIGUP_TESSERACT } else { Join-Path $env:ProgramFiles 'Tesseract-OCR\tesseract.exe' }
Write-Host "Tesseract: $(if (Test-Path -LiteralPath $tesseract) { $tesseract } else { 'missing; install UB-Mannheim.TesseractOCR using winget' })"
$vlc = if ($env:DIGUP_VLC) { $env:DIGUP_VLC } else { Join-Path $env:ProgramFiles 'VideoLAN\VLC\vlc.exe' }
Write-Host "VLC timestamp seeking: $(Test-Path -LiteralPath $vlc)"
Write-Host 'PDF rendering: install Poppler separately and put pdftoppm on PATH, or set DIGUP_PDFTOPPM to its executable. PDF #page navigation depends on the installed viewer.'
Write-Host 'Semantic search: run LM Studio or llama.cpp embedding server; enter its URL and exact model ID. Filename and exact text search need no server.'
