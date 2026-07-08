# Captures the librime 1.17.0 oracle provenance for M59 leading-single
# reachability + partial-selection composition on luna_pinyin:
#   - paged candidate lists for the PRIMARY non-lexicon case moboyi -> mo/bo/yi
#     and the moboli control, plus zhonggao / zhongguo / gao / guo (the
#     reachable-single positions the M59 acceptance rows cite), and
#   - the moboyi -> 莫伯洢 and moboli -> 莫伯李 partial-selection composition chains
#     (commit the phrase from selecting the leading singles one at a time).
#
# Runs the real rime.dll via scripts/oracle-rime-probe.cs. Requires the upstream
# oracle root laid out by the capture-upstream-* pipeline (rime.dll, rime-shared,
# rime-user/build already deployed). Emits raw UTF-8 snapshot JSON, then curates
# the checked-in fixture via scripts/curate-m59-luna-composition.py (Python reads
# the raw files as UTF-8 files, avoiding PowerShell native-pipe codepage
# corruption of the CJK candidate texts). This .ps1 is intentionally pure-ASCII.
param(
    [string]$OracleRoot,
    [string]$Output
)
$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target/upstream-oracle/1.17.0"
}
if ([string]::IsNullOrWhiteSpace($Output)) {
    $Output = Join-Path $RepoRoot "crates/yune-core/tests/fixtures/upstream-1.17.0/m59-luna-leading-single-composition.json"
}
$Extract = Join-Path $OracleRoot "extract"
$Shared = Join-Path $OracleRoot "rime-shared"
$User = Join-Path $OracleRoot "rime-user"
$Build = Join-Path $User "build"
$Probe = Join-Path $RepoRoot "scripts/oracle-rime-probe.cs"
$env:PATH = (Join-Path $Extract "dist/lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH
Add-Type -Path $Probe

$PageDown = 65366

function New-Action($type, $text, $kc, $label) {
    $a = [RimeProbe+ProbeAction]::new()
    $a.type = $type
    if ($text) { $a.text = $text }
    if ($kc) { $a.keycode = $kc; $a.mask = 0 }
    if ($label) { $a.label = $label }
    return $a
}

function New-Paging([string]$name, [int]$pages) {
    $acts = New-Object System.Collections.Generic.List[object]
    $acts.Add((New-Action "input" $name 0 $null))
    $acts.Add((New-Action "snapshot" $null 0 "p0"))
    for ($i = 1; $i -le $pages; $i++) {
        $acts.Add((New-Action "key" $null $PageDown "p$i"))
    }
    $sc = [RimeProbe+ProbeScenario]::new()
    $sc.name = $name
    $sc.actions = [RimeProbe+ProbeAction[]]$acts.ToArray()
    return $sc
}

# A composition scenario: type $input, then for each step turn $pd pages and
# press the $digit selection key (49='1' selects page position 0, etc.). Digit
# keys are 1-indexed within the page: global index N sits on page (N / page_size)
# at position (N % page_size), selected by digit (position + 1).
function New-Compose([string]$name, [string]$sequence, [object[]]$steps) {
    $acts = New-Object System.Collections.Generic.List[object]
    $acts.Add((New-Action "input" $sequence 0 $null))
    $acts.Add((New-Action "snapshot" $null 0 ($sequence + "_page0")))
    foreach ($step in $steps) {
        for ($i = 1; $i -le [int]$step.pd; $i++) {
            $acts.Add((New-Action "key" $null $PageDown ($step.label + "_pd$i")))
        }
        $acts.Add((New-Action "key" $null ([int]$step.digit) ("after_select_" + $step.label)))
    }
    $sc = [RimeProbe+ProbeScenario]::new()
    $sc.name = $name
    $sc.actions = [RimeProbe+ProbeAction[]]$acts.ToArray()
    return $sc
}

$modules = [string[]]@("default")

# PRIMARY case first: moboyi -> the non-lexicon phrase. Then the moboli control.
$pagingInputs = @("moboyi", "boyi", "yi", "moboli", "boli", "li", "zhonggao", "zhongguo", "gao", "guo")
$pagingScenarios = $pagingInputs | ForEach-Object { New-Paging $_ 40 }
$pageSnaps = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, "luna_pinyin", $modules, [RimeProbe+ProbeScenario[]]$pagingScenarios)

# moboyi -> 莫伯洢: 莫@2 (page0, digit '3'); boyi 伯@19 (page3 pos4, digit '5' after
# 3 Page_Down); yi 洢@155 (page31 pos0, digit '1' after 31 Page_Down).
$moboyi = New-Compose "moboyi_compose" "moboyi" @(
    @{ pd = 0; digit = 51; label = "mo" },
    @{ pd = 3; digit = 53; label = "bo" },
    @{ pd = 31; digit = 49; label = "yi" }
)
# moboli control -> 莫伯李: 莫@2 (digit '3'); 伯@14 (page2 pos4, digit '5' after 2
# Page_Down); 李@2 (page0, digit '3').
$moboli = New-Compose "moboli_compose" "moboli" @(
    @{ pd = 0; digit = 51; label = "mo" },
    @{ pd = 2; digit = 53; label = "bo" },
    @{ pd = 0; digit = 51; label = "li" }
)
$composeSnaps = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, "luna_pinyin", $modules, [RimeProbe+ProbeScenario[]]@($moboyi, $moboli))

$Raw = Join-Path ([System.IO.Path]::GetTempPath()) "m59-luna-raw-$PID"
New-Item -ItemType Directory -Force -Path $Raw | Out-Null
$PagesRaw = Join-Path $Raw "pages.json"
$ComposeRaw = Join-Path $Raw "compose.json"
$pageSnaps | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $PagesRaw -Encoding UTF8
$composeSnaps | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $ComposeRaw -Encoding UTF8

$Curate = Join-Path $RepoRoot "scripts/curate-m59-luna-composition.py"
& python $Curate $PagesRaw $ComposeRaw $Output
if ($LASTEXITCODE -ne 0) {
    throw "curation failed with exit code $LASTEXITCODE"
}
Remove-Item -LiteralPath $Raw -Recurse -Force -ErrorAction SilentlyContinue
Write-Output "captured M59 luna composition oracle -> $Output"
