$ErrorActionPreference = 'Stop'
$metadata = & (Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe') metadata --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml') --locked --offline --format-version 1 --filter-platform x86_64-pc-windows-msvc | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
$cache = Join-Path $PSScriptRoot 'vendor\upstream-notices'
New-Item -ItemType Directory -Path $cache -Force | Out-Null
$trees = @{}
foreach ($package in $metadata.packages | Where-Object source) {
    $root = Split-Path -Parent $package.manifest_path
    $local = @(Get-ChildItem -LiteralPath $root -File -Recurse -Depth 2 | Where-Object Name -Match '^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT|OFL|UFL)([._-]|$)')
    if ($local.Count) { continue }
    if ($package.repository -notmatch '^https://github.com/([^/]+/[^/]+)') { throw "No supported upstream for $($package.name)" }
    $repo = $Matches[1].TrimEnd('/')
    $vcs = Get-Content -LiteralPath (Join-Path $root '.cargo_vcs_info.json') -Raw | ConvertFrom-Json
    $sha = $vcs.git.sha1
    if ($sha -notmatch '^[0-9a-f]{40}$') { throw 'Missing immutable source revision' }
    $key = "$repo/$sha"
    if (-not $trees.ContainsKey($key)) { $trees[$key] = Invoke-RestMethod "https://api.github.com/repos/$repo/git/trees/$sha`?recursive=1" }
    $licenses = @($trees[$key].tree | Where-Object { $_.type -eq 'blob' -and $_.path -match '(^|/)(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT|OFL|UFL)([._-]|$)' })
    $targetRoot = Join-Path $cache "$($package.name)-$($package.version)"
    New-Item -ItemType Directory -Path $targetRoot -Force | Out-Null
    foreach ($license in $licenses) {
        $target = Join-Path $targetRoot $license.path
        New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
        if (-not (Test-Path -LiteralPath $target)) {
            Invoke-WebRequest "https://raw.githubusercontent.com/$repo/$sha/$($license.path)" -OutFile $target
        }
    }
    [IO.File]::WriteAllText((Join-Path $targetRoot 'SOURCE.txt'), "$($package.repository)`nRevision: $sha`nCrate: $($package.name) $($package.version)`nLicense: $($package.license)")
    if (-not $licenses.Count) {
        # Preserve upstream's actual declaration, rather than fabricate copyright or license files.
        Copy-Item -LiteralPath (Join-Path $root 'Cargo.toml.orig') -Destination (Join-Path $targetRoot 'LICENSE-DECLARATION.toml')
        [IO.File]::WriteAllText((Join-Path $targetRoot 'REVIEW-REQUIRED.txt'), "Upstream declares $($package.license) in its published manifest but supplies no separate license/copyright text at this revision. Full published source and manifest are included for review. Do not treat this inventory as legal clearance for public redistribution.")
    }
    Write-Host "Pinned notices checked: $($package.name) ($($licenses.Count) upstream files)"
}
