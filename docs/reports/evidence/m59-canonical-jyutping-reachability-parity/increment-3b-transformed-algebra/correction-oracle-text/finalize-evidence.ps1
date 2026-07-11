[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Hashes = Join-Path $Root "hashes"
$Manifest = Join-Path $Hashes "artifact-sha256.csv"
$ManifestHash = Join-Path $Hashes "artifact-sha256.csv.sha256"
$Verification = Join-Path $Hashes "manifest-verification.json"

function Write-Utf8NoBom([string]$Path, [string]$Text) {
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false))
}

$Rows = Get-ChildItem -LiteralPath $Root -Recurse -File |
    Where-Object { $_.FullName -notlike (Join-Path $Hashes "*") } |
    Sort-Object FullName |
    ForEach-Object {
        [pscustomobject]@{
            relative_path = $_.FullName.Substring($Root.Length + 1).Replace("\", "/")
            bytes = $_.Length
            sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant()
        }
    }
$Rows | Export-Csv -NoTypeInformation -Encoding UTF8 -LiteralPath $Manifest
$ManifestSha = (Get-FileHash -Algorithm SHA256 -LiteralPath $Manifest).Hash.ToLowerInvariant()
Write-Utf8NoBom $ManifestHash ($ManifestSha + "  artifact-sha256.csv`n")

$Failures = @()
foreach ($Row in (Import-Csv -LiteralPath $Manifest)) {
    $Path = Join-Path $Root ($Row.relative_path.Replace("/", "\"))
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        $Failures += "missing: $($Row.relative_path)"
        continue
    }
    $ActualItem = Get-Item -LiteralPath $Path
    $ActualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ([string]$ActualItem.Length -ne [string]$Row.bytes) {
        $Failures += "size mismatch: $($Row.relative_path)"
    }
    if ($ActualHash -ne $Row.sha256) {
        $Failures += "hash mismatch: $($Row.relative_path)"
    }
}

$VerificationObject = [ordered]@{
    verified_utc = [DateTimeOffset]::UtcNow.ToString("o")
    manifest = "hashes/artifact-sha256.csv"
    manifest_sha256 = $ManifestSha
    artifact_count = @($Rows).Count
    verdict = if ($Failures.Count -eq 0) { "PASS" } else { "FAIL" }
    failures = @($Failures)
}
$VerificationText = (($VerificationObject | ConvertTo-Json -Depth 10).Replace("`r`n", "`n").TrimEnd([char]10)) + "`n"
Write-Utf8NoBom $Verification $VerificationText
$VerificationObject | ConvertTo-Json -Depth 10

if ($Failures.Count -ne 0) {
    exit 1
}
