[CmdletBinding()]
param(
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

$PublicRoot = $PSScriptRoot
$RepoRoot = Resolve-Path (Join-Path $PublicRoot "..\..\..")
$AppRoot = Join-Path $RepoRoot "apps\yune-web"
$RuntimeRoot = Join-Path $RepoRoot "packages\yune-web-runtime"
$ManifestPath = Join-Path $PublicRoot "schema-asset-manifest.json"
$DefaultOutput = Join-Path $PublicRoot "dist"
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = $DefaultOutput
}

$ResolvedPublicRoot = (Resolve-Path $PublicRoot).Path
$ResolvedOutputParent = if (Test-Path $OutputDir) {
    (Resolve-Path $OutputDir).Path
} else {
    (Resolve-Path (Split-Path -Parent $OutputDir)).Path
}
if (-not $ResolvedOutputParent.StartsWith($ResolvedPublicRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "OutputDir must stay under $ResolvedPublicRoot"
}

$BinSuffix = if ($IsWindows) { ".cmd" } else { "" }
$Npm = if ($IsWindows) { "npm.cmd" } else { "npm" }
$Esbuild = Join-Path $AppRoot "node_modules\.bin\esbuild$BinSuffix"
$Vite = Join-Path $AppRoot "node_modules\.bin\vite$BinSuffix"
if (-not (Test-Path $Esbuild)) {
    throw "Missing esbuild at $Esbuild. Run the yune-web dependency install first."
}
if (-not (Test-Path $Vite)) {
    throw "Missing Vite at $Vite. Run the yune-web dependency install first."
}

Write-Host "Building @yune-ime/yune-web-runtime"
& $Npm --prefix $RuntimeRoot run build
if ($LASTEXITCODE -ne 0) {
    throw "Runtime build failed"
}

Write-Host "Bundling yune-web worker"
Push-Location $AppRoot
try {
    & $Esbuild "src/worker.ts" "--bundle" "--format=iife" "--outdir=public" "--define:YUNE_PUBLIC_DEMO_BUILD=true" "--minify"
    if ($LASTEXITCODE -ne 0) {
        throw "Public worker build failed"
    }
} finally {
    Pop-Location
}

Write-Host "Building yune-web app"
$previousPublicDemo = $env:VITE_YUNE_PUBLIC_DEMO
$env:VITE_YUNE_PUBLIC_DEMO = "1"
try {
    Push-Location $AppRoot
    try {
        & $Vite "build" "--mode" "public"
        if ($LASTEXITCODE -ne 0) {
            throw "Vite public build failed"
        }
    } finally {
        Pop-Location
    }
} finally {
    if ($null -eq $previousPublicDemo) {
        Remove-Item Env:\VITE_YUNE_PUBLIC_DEMO -ErrorAction SilentlyContinue
    } else {
        $env:VITE_YUNE_PUBLIC_DEMO = $previousPublicDemo
    }
}

$SourceDist = Join-Path $AppRoot "dist"
if (Test-Path $OutputDir) {
    $ResolvedOutput = (Resolve-Path $OutputDir).Path
    if (-not $ResolvedOutput.StartsWith($ResolvedPublicRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove output outside $ResolvedPublicRoot"
    }
    Remove-Item -LiteralPath $ResolvedOutput -Recurse -Force
}
New-Item -ItemType Directory -Path $OutputDir | Out-Null
Copy-Item -Path (Join-Path $SourceDist "*") -Destination $OutputDir -Recurse -Force

$OutputSchema = Join-Path $OutputDir "schema"
if (Test-Path $OutputSchema) {
    Remove-Item -LiteralPath $OutputSchema -Recurse -Force
}
New-Item -ItemType Directory -Path $OutputSchema | Out-Null

$Manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
if ($Manifest.generatedFor -ne "yune-web" -or $Manifest.version -ne "m31-yune-web-public-demo-v1") {
    throw "Unexpected schema asset manifest metadata"
}

$SourceSchema = Join-Path $AppRoot "public\schema"
foreach ($asset in $Manifest.assets) {
    $relative = [string]$asset.path
    $source = Join-Path $SourceSchema ($relative -replace '/', '\')
    $target = Join-Path $OutputSchema ($relative -replace '/', '\')
    if (-not (Test-Path $source)) {
        throw "Missing public schema source asset: $relative"
    }
    New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
    Copy-Item -LiteralPath $source -Destination $target -Force
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $target).Hash.ToLowerInvariant()
    if ($hash -ne $asset.sha256) {
        throw "SHA-256 mismatch for $relative. Expected $($asset.sha256), got $hash"
    }
}

foreach ($publicFile in @("README.md", "PROVENANCE.md", "asset-manifest.md", "cache-policy.md", "schema-asset-manifest.json", "_headers")) {
    Copy-Item -LiteralPath (Join-Path $PublicRoot $publicFile) -Destination (Join-Path $OutputDir $publicFile) -Force
}

$totalSchemaBytes = ($Manifest.assets | Measure-Object -Property bytes -Sum).Sum
Write-Host "Built yune-web public demo at $OutputDir"
Write-Host "Pinned schema payload bytes: $totalSchemaBytes"
