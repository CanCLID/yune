# Captures the librime 1.17.0 oracle provenance for M59 leading-single
# reachability + partial-selection composition on luna_pinyin:
#   - paged candidate lists for moboli / boli / li / zhonggao / zhongguo / gao / guo
#     (the reachable-single positions the M59 acceptance rows cite), and
#   - the moboli -> mo/bo/li partial-selection composition chain (commits the
#     three-character phrase from selecting the leading singles one at a time).
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

function New-Action($type, $text, $kc, $label) {
    $a = [RimeProbe+ProbeAction]::new()
    $a.type = $type
    if ($text) { $a.text = $text }
    if ($kc) { $a.keycode = $kc; $a.mask = 0 }
    if ($label) { $a.label = $label }
    return $a
}
$PageDown = 65366

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

$pagingInputs = @("moboli", "boli", "li", "zhonggao", "zhongguo", "gao", "guo")
$pagingScenarios = $pagingInputs | ForEach-Object { New-Paging $_ 30 }
$modules = [string[]]@("default")
$pageSnaps = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, "luna_pinyin", $modules, [RimeProbe+ProbeScenario[]]$pagingScenarios)

# moboli composition: leading single at page-0 index 2 (select key '3');
# then boli's target at global index 14 (page 2 position 4, key '5' after two
# Page_Down); then li's target at page-0 index 2 (key '3') -> commits the phrase.
$composeActs = @(
    (New-Action "input" "moboli" 0 $null),
    (New-Action "snapshot" $null 0 "moboli_page0"),
    (New-Action "key" $null 51 "after_select_mo"),
    (New-Action "snapshot" $null 0 "boli_page0"),
    (New-Action "key" $null $PageDown "boli_p1"),
    (New-Action "key" $null $PageDown "boli_p2"),
    (New-Action "key" $null 53 "after_select_bo"),
    (New-Action "snapshot" $null 0 "li_page0"),
    (New-Action "key" $null 51 "after_select_li")
)
$composeSc = [RimeProbe+ProbeScenario]::new()
$composeSc.name = "moboli_compose"
$composeSc.actions = [RimeProbe+ProbeAction[]]$composeActs
$composeSnaps = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, "luna_pinyin", $modules, [RimeProbe+ProbeScenario[]]@($composeSc))

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
