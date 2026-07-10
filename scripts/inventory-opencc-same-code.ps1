[CmdletBinding()]
param(
    [string]$OpenCcPath = "crates/yune-core/src/opencc/data/HKVariantsFull.txt",
    [string]$DictionaryManifest = "jyut6ping3.dict.yaml",
    [Parameter(Mandatory = $true)]
    [string]$DictionaryRoot,
    [Parameter(Mandatory = $true)]
    [string]$Output,
    [Parameter(Mandatory = $true)]
    [ValidatePattern("^[0-9a-fA-F]{40}$")]
    [string]$ExpectedDictionaryCommit,
    [Parameter(Mandatory = $true)]
    [ValidatePattern("^[0-9a-fA-F]{40}$")]
    [string]$ExpectedDictionaryTree,
    [Parameter(Mandatory = $true)]
    [ValidatePattern("^[0-9a-fA-F]{64}$")]
    [string]$ExpectedOpenCcSha256,
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-RequiredPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$Label
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "$Label does not exist: $Path"
    }
    return (Resolve-Path -LiteralPath $Path).Path
}

function Invoke-GitText {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Repository,
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    $text = & git -C $Repository @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "git -C $Repository $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
    return ($text | Out-String).Trim()
}

$resolvedOpenCc = Resolve-RequiredPath -Path $OpenCcPath -Label "OpenCC mapping"
$resolvedDictionaryRoot = Resolve-RequiredPath -Path $DictionaryRoot -Label "Dictionary root"
$resolvedDictionaryManifest = Resolve-RequiredPath `
    -Path (Join-Path $resolvedDictionaryRoot $DictionaryManifest) `
    -Label "Dictionary manifest"
$resolvedOutput = if ([IO.Path]::IsPathRooted($Output)) {
    [IO.Path]::GetFullPath($Output)
} else {
    [IO.Path]::GetFullPath((Join-Path (Get-Location) $Output))
}

if ((Test-Path -LiteralPath $resolvedOutput) -and -not $Force) {
    throw "Output already exists; pass -Force to replace it: $resolvedOutput"
}

$dictionaryCommit = Invoke-GitText -Repository $resolvedDictionaryRoot -Arguments @("rev-parse", "HEAD")
$dictionaryTree = Invoke-GitText -Repository $resolvedDictionaryRoot -Arguments @("rev-parse", "HEAD^{tree}")
$dictionaryStatus = Invoke-GitText -Repository $resolvedDictionaryRoot -Arguments @("status", "--short")
if ($dictionaryStatus) {
    throw "Dictionary checkout is not clean: $dictionaryStatus"
}
if ($dictionaryCommit -ne $ExpectedDictionaryCommit.ToLowerInvariant()) {
    throw "Dictionary commit mismatch: expected $ExpectedDictionaryCommit, got $dictionaryCommit"
}
if ($dictionaryTree -ne $ExpectedDictionaryTree.ToLowerInvariant()) {
    throw "Dictionary tree mismatch: expected $ExpectedDictionaryTree, got $dictionaryTree"
}

$openCcSha256 = (Get-FileHash -LiteralPath $resolvedOpenCc -Algorithm SHA256).Hash.ToLowerInvariant()
if ($openCcSha256 -ne $ExpectedOpenCcSha256.ToLowerInvariant()) {
    throw "OpenCC mapping hash mismatch: expected $ExpectedOpenCcSha256, got $openCcSha256"
}

$manifestSha256 = (Get-FileHash -LiteralPath $resolvedDictionaryManifest -Algorithm SHA256).Hash.ToLowerInvariant()
$importTables = [System.Collections.Generic.List[string]]::new()
$inImportTables = $false
foreach ($line in Get-Content -LiteralPath $resolvedDictionaryManifest -Encoding UTF8) {
    if (-not $inImportTables) {
        if ($line -match "^\s*import_tables\s*:\s*$") {
            $inImportTables = $true
        }
        continue
    }

    if ($line -match "^\s*-\s*([^\s#]+)\s*(?:#.*)?$") {
        $importTables.Add($Matches[1])
        continue
    }
    if ($line.Trim() -and -not $line.TrimStart().StartsWith("#")) {
        break
    }
}
if ($importTables.Count -eq 0) {
    throw "Dictionary manifest has no import_tables entries: $resolvedDictionaryManifest"
}
$duplicateImports = @($importTables | Group-Object | Where-Object Count -gt 1)
if ($duplicateImports) {
    throw "Dictionary manifest repeats import_tables entries: $($duplicateImports.Name -join ', ')"
}

