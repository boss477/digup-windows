param(
    [Parameter(Mandatory)][string]$Device,
    [ValidateRange(1024,65535)][int]$Port = 8082,
    [switch]$Multimodal,
    [string]$ModelPath = (Join-Path $env:USERPROFILE '.lmstudio\models\ggml-org\embeddinggemma-2-GGUF\embeddinggemma-2-Q8_0.gguf')
)
$ErrorActionPreference = 'Stop'
$server = Join-Path $PSScriptRoot 'vendor\llama-b11461-vulkan\llama-server.exe'
$projector = Join-Path (Split-Path -Parent $ModelPath) 'mmproj-embeddinggemma-2-BF16.gguf'
foreach ($file in @($server,$ModelPath,$projector)) { if (-not (Test-Path -LiteralPath $file)) { throw "Missing prerequisite: $file" } }
if (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue) { throw "Port $Port already occupied; existing servers will not be stopped" }
$root = Join-Path $PSScriptRoot ('vendor\gpu-check-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root -Force | Out-Null
$stderr = Join-Path $root 'server.stderr.log'
$process = Start-Process -FilePath $server -WindowStyle Hidden -PassThru -ArgumentList @('-m',('"'+$ModelPath+'"'),'--mmproj',('"'+$projector+'"'),'--embedding','--pooling','mean','--device',$Device,'--mmproj-device',$Device,'-ngl','99','--host','127.0.0.1','--port',$Port,'--alias','embeddinggemma-2','-c','2048','--parallel','1','--verbose') -RedirectStandardOutput (Join-Path $root 'server.stdout.log') -RedirectStandardError $stderr
try {
    $deadline = (Get-Date).AddMinutes(4)
    $ready = $false
    while ((Get-Date) -lt $deadline -and -not $process.HasExited) {
        try { $health = Invoke-RestMethod "http://127.0.0.1:$Port/health" -TimeoutSec 2; if ($health.status -eq 'ok') { $ready = $true; break } } catch { }
        Start-Sleep -Seconds 1
    }
    if (-not $ready) { throw "GPU server did not become healthy; evidence $root" }
    $body = @{model='embeddinggemma-2';input='task: search result | A red square on a white background.'} | ConvertTo-Json
    $response = Invoke-RestMethod "http://127.0.0.1:$Port/v1/embeddings" -Method Post -ContentType 'application/json' -Body $body -TimeoutSec 120
    $vector = @($response.data[0].embedding)
    if ($vector.Count -lt 1 -or @($vector | Where-Object { [double]::IsNaN($_) -or [double]::IsInfinity($_) }).Count -gt 0) { throw 'Missing/nonfinite embedding' }
    $norm = ($vector | ForEach-Object { $_ * $_ } | Measure-Object -Sum).Sum
    if ($norm -le 0) { throw 'Zero embedding' }
    if ($Multimodal) { & (Join-Path $PSScriptRoot 'verify-media.ps1') -BaseUrl "http://127.0.0.1:$Port/v1" }
    $log = Get-Content -LiteralPath $stderr -Raw
    if ($log -notmatch 'offloaded [1-9][0-9]*/[0-9]+ layers to GPU') { throw "No positive GPU layer offload evidence; see $stderr" }
    if ($log -notmatch [regex]::Escape($Device)) { throw 'Requested device missing from runtime evidence' }
    if ($Multimodal -and $log -notmatch ('CLIP using ' + [regex]::Escape($Device) + ' backend')) { throw 'Multimodal projector GPU backend not established' }
    $report = @{status='ok';device=$Device;dimensions=$vector.Count;model='embeddinggemma-2';multimodal_tested=[bool]$Multimodal;scope='explicit Vulkan device, positive model-layer offload, real nonzero finite embedding';os=(Get-CimInstance Win32_OperatingSystem).Caption}
    $report | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'report.json')
    Write-Host "PASS: GPU inference on $Device; $($vector.Count) dimensions. Evidence $root"
} finally {
    if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
}
