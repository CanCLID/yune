param(
    [string]$OracleRoot,
    [string]$JyutpingSchemaSource,
    [string]$Output,
    [string]$EvidenceOutput
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}
if ([string]::IsNullOrWhiteSpace($Output)) {
    $Output = Join-Path $RepoRoot "crates\yune-core\tests\fixtures\upstream-jyutping\jyutping-m28-followup-composition.json"
}
if ([string]::IsNullOrWhiteSpace($EvidenceOutput)) {
    $EvidenceOutput = Join-Path $RepoRoot "third_party\typeduck-web\e2e\results\m28-follow-up-upstream-jyutping\oracle-capture.md"
}

function Resolve-JyutpingSchemaSource {
    param([string]$Root)

    if (-not [string]::IsNullOrWhiteSpace($Root)) {
        return [System.IO.Path]::GetFullPath($Root)
    }

    $Candidates = Get-ChildItem -Path (Join-Path $RepoRoot "target") -Recurse -Filter "jyut6ping3.dict.yaml" -ErrorAction SilentlyContinue |
        Where-Object {
            (Test-Path -LiteralPath (Join-Path $_.DirectoryName "jyut6ping3_mobile.schema.yaml")) -and
            (Test-Path -LiteralPath (Join-Path $_.DirectoryName "jyut6ping3.schema.yaml"))
        } |
        Where-Object { $_.DirectoryName -notlike "*third_party*" } |
        Sort-Object @{
            Expression = {
                if ((Split-Path -Leaf $_.DirectoryName) -match "[0-9a-f]{40}") { 0 } else { 1 }
            }
        }, FullName

    if ($Candidates.Count -eq 0) {
        throw "Could not find pinned Jyutping schema source under target. Pass -JyutpingSchemaSource."
    }
    return [System.IO.Path]::GetFullPath($Candidates[0].DirectoryName)
}

function Get-SourceCommit {
    param([string]$SourceRoot)

    if (Test-Path -LiteralPath (Join-Path $SourceRoot ".git")) {
        $Commit = git -C $SourceRoot rev-parse HEAD
        if ($LASTEXITCODE -ne 0) {
            throw "git rev-parse failed for $SourceRoot"
        }
        return $Commit.Trim()
    }

    $Leaf = Split-Path -Leaf $SourceRoot
    if ($Leaf -match "([0-9a-f]{40})") {
        return $Matches[1]
    }
    throw "Could not derive pinned schema commit from $SourceRoot"
}

$OracleRoot = [System.IO.Path]::GetFullPath($OracleRoot)
$Output = [System.IO.Path]::GetFullPath($Output)
$EvidenceOutput = [System.IO.Path]::GetFullPath($EvidenceOutput)
$JyutpingSchemaSource = Resolve-JyutpingSchemaSource $JyutpingSchemaSource
$SchemaCommit = Get-SourceCommit $JyutpingSchemaSource

$Extract = Join-Path $OracleRoot "extract"
$Shared = Join-Path $OracleRoot "m28-follow-up-jyutping-shared"
$User = Join-Path $OracleRoot "m28-follow-up-jyutping-user"
$Build = Join-Path $User "build"
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"

$RequiredPaths = @(
    (Join-Path $Extract "dist\lib\rime.dll"),
    (Join-Path $Extract "dist\bin\rime_deployer.exe"),
    (Join-Path $Extract "dist\include\rime_api.h"),
    (Join-Path $Extract "share\opencc"),
    $JyutpingSchemaSource,
    (Join-Path $JyutpingSchemaSource "jyut6ping3_mobile.schema.yaml"),
    (Join-Path $JyutpingSchemaSource "jyut6ping3.schema.yaml"),
    (Join-Path $JyutpingSchemaSource "jyut6ping3.dict.yaml"),
    $ProbeSource
)
foreach ($Path in $RequiredPaths) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing required upstream Jyutping oracle input: $Path"
    }
}
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    throw "Node.js is required to write deterministic UTF-8 fixture JSON."
}