$dictionarySources = @(
    foreach ($table in $importTables) {
        $name = "$table.dict.yaml"
        [pscustomobject]@{
            table = $table
            name = $name
            path = Resolve-RequiredPath `
                -Path (Join-Path $resolvedDictionaryRoot $name) `
                -Label "Imported dictionary file"
        }
    }
)
$protectedInputs = @($resolvedOpenCc, $resolvedDictionaryManifest) + @($dictionarySources.path)
foreach ($inputPath in $protectedInputs) {
    if ($resolvedOutput.Equals($inputPath, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Output must not overwrite an input file: $resolvedOutput"
    }
}

$openRows = [System.Collections.Generic.List[object]]::new()
$variantSet = [System.Collections.Generic.HashSet[string]]::new(
    [System.StringComparer]::Ordinal
)

$openLine = 0
foreach ($line in Get-Content -LiteralPath $resolvedOpenCc -Encoding UTF8) {
    $openLine++
    $trimmed = $line.Trim()
    if (-not $trimmed -or $trimmed.StartsWith("#")) {
        continue
    }

    $parts = @($trimmed -split "\s+")
    if ($parts.Count -lt 2) {
        throw "Malformed OpenCC row at line ${openLine}: $line"
    }
    $values = @($parts[1..($parts.Count - 1)] | Select-Object -Unique)
    $openRows.Add([pscustomobject]@{
            line = $openLine
            key = $parts[0]
            values = $values
        })
    foreach ($value in $values) {
        [void]$variantSet.Add($value)
    }
}

$byText = @{}
$recordCount = 0

foreach ($source in $dictionarySources) {
    $name = $source.name
    $path = $source.path
    $inBody = $false
    $lineNo = 0

    foreach ($line in Get-Content -LiteralPath $path -Encoding UTF8) {
        $lineNo++
        if (-not $inBody) {
            if ($line.Trim() -eq "...") {
                $inBody = $true
            }
            continue
        }

        if ([string]::IsNullOrWhiteSpace($line) -or $line.TrimStart().StartsWith("#")) {
            continue
        }

        $recordCount++
        $tab = $line.IndexOf("`t")
        if ($tab -lt 1) {
            continue
        }

        $text = $line.Substring(0, $tab)
        if (-not $variantSet.Contains($text)) {
            continue
        }

        $rest = $line.Substring($tab + 1)
        $tab2 = $rest.IndexOf("`t")
        $code = if ($tab2 -ge 0) { $rest.Substring(0, $tab2) } else { $rest }
        if (-not $code) {
            continue
        }

        if (-not $byText.ContainsKey($text)) {
            $byText[$text] = [System.Collections.Generic.List[object]]::new()
        }
        $byText[$text].Add([pscustomobject]@{
                text = $text
                code = $code
                file = $name
                line = $lineNo
            })
    }
}

$inventory = [System.Collections.Generic.List[object]]::new()
foreach ($row in $openRows) {
    $codes = @{}
    foreach ($variant in $row.values) {
        if (-not $byText.ContainsKey($variant)) {
            continue
        }
        foreach ($record in $byText[$variant]) {
            if (-not $codes.ContainsKey($record.code)) {
                $codes[$record.code] = [System.Collections.Generic.List[string]]::new()
            }
            if (-not $codes[$record.code].Contains($variant)) {
                $codes[$record.code].Add($variant)
            }
        }
    }

    foreach ($code in $codes.Keys | Sort-Object) {
        $siblings = @($codes[$code])
        if ($siblings.Count -lt 2) {
            continue
        }

        $locations = [System.Collections.Generic.List[string]]::new()
        foreach ($sibling in $siblings) {
            foreach ($record in $byText[$sibling] | Where-Object code -eq $code) {
                $locations.Add("$sibling@$($record.file):$($record.line)")
            }
        }
        $inventory.Add([pscustomobject]@{
                dictionary_commit = $dictionaryCommit
                dictionary_tree = $dictionaryTree
                dictionary_manifest_sha256 = $manifestSha256
                dictionary_import_tables = $importTables -join ";"
                opencc_sha256 = $openCcSha256
                opencc_line = $row.line
                key = $row.key
                outputs = $row.values -join " "
                code = $code
                siblings = $siblings -join " "
                locations = $locations -join ";"
            })
    }
}

$orderedInventory = @($inventory | Sort-Object opencc_line, code)
$parent = Split-Path -Parent $resolvedOutput
if ($parent -and -not (Test-Path -LiteralPath $parent)) {
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
}
$csvLines = @($orderedInventory | ConvertTo-Csv -NoTypeInformation)
$csvText = (($csvLines -join "`n") + "`n")
$temporaryOutput = Join-Path $parent (".{0}.tmp.{1}" -f ([IO.Path]::GetFileName($resolvedOutput)), [Guid]::NewGuid().ToString("N"))
$backupOutput = Join-Path $parent (".{0}.bak.{1}" -f ([IO.Path]::GetFileName($resolvedOutput)), [Guid]::NewGuid().ToString("N"))
try {
    [IO.File]::WriteAllText($temporaryOutput, $csvText, [Text.UTF8Encoding]::new($false))
    if (Test-Path -LiteralPath $resolvedOutput) {
        [IO.File]::Replace($temporaryOutput, $resolvedOutput, $backupOutput)
    } else {
        [IO.File]::Move($temporaryOutput, $resolvedOutput)
    }
} finally {
    if (Test-Path -LiteralPath $temporaryOutput) {
        Remove-Item -LiteralPath $temporaryOutput -Force
    }
    if (Test-Path -LiteralPath $backupOutput) {
        Remove-Item -LiteralPath $backupOutput -Force
    }
}
$outputSha256 = (Get-FileHash -LiteralPath $resolvedOutput -Algorithm SHA256).Hash.ToLowerInvariant()

[pscustomobject]@{
    dictionary_commit = $dictionaryCommit
    dictionary_tree = $dictionaryTree
    dictionary_manifest = $DictionaryManifest
    dictionary_manifest_sha256 = $manifestSha256
    dictionary_import_tables = $importTables -join ";"
    dictionary_records_scanned = $recordCount
    opencc_sha256 = $openCcSha256
    opencc_rows = $openRows.Count
    variant_texts = $variantSet.Count
    same_code_groups = $orderedInventory.Count
    output = $resolvedOutput
    output_sha256 = $outputSha256
}
