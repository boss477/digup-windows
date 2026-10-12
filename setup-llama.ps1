$ErrorActionPreference = 'Stop'
$runtimeRoot = Join-Path $PSScriptRoot 'vendor\llama-b11461-vulkan'
New-Item -ItemType Directory -Path $runtimeRoot -Force | Out-Null
$archivePath = Join-Path $runtimeRoot 'runtime.zip'
$release = Invoke-RestMethod 'https://api.github.com/repos/ggml-org/llama.cpp/releases/tags/b11461'
$asset = $release.assets | Where-Object name -eq 'llama-b11461-bin-win-vulkan-x64.zip'
if (-not $asset) { throw 'Pinned Vulkan runtime release asset is unavailable.' }
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $archivePath
Expand-Archive -LiteralPath $archivePath -DestinationPath $runtimeRoot -Force
Write-Host "Runtime installed in $runtimeRoot"