foreach ($Dir in @($Shared, $User)) {
    $ResolvedDir = [System.IO.Path]::GetFullPath($Dir)
    if (-not $ResolvedDir.StartsWith($OracleRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to recreate outside oracle root: $ResolvedDir"
    }
    if (Test-Path -LiteralPath $Dir) {
        Remove-Item -LiteralPath $Dir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
}

Get-ChildItem -LiteralPath $JyutpingSchemaSource -File |
    Where-Object { $_.Name -like "*.yaml" -or $_.Name -eq "essay.txt" } |
    ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $Shared $_.Name) -Force
    }

$OpenCcDest = Join-Path $Shared "opencc"
New-Item -ItemType Directory -Force -Path $OpenCcDest | Out-Null
Get-ChildItem -LiteralPath (Join-Path $Extract "share\opencc") | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $OpenCcDest -Recurse -Force
}

@"
patch:
  schema_list:
    - schema: jyut6ping3_mobile
"@ | Set-Content -LiteralPath (Join-Path $Shared "default.custom.yaml") -Encoding UTF8

New-Item -ItemType Directory -Force -Path $Build | Out-Null
$env:PATH = (Join-Path $Extract "dist\lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH
& (Join-Path $Extract "dist\bin\rime_deployer.exe") --build $User $Shared $Build
if ($LASTEXITCODE -ne 0) {
    throw "rime_deployer.exe --build failed with exit code $LASTEXITCODE"
}

Add-Type -Path $ProbeSource
$Modules = [string[]]@("default")
$Scenario = [RimeProbe+ProbeScenario]::new()
$Scenario.name = "auto_composition_default_before_space"
$Actions = @()
$Action = [RimeProbe+ProbeAction]::new()
$Action.type = "input"
$Action.text = "caksijathaacoenggeoizi"
$Actions += $Action
$Action = [RimeProbe+ProbeAction]::new()
$Action.type = "snapshot"
$Action.label = "before_space"
$Actions += $Action
$Action = [RimeProbe+ProbeAction]::new()
$Action.type = "key"
$Action.keycode = 32
$Action.mask = 0
$Action.label = "after_space"
$Actions += $Action
$Scenario.actions = [RimeProbe+ProbeAction[]]$Actions

$Snapshots = [RimeProbe]::CaptureScenarios(
    $Shared,
    $User,
    $Build,
    "jyut6ping3_mobile",
    $Modules,
    [RimeProbe+ProbeScenario[]]@($Scenario)
)
$SnapshotsJson = Join-Path $OracleRoot "m28-follow-up-jyutping-snapshots.json"
$Snapshots | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $SnapshotsJson -Encoding UTF8

$Composer = Join-Path $OracleRoot "compose-m28-follow-up-jyutping-fixture.js"
@'
const fs = require('fs');
const path = require('path');

const snapshotsFile = process.env.SNAPSHOTS_JSON;
const output = process.env.OUTPUT;
const evidenceOutput = process.env.EVIDENCE_OUTPUT;
const schemaCommit = process.env.SCHEMA_COMMIT;

const readUtf8 = (file) => fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '');
const writeJson = (file, value) => {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
};
const writeText = (file, text) => {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, text, 'utf8');
};

const snapshots = JSON.parse(readUtf8(snapshotsFile));
const beforeSpace = snapshots.find((row) =>
  row.scenario === 'auto_composition_default_before_space' && row.label === 'before_space');
const afterSpace = snapshots.find((row) =>
  row.scenario === 'auto_composition_default_before_space' && row.label === 'after_space');
if (!beforeSpace) {
  throw new Error('missing before_space snapshot');
}
if (!afterSpace) {
  throw new Error('missing after_space snapshot');
}

