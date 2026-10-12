param([Parameter(Mandatory)][string]$Archive)
$ErrorActionPreference = 'Stop'
$testRoot = Join-Path $PSScriptRoot ("vendor\package-check-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
Expand-Archive -LiteralPath $Archive -DestinationPath $testRoot
foreach ($required in @('digup-windows.exe','install.ps1','uninstall.ps1','README.md','THIRD-PARTY-NOTICES.txt','THIRD-PARTY.zip','check-prerequisites.ps1','install-dependencies.ps1')) {
    if (-not (Test-Path -LiteralPath (Join-Path $testRoot $required))) { throw "Missing entry: $required" }
}
Add-Type -AssemblyName System.IO.Compression.FileSystem
$notices = [IO.Compression.ZipFile]::OpenRead((Join-Path $testRoot 'THIRD-PARTY.zip'))
try {
    $names = @($notices.Entries | ForEach-Object { $_.FullName.Replace('\','/') })
    if ('source-option-ext-0.2.0/src/lib.rs' -notin $names) { throw 'MPL source missing from notice archive' }
    if ($names | Where-Object { $_ -match 'REVIEW-REQUIRED\.txt$' }) { throw 'Unresolved license flag in release package' }
    if ($names | Where-Object { $_ -match '(pdf-extract|adobe-cmap-parser|type1-encoding-parser|hexf-parse)-' }) { throw 'Excluded dependency remains in release notices' }
    $licenseGroups = @($names | Where-Object { $_ -match '^third-party-licenses/([^/]+)/' } | ForEach-Object { ($_ -split '/')[1] } | Sort-Object -Unique)
    Write-Host "PASS: notice archive contains $($licenseGroups.Count) active dependency groups, MPL source and no unresolved review flags."
} finally { $notices.Dispose() }
$previousPath = $env:Path
$previousData = $env:LOCALAPPDATA
try {
    # This removes developer tools from PATH, but is not a fresh Windows VM.
    $env:Path = "$env:WINDIR\System32;$env:WINDIR"
    $env:LOCALAPPDATA = Join-Path $testRoot 'isolated-data'
    $stdout = Join-Path $testRoot 'self-check.json'
    $stderr = Join-Path $testRoot 'self-check.stderr'
    $process = Start-Process -FilePath (Join-Path $testRoot 'digup-windows.exe') -ArgumentList '--self-check' -WorkingDirectory $testRoot -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    if (-not $process.WaitForExit(30000)) { $process.Kill(); throw 'Package self-check exceeded 30 seconds.' }
    if ($process.ExitCode -ne 0) { throw "Package self-check failed: $(Get-Content -LiteralPath $stderr -Raw)" }
    $report = Get-Content -LiteralPath $stdout -Raw | ConvertFrom-Json
    if ($report.status -ne 'ok') { throw 'Invalid self-check report' }
    Write-Host "PASS: extracted Windows package, minimal PATH, isolated data, no model/build tools; $($report.checks -join ', '). Evidence retained at $testRoot"
} finally { $env:Path = $previousPath; $env:LOCALAPPDATA = $previousData }
