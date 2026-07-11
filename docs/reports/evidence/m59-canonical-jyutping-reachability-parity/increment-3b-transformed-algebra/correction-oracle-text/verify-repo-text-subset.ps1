[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$ManifestPath = Join-Path $Root "repo-text-subset-manifest.json"
$Manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $ManifestPath | ConvertFrom-Json
$Failures = [System.Collections.Generic.List[string]]::new()

foreach ($File in $Manifest.files) {
    $Path = Join-Path $Root ([string]$File.path).Replace('/', '\')
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        $Failures.Add("missing: $($File.path)")
        continue
    }
    $Info = Get-Item -LiteralPath $Path
    $Hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ($Info.Length -ne [long]$File.repo_bytes) {
        $Failures.Add("byte count: $($File.path) expected=$($File.repo_bytes) actual=$($Info.Length)")
    }
    if ($Hash -ne [string]$File.repo_sha256) {
        $Failures.Add("sha256: $($File.path) expected=$($File.repo_sha256) actual=$Hash")
    }
}

foreach ($Omitted in $Manifest.omitted) {
    $Pattern = Join-Path $Root ([string]$Omitted.path).Replace('/', '\')
    if (Get-ChildItem -Path $Pattern -Force -ErrorAction SilentlyContinue) {
        $Failures.Add("excluded payload present: $($Omitted.path)")
    }
}

if ($Manifest.external_full_manifest.sha256 -ne
    "4cb688f0624a7c19dd7a35b506aec0f30419f62a4eee0f93911d8caf7c6dcf48") {
    $Failures.Add("external full-manifest binding changed")
}
if ($Manifest.external_full_manifest.verdict -ne "PASS" -or
    $Manifest.external_full_manifest.verified_artifact_count -ne 36) {
    $Failures.Add("external verification summary changed")
}

try {
    Add-Type -Path (Join-Path $Root "probe\oracle-rime-probe.cs") -ErrorAction Stop
}
catch {
    $Failures.Add("preserved C# probe does not compile: $($_.Exception.Message)")
}

$Python = Get-Command python -ErrorAction SilentlyContinue
if (-not $Python) {
    $Failures.Add("python executable not found for independent prism decoder")
}
else {
    $Decoder = Join-Path $Root "probe\decode-prism.py"
    $Observation = Join-Path $Root "prism-observation.json"
    & $Python.Source $Decoder --fixture $Observation --verify --self-test
    if ($LASTEXITCODE -ne 0) {
        $Failures.Add("independent prism decoder self-test failed with exit $LASTEXITCODE")
    }
}

$ObservationDocument = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $Root "prism-observation.json") | ConvertFrom-Json
$ObservationCase = @($ObservationDocument.cases)[0]
if (@($ObservationDocument.cases).Count -ne 1 -or
    $ObservationCase.artifact.path -ne "source/user/build/correction_oracle.prism.bin" -or
    $ObservationCase.artifact.sha256 -ne "d563af9c88e983728107cc15e472f1f1fac41571463416c2668a08d7bfc18c8e" -or
    $ObservationCase.artifact.bytes -ne 1408) {
    $Failures.Add("correction prism observation binding/shape changed")
}
$CuDescriptors = @($ObservationCase.surfaces.cu.descriptors)
if ($ObservationCase.surfaces.cu.present -ne $true -or
    $CuDescriptors.Count -ne 2 -or
    $CuDescriptors[0].is_correction -ne $false -or
    $CuDescriptors[1].is_correction -ne $true -or
    $CuDescriptors[1].credibility_f32_bits -ne "0xC0935D8E" -or
    $ObservationCase.surfaces.cuo.present -ne $true) {
    $Failures.Add("correction prism decoded observation semantics changed")
}

if ($Failures.Count -gt 0) {
    $Failures | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Output "PASS: verified $($Manifest.files.Count) repo text artifacts, compiled the preserved C# probe, ran decoder self-tests, checked the hash-bound decoded observation semantics, and confirmed binary/generated exclusions remain absent."