const captureDate = new Date().toISOString().slice(0, 10);
const fixture = {
  oracle: {
    engine: 'rime/librime',
    engine_tag: '1.17.0',
    engine_commit: '33e78140250125871856cdc5b42ddc6a5fcd3cd4',
    release_url: 'https://github.com/rime/librime/releases/tag/1.17.0',
    binary_assets: [
      'rime-33e7814-Windows-msvc-x64.7z',
      'rime-deps-33e7814-Windows-msvc-x64.7z',
    ],
    canonical_repository: 'https://github.com/rime/librime',
    schema: 'TypeDuck-HK/schema',
    schema_commit: schemaCommit,
    capture_date: captureDate,
    capture_command: 'powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-jyutping-composition.ps1 -OracleRoot target/upstream-oracle/1.17.0',
  },
  schema: 'jyut6ping3_mobile',
  module_list: ['default'],
  scenarios: [
    {
      name: 'auto_composition_default_before_space',
      actions: [
        { type: 'input', text: 'caksijathaacoenggeoizi' },
        { type: 'snapshot', label: 'before_space' },
        { type: 'key', keycode: 32, mask: 0, label: 'after_space' },
      ],
    },
  ],
  capture: {
    schema_data: 'TypeDuck-HK/schema',
    schema_data_commit: schemaCommit,
    source_row_policy: 'm28_followup_upstream_librime_pinned_jyutping_yaml_composition',
    source_dictionary_file: 'TypeDuck-HK/schema/jyut6ping3.dict.yaml',
    deployed_schema_file: 'jyut6ping3_mobile.schema.yaml',
    target_input: 'caksijathaacoenggeoizi',
    oracle_scope: 'upstream librime 1.17.0 engine plus pinned TypeDuck-HK Jyutping source YAML; no TypeDuck fork ABI/plugin/comment oracle',
  },
  oracle_scope: 'composition_and_ranking_only_not_comment_payloads',
  ranking_contract: [
    'sentence_candidate_first',
    'longest_valid_fuzzy_phrase_prefix_before_single_character',
    'single_character_after_phrase_prefix',
    'invalid_fuzzy_missegmentation_ranked_low',
  ],
  auto_composition_on: {
    candidate_rows: beforeSpace.selected_candidates || [],
    space_commit: afterSpace.commit_text,
    remaining_input_after_space: afterSpace.rime_get_input,
  },
  snapshots,
};

writeJson(output, fixture);

const firstCandidates = beforeSpace.selected_candidates || [];
const evidence = [
  '# M28 Follow-Up Upstream Jyutping Oracle Capture',
  '',
  `- Captured: ${captureDate}`,
  '- Engine: upstream rime/librime 1.17.0',
  '- Schema source: TypeDuck-HK/schema pinned Jyutping YAML',
  `- Schema commit: ${schemaCommit}`,
  '- Module list: default',
  '- Runtime note: stock upstream may log `error creating filter: dictionary_lookup_filter`; dictionary_lookup comments are out of scope for this fixture.',
  '- Scenario: auto_composition_default_before_space',
  '',
  '## Before Space',
  '',
  `- input: ${beforeSpace.rime_get_input}`,
  `- preedit: ${beforeSpace.preedit}`,
  `- preview: ${beforeSpace.commit_text_preview}`,
  `- candidates: ${firstCandidates.slice(0, 10).map((row) => `${row.index}:${row.text}`).join(', ')}`,
  '',
  '## After Space',
  '',
  `- processed: ${afterSpace.processed}`,
  `- commit_text: ${afterSpace.commit_text}`,
  `- remaining_input: ${afterSpace.rime_get_input}`,
  `- preedit: ${afterSpace.preedit}`,
  '',
  `Fixture: \`${path.relative(process.cwd(), output).replace(/\\/g, '/')}\``,
  '',
].join('\n');
writeText(evidenceOutput, evidence);
'@ | Set-Content -LiteralPath $Composer -Encoding UTF8

$env:SNAPSHOTS_JSON = $SnapshotsJson
$env:OUTPUT = $Output
$env:EVIDENCE_OUTPUT = $EvidenceOutput
$env:SCHEMA_COMMIT = $SchemaCommit
node $Composer
if ($LASTEXITCODE -ne 0) {
    throw "M28 follow-up Jyutping fixture composer failed with exit code $LASTEXITCODE"
}
Write-Host "Wrote $Output"
Write-Host "Wrote $EvidenceOutput"
