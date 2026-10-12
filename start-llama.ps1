param(
    [string]$ModelPath = (Join-Path $env:USERPROFILE '.lmstudio\models\ggml-org\embeddinggemma-2-GGUF\embeddinggemma-2-Q8_0.gguf'),
    [ValidateRange(1024,65535)][int]$Port = 8080
)
$ErrorActionPreference = 'Stop'
$serverPath = Join-Path $PSScriptRoot 'vendor\llama-b11461-vulkan\llama-server.exe'
if (-not (Test-Path -LiteralPath $serverPath)) { & (Join-Path $PSScriptRoot 'setup-llama.ps1') }
if (-not (Test-Path -LiteralPath $ModelPath)) { throw 'Download EmbeddingGemma 2 GGUF in LM Studio, or pass -ModelPath.' }
$logRoot = Join-Path $PSScriptRoot 'vendor\logs'
$projectorPath = Join-Path (Split-Path -Parent $ModelPath) 'mmproj-embeddinggemma-2-BF16.gguf'
if (-not (Test-Path -LiteralPath $projectorPath)) { throw 'EmbeddingGemma 2 multimodal projector is missing beside the model.' }
New-Item -ItemType Directory -Path $logRoot -Force | Out-Null
Start-Process -FilePath $serverPath -WindowStyle Hidden -ArgumentList @('-m', ('"' + $ModelPath + '"'), '--mmproj', ('"' + $projectorPath + '"'), '--embedding', '--pooling', 'mean', '-ngl', '99', '--host', '127.0.0.1', '--port', $Port, '--alias', 'embeddinggemma-2', '-c', '8192') -RedirectStandardOutput (Join-Path $logRoot "llama-$Port.stdout.log") -RedirectStandardError (Join-Path $logRoot "llama-$Port.stderr.log")
Write-Host "Starting llama.cpp on http://127.0.0.1:$Port/v1. Model ID: embeddinggemma-2. Logs: $logRoot"
