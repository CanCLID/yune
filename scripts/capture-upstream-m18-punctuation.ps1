param(
    [string]$OracleRoot,
    [string]$Output
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}
if ([string]::IsNullOrWhiteSpace($Output)) {
    $Output = Join-Path $RepoRoot "crates\yune-core\tests\fixtures\upstream-1.17.0\m18-punctuation-processor.json"
}

$OracleRoot = [System.IO.Path]::GetFullPath($OracleRoot)
$Output = [System.IO.Path]::GetFullPath($Output)
$Extract = Join-Path $OracleRoot "extract"
$Shared = Join-Path $OracleRoot "m18-punctuation-shared"
$User = Join-Path $OracleRoot "m18-punctuation-user"
$Build = Join-Path $User "build"
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"

$RequiredPaths = @(
    (Join-Path $Extract "dist\lib\rime.dll"),
    (Join-Path $Extract "dist\bin\rime_deployer.exe"),
    (Join-Path $Extract "dist\include\rime_api.h"),
    $ProbeSource
)
foreach ($Path in $RequiredPaths) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing required upstream oracle input: $Path"
    }
}

foreach ($Dir in @($Shared, $User)) {
    $ResolvedRoot = [System.IO.Path]::GetFullPath($OracleRoot)
    $ResolvedDir = [System.IO.Path]::GetFullPath($Dir)
    if (-not $ResolvedDir.StartsWith($ResolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to recreate outside oracle root: $ResolvedDir"
    }
    if (Test-Path -LiteralPath $Dir) {
        Remove-Item -LiteralPath $Dir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
}

$IdeographicComma = [string][char]0x3001
$IdeographicFullStop = [string][char]0x3002
$FullwidthExclamation = [string][char]0xFF01
$FullwidthLeftParen = [string][char]0xFF08
$FullwidthRightParen = [string][char]0xFF09
$FullwidthSlash = [string][char]0xFF0F
$DivisionSign = [string][char]0x00F7

$SchemaYaml = @"
schema:
  schema_id: m18_punct
  name: M18 Punctuation
  version: '1'
engine:
  processors:
    - punctuator
  segmentors:
    - abc_segmentor
    - punct_segmentor
  translators:
    - punct_translator
    - echo_translator
punctuator:
  use_space: true
  half_shape:
    ".": {commit: "$IdeographicFullStop"}
    "!": "$FullwidthExclamation"
    "(": {pair: ["$FullwidthLeftParen", "$FullwidthRightParen"]}
    "/": ["$IdeographicComma", "/"]
  full_shape:
    ".": {commit: "$IdeographicFullStop"}
    "!": "$FullwidthExclamation"
    "(": {pair: ["$FullwidthLeftParen", "$FullwidthRightParen"]}
    "/": ["$FullwidthSlash", "$DivisionSign"]
"@

@"
config_version: '1.0'
schema_list:
  - schema: m18_punct
"@ | Set-Content -LiteralPath (Join-Path $Shared "default.yaml") -Encoding UTF8
$SchemaYaml | Set-Content -LiteralPath (Join-Path $Shared "m18_punct.schema.yaml") -Encoding UTF8

New-Item -ItemType Directory -Force -Path $Build | Out-Null
$env:PATH = (Join-Path $Extract "dist\lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH
& (Join-Path $Extract "dist\bin\rime_deployer.exe") --build $User $Shared $Build
if ($LASTEXITCODE -ne 0) {
    throw "rime_deployer.exe --build failed with exit code $LASTEXITCODE"
}

Add-Type -Path $ProbeSource
$Modules = [string[]]@("default")

function New-ProbeAction($Type, $Keycode, $Label) {
    $Action = [RimeProbe+ProbeAction]::new()
    $Action.type = $Type
    $Action.keycode = $Keycode
    $Action.mask = 0
    $Action.label = $Label
    $Action
}

function New-SetOptionAction($Name, $Value) {
    $Action = [RimeProbe+ProbeAction]::new()
    $Action.type = "set_option"
    $Action.option = $Name
    $Action.value = $Value
    $Action
}

function New-Scenario($Name, $Actions) {
    $Scenario = [RimeProbe+ProbeScenario]::new()
    $Scenario.name = $Name
    $Scenario.actions = [RimeProbe+ProbeAction[]]$Actions
    $Scenario
}

$Scenarios = [RimeProbe+ProbeScenario[]]@(
    (New-Scenario "ascii_punct_period" @(
        (New-SetOptionAction "ascii_punct" 1),
        (New-ProbeAction "key" 46 "period_noop")
    )),
    (New-Scenario "direct_commit_period" @(
        (New-ProbeAction "key" 46 "period_commit")
    )),
    (New-Scenario "confirm_unique_bang" @(
        (New-ProbeAction "key" 33 "bang_commit")
    )),
    (New-Scenario "pair_parenthesis" @(
        (New-ProbeAction "key" 40 "open_commit"),
        (New-ProbeAction "key" 40 "close_commit"),
        (New-ProbeAction "key" 40 "open_again_commit")
    )),
    (New-Scenario "slash_candidates" @(
        (New-ProbeAction "key" 47 "slash_candidates"),
        (New-ProbeAction "key" 47 "slash_next")
    ))
)

$Snapshots = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, "m18_punct", $Modules, $Scenarios)

$Oracle = [ordered]@{
    engine = "rime/librime"
    engine_tag = "1.17.0"
    engine_commit = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
    release_url = "https://github.com/rime/librime/releases/tag/1.17.0"
    binary_assets = @(
        "rime-33e7814-Windows-msvc-x64.7z",
        "rime-deps-33e7814-Windows-msvc-x64.7z"
    )
    capture_date = "2026-06-21"
    capture_command = "powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-m18-punctuation.ps1 -OracleRoot target/upstream-oracle/1.17.0"
}

$Fixture = [ordered]@{
    oracle = $Oracle
    schema = "m18_punct"
    module_list = @("default")
    scenarios = @(
        [ordered]@{
            name = "ascii_punct_period"
            actions = @(
                [ordered]@{ type = "set_option"; option = "ascii_punct"; value = 1 },
                [ordered]@{ type = "key"; keycode = 46; mask = 0; label = "period_noop" }
            )
        },
        [ordered]@{
            name = "direct_commit_period"
            actions = @([ordered]@{ type = "key"; keycode = 46; mask = 0; label = "period_commit" })
        },
        [ordered]@{
            name = "confirm_unique_bang"
            actions = @([ordered]@{ type = "key"; keycode = 33; mask = 0; label = "bang_commit" })
        },
        [ordered]@{
            name = "pair_parenthesis"
            actions = @(
                [ordered]@{ type = "key"; keycode = 40; mask = 0; label = "open_commit" },
                [ordered]@{ type = "key"; keycode = 40; mask = 0; label = "close_commit" },
                [ordered]@{ type = "key"; keycode = 40; mask = 0; label = "open_again_commit" }
            )
        },
        [ordered]@{
            name = "slash_candidates"
            actions = @(
                [ordered]@{ type = "key"; keycode = 47; mask = 0; label = "slash_candidates" },
                [ordered]@{ type = "key"; keycode = 47; mask = 0; label = "slash_next" }
            )
        }
    )
    capture = [ordered]@{
        schema_data = "inline curated m18_punct.schema.yaml"
        source_row_policy = "curated_processor_schema_literal"
        fixture_schema_yaml = $SchemaYaml
        punctuation_definitions = [ordered]@{
            half_shape = @(
                [ordered]@{ key = "."; kind = "commit"; values = @($IdeographicFullStop) },
                [ordered]@{ key = "!"; kind = "confirm_unique"; values = @($FullwidthExclamation) },
                [ordered]@{ key = "("; kind = "pair"; values = @($FullwidthLeftParen, $FullwidthRightParen) },
                [ordered]@{ key = "/"; kind = "candidates"; values = @($IdeographicComma, "/") }
            )
            full_shape = @(
                [ordered]@{ key = "."; kind = "commit"; values = @($IdeographicFullStop) },
                [ordered]@{ key = "!"; kind = "confirm_unique"; values = @($FullwidthExclamation) },
                [ordered]@{ key = "("; kind = "pair"; values = @($FullwidthLeftParen, $FullwidthRightParen) },
                [ordered]@{ key = "/"; kind = "candidates"; values = @($FullwidthSlash, $DivisionSign) }
            )
            symbols = @()
        }
    }
    snapshots = $Snapshots
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Output) | Out-Null
$Json = $Fixture | ConvertTo-Json -Depth 16
$Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($Output, "$Json`n", $Utf8NoBom)
