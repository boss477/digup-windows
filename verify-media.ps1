param([string]$BaseUrl = 'http://127.0.0.1:8081/v1')
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$fixtureRoot = Join-Path $PSScriptRoot 'vendor\media-fixtures'
New-Item -ItemType Directory -Path $fixtureRoot -Force | Out-Null
& ffmpeg -y -hide_banner -loglevel error -f lavfi -i 'color=c=red:s=64x64' -frames:v 1 -update 1 (Join-Path $fixtureRoot 'red-fixture.png')
if ($LASTEXITCODE -ne 0) { throw 'Image fixture generation failed' }
& ffmpeg -y -hide_banner -loglevel error -f lavfi -i 'sine=frequency=440:duration=1' -ar 16000 -ac 1 (Join-Path $fixtureRoot 'tone-fixture.wav')
if ($LASTEXITCODE -ne 0) { throw 'Audio fixture generation failed' }
& ffmpeg -y -hide_banner -loglevel error -f lavfi -i 'color=c=red:s=64x64:d=1' -c:v libx264 -pix_fmt yuv420p (Join-Path $fixtureRoot 'video-fixture.mp4')
if ($LASTEXITCODE -ne 0) { throw 'Video fixture generation failed' }
& (Join-Path $PSScriptRoot 'build.ps1') -TestsOnly
$env:DIGUP_LIVE_URL = $BaseUrl
$env:DIGUP_IMAGE_FIXTURE = Join-Path $fixtureRoot 'red-fixture.png'
& (Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe') test --locked live_multimodal_embeddings -- --ignored --nocapture
if ($LASTEXITCODE -ne 0) { throw 'Live multimodal verification failed' }
