param([Parameter(Mandatory)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$metadataJson = & $cargo metadata --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml') --locked --offline --format-version 1 --filter-platform x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed. Run the locked Windows build first.' }
$metadata = $metadataJson | ConvertFrom-Json
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$active = & $cargo tree --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml') --locked --offline --target x86_64-pc-windows-msvc --edges normal,build --prefix none --format '{p}' --no-dedupe
if ($LASTEXITCODE -ne 0) { throw 'Active dependency graph failed' }
$activeNames = @{}
foreach ($line in $active) { if ($line -match '^(\S+) v(\S+)') { $activeNames["$($Matches[1])-$($Matches[2])"] = $true } }
$packages = @($metadata.packages | Where-Object { $_.source -and $activeNames.ContainsKey("$($_.name)-$($_.version)") } | Sort-Object name,version)
$catalog = New-Object Text.StringBuilder
[void]$catalog.AppendLine("DigUp Windows third-party inventory (active Windows normal/build dependency graph; excludes inactive optional crates).")
[void]$catalog.AppendLine("License expressions are upstream metadata, not a legal opinion. Original notices follow in third-party-licenses. External models, LM Studio, llama.cpp, FFmpeg, Tesseract, Poppler and VLC are not bundled; their separate licenses apply.")
$missing = @()
foreach ($package in $packages) {
    $root = Split-Path -Parent $package.manifest_path
    $name = "$($package.name)-$($package.version)"
    [void]$catalog.AppendLine("`n$name | $($package.license) | $($package.repository)")
    $licenses = @(Get-ChildItem -LiteralPath $root -File -Recurse -Depth 2 | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT|OFL|UFL)([._-]|$)' })
    if ($package.license_file) {
        $declared = Join-Path $root $package.license_file
        if (Test-Path -LiteralPath $declared -PathType Leaf) { $licenses += Get-Item -LiteralPath $declared }
    }
    $licenses = @($licenses | Sort-Object FullName -Unique)
    if (-not $licenses.Count) {
        $upstream = Join-Path $PSScriptRoot "vendor\upstream-notices\$name"
        if (Test-Path -LiteralPath $upstream) {
            $target = Join-Path $OutputDirectory "third-party-licenses\$name"
            Copy-Item -LiteralPath $upstream -Destination $target -Recurse
            if (Test-Path -LiteralPath (Join-Path $upstream 'REVIEW-REQUIRED.txt')) {
                throw "Unresolved active license declaration: $name. Release packaging blocked."
            }
            continue
        }
        $missing += $name; continue
    }
    foreach ($license in $licenses) {
        $relative = [IO.Path]::GetRelativePath($root, $license.FullName)
        $target = Join-Path $OutputDirectory "third-party-licenses\$name\$relative"
        New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
        Copy-Item -LiteralPath $license.FullName -Destination $target
    }
    # Preserve the unmodified source for the sole MPL-only dependency, rather than relying on an external source offer.
    if ($package.license -eq 'MPL-2.0') {
        Copy-Item -LiteralPath $root -Destination (Join-Path $OutputDirectory "source-$name") -Recurse
    }
}
[IO.File]::WriteAllText((Join-Path $OutputDirectory 'THIRD-PARTY-NOTICES.txt'), $catalog.ToString())
if ($missing.Count) { throw "License text missing for: $($missing -join ', '). Packaging must not proceed until resolved." }
Write-Host "Collected license/notice evidence for $($packages.Count) active Windows dependencies; no metadata-only exceptions allowed."
