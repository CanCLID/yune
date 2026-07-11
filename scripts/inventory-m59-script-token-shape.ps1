param(
    [Parameter(Mandatory = $true)]
    [string]$OracleCapture,
    [Parameter(Mandatory = $true)]
    [string]$Output
)

$ErrorActionPreference = "Stop"
$RepoRoot = [System.IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot "..")).Path)

function Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function GraphemeCount([string]$Text) {
    return [System.Globalization.StringInfo]::ParseCombiningCharacters($Text).Count
}

function CommentTokenCount([object]$Comment) {
    if ($null -eq $Comment -or [string]::IsNullOrWhiteSpace([string]$Comment)) {
        return 0
    }
    return @("$Comment" -split '\s+' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count
}

function EvidencePath([string]$Path) {
    $Full = [System.IO.Path]::GetFullPath($Path)
    $Root = $RepoRoot.TrimEnd("\", "/")
    if ($Full.StartsWith($Root + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $Full.Substring($Root.Length + 1).Replace("\", "/")
    }
    return $Full.Replace("\", "/")
}

$OraclePath = [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $OracleCapture).Path)
$OutputPath = [System.IO.Path]::GetFullPath($Output)
if (Test-Path -LiteralPath $OutputPath) {
    throw "Refusing to overwrite inventory output: $OutputPath"
}

$Utf8 = [System.Text.UTF8Encoding]::new($false)
$Document = [System.IO.File]::ReadAllText($OraclePath, $Utf8) | ConvertFrom-Json
$CasesByInput = @{}
foreach ($Case in $Document.cases) {
    $CasesByInput[[string]$Case.input] = $Case
}

$Rows = @()
$OverallPass = $true
foreach ($InputText in @("being", "beingo", "mgoi", "zijiguk")) {
    if (-not $CasesByInput.ContainsKey($InputText)) {
        throw "Oracle capture is missing required input: $InputText"
    }
    $Case = $CasesByInput[$InputText]
    $Mismatches = @()
    foreach ($Candidate in $Case.all_candidates) {
        $Graphemes = GraphemeCount ([string]$Candidate.text)
        $Tokens = CommentTokenCount $Candidate.comment
        if ($Graphemes -ne $Tokens) {
            $Mismatches += [ordered]@{
                global_index = [int]$Candidate.global_index
                text = [string]$Candidate.text
                comment = $Candidate.comment
                grapheme_count = $Graphemes
                comment_token_count = $Tokens
            }
        }
    }
    $Pass = $Mismatches.Count -eq 0
    $OverallPass = $OverallPass -and $Pass
    $Rows += [ordered]@{
        input = $InputText
        row_count = @($Case.all_candidates).Count
        mismatch_count = $Mismatches.Count
        mismatches = $Mismatches
        pass = $Pass
    }
}

foreach ($InputText in @("beix", "beixngoxx")) {
    if (-not $CasesByInput.ContainsKey($InputText)) {
        throw "Oracle capture is missing required input: $InputText"
    }
}
$Beix = @($CasesByInput["beix"].all_candidates)
$Whole = @($CasesByInput["beixngoxx"].all_candidates)
$WholeTail = @($Whole | Select-Object -Skip 2)
$TailMatches = $WholeTail.Count -eq $Beix.Count
if ($TailMatches) {
    for ($Index = 0; $Index -lt $Beix.Count; $Index += 1) {
        if ([string]$WholeTail[$Index].text -cne [string]$Beix[$Index].text) {
            $TailMatches = $false
            break
        }
    }
}
$HeadGraphemes = @($Whole | Select-Object -First 2 | ForEach-Object { GraphemeCount ([string]$_.text) })
$TailSingleGraphemes = @($WholeTail | Where-Object { (GraphemeCount ([string]$_.text)) -ne 1 }).Count -eq 0
$SpecialPass = $Whole.Count -eq 38 -and
    $Beix.Count -eq 36 -and
    $TailMatches -and
    ($HeadGraphemes -join ",") -eq "2,2" -and
    $TailSingleGraphemes
$OverallPass = $OverallPass -and $SpecialPass

$Result = [ordered]@{
    tool_version = 1
    source_capture = EvidencePath $OraclePath
    source_sha256 = Sha256 $OraclePath
    algorithm = "StringInfo.ParseCombiningCharacters(text).Count versus whitespace comment tokens; blank-comment beixngoxx uses the declared beix tail structure"
    ordinary_cases = $Rows
    beixngoxx = [ordered]@{
        row_count = $Whole.Count
        head_grapheme_counts = $HeadGraphemes
        tail_row_count = $WholeTail.Count
        beix_row_count = $Beix.Count
        tail_text_order_matches_beix = $TailMatches
        every_tail_text_has_one_grapheme = $TailSingleGraphemes
        pass = $SpecialPass
    }
    conclusion = "The five M59 4a acceptance rows do not require multi-character one-syllable ScriptEncoder tokens. Full stem/token recursion is not claimed."
    overall_pass = $OverallPass
}

$Parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($Parent) -and -not (Test-Path -LiteralPath $Parent)) {
    New-Item -ItemType Directory -Force -Path $Parent | Out-Null
}
$Json = ($Result | ConvertTo-Json -Depth 20).Replace("`r`n", "`n").TrimEnd([char]10) + "`n"
[System.IO.File]::WriteAllText($OutputPath, $Json, $Utf8)

if (-not $OverallPass) {
    throw "M59 ScriptEncoder token-shape inventory failed; see $OutputPath"
}
