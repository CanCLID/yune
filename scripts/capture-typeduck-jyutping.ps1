param(
    [ValidateSet("Smoke", "Options", "CompletionCorrection", "SchemaMenu", "UserDb", "PreferUserPhrase", "LetterToTone", "StateLabels", "PredictionRanking", "M21Closeout", "All")]
    [string]$Fixture = "Smoke",
    [string]$OracleRoot,
    [string]$Output,
    [string[]]$Inputs,
    [switch]$InternalCapture,
    [switch]$InternalSchemaList,
    [switch]$InternalStateLabels,
    [switch]$InternalScenarioCapture,
    [switch]$InternalUserDbProbe,
    [switch]$InternalUserDbImportCapture,
    [string]$Shared,
    [string]$User,
    [string]$Build,
    [string]$Schema,
    [string[]]$Modules,
    [string]$InputsFile,
    [string]$ScenariosFile,
    [string]$CasesOutput,
    [string]$DictName = "jyut6ping3",
    [string]$ExportPath,
    [string]$ImportPath,
    [string]$TrainingInput = "nei"
)

$ErrorActionPreference = "Stop"

function Write-Utf8NoBom([string]$Path, [string]$Text) {
    $Dir = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($Dir)) {
        New-Item -ItemType Directory -Force -Path $Dir | Out-Null
    }
    $Encoding = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($Path, $Text, $Encoding)
}

function Write-JsonFile([string]$Path, $Value) {
    Write-Utf8NoBom $Path (($Value | ConvertTo-Json -Depth 32) + "`n")
}

function Read-JsonFile([string]$Path) {
    Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
}

function FullPath([string]$Path) {
    [System.IO.Path]::GetFullPath($Path)
}

function Assert-UnderRoot([string]$Path, [string]$Root) {
    $Full = FullPath $Path
    $FullRoot = FullPath $Root
    if (-not $Full.StartsWith($FullRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to operate outside capture root: $Full"
    }
    $Full
}

function Remove-CaptureTree([string]$Path, [string]$Root) {
    $Full = Assert-UnderRoot $Path $Root
    if (Test-Path -LiteralPath $Full) {
        Remove-Item -LiteralPath $Full -Recurse -Force
    }
}

function Write-CommonCustom([string]$SharedDir, [string[]]$Patches) {
    if ($null -eq $Patches -or $Patches.Count -eq 0) {
        $Text = "patch:`n  __patch: []`n"
    } else {
        $Lines = @("patch:", "  __patch:")
        foreach ($Patch in $Patches) {
            $Lines += "    - $Patch"
        }
        $Text = ($Lines -join "`n") + "`n"
    }
    Write-Utf8NoBom (Join-Path $SharedDir "common.custom.yaml") $Text
}

function Write-SchemaCustom([string]$UserDir, [string]$SchemaId, [string[]]$PatchLines) {
    if ($null -eq $PatchLines -or $PatchLines.Count -eq 0) {
        return
    }
    $Lines = @("patch:")
    foreach ($PatchLine in $PatchLines) {
        $Lines += "  $PatchLine"
    }
    Write-Utf8NoBom (Join-Path $UserDir "$SchemaId.custom.yaml") (($Lines -join "`n") + "`n")
}

function Assert-OracleInputs {
    $RequiredPaths = @(
        (Join-Path $Extract "dist\lib\rime.dll"),
        (Join-Path $Extract "dist\bin\rime_deployer.exe"),
        (Join-Path $Extract "dist\include\rime_api.h"),
        (Join-Path $SharedBase "jyut6ping3_mobile.schema.yaml"),
        (Join-Path $SharedBase "jyut6ping3.schema.yaml"),
        (Join-Path $SharedBase "jyut6ping3.dict.yaml"),
        (Join-Path $RepoRoot "scripts\oracle-rime-probe.cs")
    )
    foreach ($Path in $RequiredPaths) {
        if (-not (Test-Path -LiteralPath $Path)) {
            throw "Missing required TypeDuck oracle input: $Path"
        }
    }
}

function Get-SchemaCommit {
    $SchemaSource = Get-ChildItem -LiteralPath $SchemaRoot -Directory -Filter "schema-*" |
        Select-Object -First 1
    if ($null -eq $SchemaSource) {
        throw "Missing TypeDuck schema source checkout under $SchemaRoot"
    }
    $Commit = $SchemaSource.Name -replace "^schema-", ""
    if ($Commit.Length -ne 40) {
        throw "Unexpected TypeDuck schema source directory name: $($SchemaSource.Name)"
    }
    $Commit
}

function Invoke-Deployer([string]$VariantUser, [string]$VariantShared, [string]$VariantBuild) {
    New-Item -ItemType Directory -Force -Path $VariantBuild | Out-Null
    & (Join-Path $Extract "dist\bin\rime_deployer.exe") --build $VariantUser $VariantShared $VariantBuild
    if ($LASTEXITCODE -ne 0) {
        throw "rime_deployer.exe --build failed with exit code $LASTEXITCODE"
    }
}

function New-Variant(
    [string]$Group,
    [string]$Name,
    [string]$VariantSchema,
    [string[]]$Patches,
    [string[]]$VariantInputs,
    [string]$DefaultCustomMode = "Keep",
    [string[]]$SchemaCustomPatchLines = @()
) {
    $Root = Join-Path $CaptureRoot (Join-Path $Group $Name)
    Remove-CaptureTree $Root $CaptureRoot
    $VariantShared = Join-Path $Root "shared"
    $VariantUser = Join-Path $Root "user"
    $VariantBuild = Join-Path $VariantUser "build"
    New-Item -ItemType Directory -Force -Path $Root, $VariantUser | Out-Null
    Copy-Item -LiteralPath $SharedBase -Destination $VariantShared -Recurse
    Write-CommonCustom $VariantShared $Patches
    Write-SchemaCustom $VariantUser $VariantSchema $SchemaCustomPatchLines
    if ($DefaultCustomMode -eq "Remove") {
        $DefaultCustom = Join-Path $VariantShared "default.custom.yaml"
        if (Test-Path -LiteralPath $DefaultCustom) {
            Remove-Item -LiteralPath $DefaultCustom -Force
        }
    }
    Invoke-Deployer $VariantUser $VariantShared $VariantBuild
    [ordered]@{
        group = $Group
        name = $Name
        schema = $VariantSchema
        patches = $Patches
        inputs = $VariantInputs
        root = $Root
        shared = $VariantShared
        user = $VariantUser
        build = $VariantBuild
        default_custom_mode = $DefaultCustomMode
        schema_custom_patch_lines = $SchemaCustomPatchLines
    }
}

function Invoke-ChildCapture($Variant) {
    $CasesPath = Join-Path $Variant.root "cases.json"
    $InputsPath = Join-Path $Variant.root "inputs.json"
    Write-JsonFile $InputsPath $Variant.inputs
    $Args = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $PSCommandPath,
        "-InternalCapture",
        "-OracleRoot", $OracleRoot,
        "-Shared", $Variant.shared,
        "-User", $Variant.user,
        "-Build", $Variant.build,
        "-Schema", $Variant.schema,
        "-CasesOutput", $CasesPath,
        "-Modules", ($Modules -join ","),
        "-InputsFile", $InputsPath
    )
    & powershell @Args
    if ($LASTEXITCODE -ne 0) {
        throw "child capture failed for $($Variant.name) with exit code $LASTEXITCODE"
    }
    $RawCases = Get-Content -LiteralPath $CasesPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $CaseList = if ($RawCases -is [System.Array]) { $RawCases } else { @($RawCases) }
    foreach ($_ in $CaseList) {
        $Row = [ordered]@{}
        foreach ($Property in $_.PSObject.Properties) {
            $Row[$Property.Name] = $Property.Value
        }
        $Row["variant"] = $Variant.name
        $Row["variant_schema"] = $Variant.schema
        $Row["patches"] = $Variant.patches
        $Row["default_custom_mode"] = $Variant.default_custom_mode
        [pscustomobject]$Row
    }
}

function Invoke-ChildSchemaList($Variant) {
    $CasesPath = Join-Path $Variant.root "schema-list.json"
    $Args = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $PSCommandPath,
        "-InternalSchemaList",
        "-OracleRoot", $OracleRoot,
        "-Shared", $Variant.shared,
        "-User", $Variant.user,
        "-Build", $Variant.build,
        "-CasesOutput", $CasesPath,
        "-Modules", ($Modules -join ",")
    )
    & powershell @Args
    if ($LASTEXITCODE -ne 0) {
        throw "schema-list capture failed for $($Variant.name) with exit code $LASTEXITCODE"
    }
    $Result = Read-JsonFile $CasesPath
    [ordered]@{
        variant = $Variant.name
        patches = $Variant.patches
        default_custom_mode = $Variant.default_custom_mode
        rime_get_schema_list = $Result.rime_get_schema_list
        schemas = $Result.schemas
    }
}

function Invoke-ChildStateLabels($Variant) {
    $CasesPath = Join-Path $Variant.root "state-labels.json"
    $Args = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $PSCommandPath,
        "-InternalStateLabels",
        "-OracleRoot", $OracleRoot,
        "-Shared", $Variant.shared,
        "-User", $Variant.user,
        "-Build", $Variant.build,
        "-Schema", $Variant.schema,
        "-CasesOutput", $CasesPath,
        "-Modules", ($Modules -join ",")
    )
    & powershell @Args
    if ($LASTEXITCODE -ne 0) {
        throw "state-label capture failed for $($Variant.name) with exit code $LASTEXITCODE"
    }
    $Result = Read-JsonFile $CasesPath
    [ordered]@{
        variant = $Variant.name
        schema_id = $Result.schema_id
        schema_name = $Result.schema_name
        patches = $Variant.patches
        default_custom_mode = $Variant.default_custom_mode
        labels = $Result.labels
    }
}

function New-ProbeAction($Action) {
    $TypedAction = [RimeProbe+ProbeAction]::new()
    $TypedAction.type = [string]$Action.type
    if ($null -ne $Action.text) {
        $TypedAction.text = [string]$Action.text
    }
    if ($null -ne $Action.keycode) {
        $TypedAction.keycode = [int]$Action.keycode
    }
    if ($null -ne $Action.mask) {
        $TypedAction.mask = [int]$Action.mask
    }
    if ($null -ne $Action.option) {
        $TypedAction.option = [string]$Action.option
    }
    if ($null -ne $Action.value) {
        $TypedAction.value = [int]$Action.value
    }
    if ($null -ne $Action.label) {
        $TypedAction.label = [string]$Action.label
    }
    $TypedAction
}

function New-ProbeScenario($Scenario) {
    $TypedScenario = [RimeProbe+ProbeScenario]::new()
    $TypedScenario.name = [string]$Scenario.name
    $Actions = @()
    foreach ($Action in @($Scenario.actions)) {
        $Actions += New-ProbeAction $Action
    }
    $TypedScenario.actions = [RimeProbe+ProbeAction[]]$Actions
    $TypedScenario
}

function Invoke-ChildScenarioCapture($Variant, $Scenarios) {
    $CasesPath = Join-Path $Variant.root "scenario-cases.json"
    $ScenariosPath = Join-Path $Variant.root "scenarios.json"
    Write-JsonFile $ScenariosPath $Scenarios
    $Args = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $PSCommandPath,
        "-InternalScenarioCapture",
        "-OracleRoot", $OracleRoot,
        "-Shared", $Variant.shared,
        "-User", $Variant.user,
        "-Build", $Variant.build,
        "-Schema", $Variant.schema,
        "-CasesOutput", $CasesPath,
        "-ScenariosFile", $ScenariosPath,
        "-Modules", ($Modules -join ",")
    )
    & powershell @Args
    if ($LASTEXITCODE -ne 0) {
        throw "scenario capture failed for $($Variant.name) with exit code $LASTEXITCODE"
    }

    $ScenarioInputs = @{}
    foreach ($Scenario in @($Scenarios)) {
        if ($null -ne $Scenario.input) {
            $ScenarioInputs[[string]$Scenario.name] = [string]$Scenario.input
        }
    }
    $RawCases = Get-Content -LiteralPath $CasesPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $CaseList = if ($RawCases -is [System.Array]) { $RawCases } else { @($RawCases) }
    foreach ($_ in $CaseList) {
        $Row = [ordered]@{}
        foreach ($Property in $_.PSObject.Properties) {
            $Row[$Property.Name] = $Property.Value
        }
        $ScenarioName = [string]$Row["scenario"]
        if ($ScenarioInputs.ContainsKey($ScenarioName)) {
            $Row["input"] = $ScenarioInputs[$ScenarioName]
        }
        $Row["variant"] = $Variant.name
        $Row["variant_schema"] = $Variant.schema
        $Row["patches"] = $Variant.patches
        $Row["default_custom_mode"] = $Variant.default_custom_mode
        [pscustomobject]$Row
    }
}

function Invoke-ChildUserDbProbe($Variant) {
    $CasesPath = Join-Path $Variant.root "userdb-probe.json"
    $Export = Join-Path $Variant.root "jyut6ping3-userdb-export.txt"
    $Args = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $PSCommandPath,
        "-InternalUserDbProbe",
        "-OracleRoot", $OracleRoot,
        "-Shared", $Variant.shared,
        "-User", $Variant.user,
        "-Build", $Variant.build,
        "-Schema", $Variant.schema,
        "-CasesOutput", $CasesPath,
        "-DictName", $DictName,
        "-ExportPath", $Export,
        "-TrainingInput", $TrainingInput,
        "-Modules", ($Modules -join ",")
    )
    & powershell @Args
    if ($LASTEXITCODE -ne 0) {
        throw "userdb probe failed for $($Variant.name) with exit code $LASTEXITCODE"
    }
    Read-JsonFile $CasesPath
}

function Invoke-ChildUserDbImportCapture($Variant, [string]$ImportText) {
    $CasesPath = Join-Path $Variant.root "userdb-import-capture.json"
    $Import = Join-Path $Variant.root "prefer-user-phrase-import.tsv"
    Write-Utf8NoBom $Import $ImportText
    $InputsPath = Join-Path $Variant.root "inputs.json"
    Write-JsonFile $InputsPath $Variant.inputs
    $Args = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $PSCommandPath,
        "-InternalUserDbImportCapture",
        "-OracleRoot", $OracleRoot,
        "-Shared", $Variant.shared,
        "-User", $Variant.user,
        "-Build", $Variant.build,
        "-Schema", $Variant.schema,
        "-CasesOutput", $CasesPath,
        "-DictName", $DictName,
        "-ImportPath", $Import,
        "-InputsFile", $InputsPath,
        "-Modules", ($Modules -join ",")
    )
    & powershell @Args
    if ($LASTEXITCODE -ne 0) {
        throw "userdb import capture failed for $($Variant.name) with exit code $LASTEXITCODE"
    }
    Read-JsonFile $CasesPath
}

function New-Fixture([string]$SourceRowPolicy, [string[]]$Schemas, [string[]]$Scenarios, $Cases, $ExtraCapture) {
    $Capture = [ordered]@{
        schema_data = "TypeDuck-HK/schema"
        schema_data_commit = $SchemaCommit
        dictionary = "jyut6ping3.dict.yaml"
        source_row_policy = $SourceRowPolicy
        candidate_layout = "TypeDuck v1.1.2 RimeCandidate includes quality before reserved"
        option_delivery = "deploy-time common.custom.yaml/default.custom.yaml variants; runtime set_option does not apply these schema __patch hooks"
        modules = $Modules
    }
    if ($null -ne $ExtraCapture) {
        foreach ($Property in $ExtraCapture.GetEnumerator()) {
            $Capture[$Property.Key] = $Property.Value
        }
    }

    [ordered]@{
        oracle = [ordered]@{
            engine = "TypeDuck-HK/librime"
            engine_tag = "v1.1.2"
            engine_commit = "74cb52b78fb2411137a7643f6c8bc6517acfde69"
            canonical_repository = "https://github.com/TypeDuck-HK/librime"
            release_url = "https://github.com/TypeDuck-HK/librime/releases/tag/v1.1.2"
            release_asset = "rime-TypeDuck-v1.1.2-Windows-msvc-x64.7z"
            dependency_asset = "rime-deps-TypeDuck-v1.1.2-Windows-msvc-x64.7z"
            plugin = "TypeDuck-HK/rime-dictionary-lookup-filter"
            plugin_commit = "3e4605c4fae99f068df2edb85aaeab5a97752795"
            schema = "TypeDuck-HK/schema"
            schema_commit = $SchemaCommit
            capture_date = (Get-Date -Format "yyyy-MM-dd")
            capture_command = "powershell -ExecutionPolicy Bypass -File scripts/capture-typeduck-jyutping.ps1 -Fixture $Fixture"
            host = "Windows"
        }
        schema = if ($Schemas.Count -eq 1) { $Schemas[0] } else { "mixed" }
        schemas = $Schemas
        module_list = $Modules
        scenarios = $Scenarios
        capture = $Capture
        cases = @($Cases)
    }
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target\typeduck-oracle\v1.1.2"
}
$OracleRoot = FullPath $OracleRoot
$Extract = Join-Path $OracleRoot "extract"
$SharedBase = Join-Path $OracleRoot "rime-shared"
$UserBase = Join-Path $OracleRoot "rime-user"
$BuildBase = Join-Path $UserBase "build"
$SchemaRoot = Join-Path $OracleRoot "schema-src"
$CaptureRoot = Join-Path $OracleRoot "m14-capture"
if ($null -eq $Modules -or $Modules.Count -eq 0) {
    $Modules = [string[]]@("default", "dictionary_lookup")
}
if ($Modules.Count -eq 1 -and $Modules[0].Contains(",")) {
    $Modules = [string[]]($Modules[0].Split(","))
}
if (-not [string]::IsNullOrWhiteSpace($InputsFile)) {
    $RawInputs = Get-Content -LiteralPath $InputsFile -Raw -Encoding UTF8 | ConvertFrom-Json
    $Inputs = if ($RawInputs -is [System.Array]) { [string[]]$RawInputs } else { [string[]]@($RawInputs) }
}
if ($null -ne $Inputs -and $Inputs.Count -eq 1 -and $Inputs[0].Contains(",")) {
    $Inputs = [string[]]($Inputs[0].Split(","))
}

$env:PATH = (Join-Path $Extract "dist\lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH

if ($InternalCapture -or $InternalSchemaList -or $InternalStateLabels -or $InternalScenarioCapture -or $InternalUserDbProbe -or $InternalUserDbImportCapture) {
    Add-Type -Path (Join-Path $RepoRoot "scripts\oracle-rime-probe.cs")
    $Identity = [RimeProbe]::TypeDuckV112Identity()
    if ($InternalCapture) {
        if ($null -eq $Inputs -or $Inputs.Count -eq 0) {
            throw "Internal capture requires -Inputs"
        }
        $Cases = [RimeProbe]::CaptureWithIdentity(
            $Shared,
            $User,
            $Build,
            $Schema,
            [string[]]$Modules,
            [string[]]$Inputs,
            $Identity
        )
        Write-JsonFile $CasesOutput $Cases
    } elseif ($InternalSchemaList) {
        $Result = [RimeProbe]::CaptureSchemaListWithIdentity(
            $Shared,
            $User,
            $Build,
            [string[]]$Modules,
            $Identity
        )
        Write-JsonFile $CasesOutput $Result
    } elseif ($InternalStateLabels) {
        $Result = [RimeProbe]::CaptureStateLabelsWithIdentity(
            $Shared,
            $User,
            $Build,
            $Schema,
            [string[]]$Modules,
            $Identity
        )
        Write-JsonFile $CasesOutput $Result
    } elseif ($InternalScenarioCapture) {
        if ([string]::IsNullOrWhiteSpace($ScenariosFile)) {
            throw "Internal scenario capture requires -ScenariosFile"
        }
        $RawScenarios = Read-JsonFile $ScenariosFile
        $ScenarioList = if ($RawScenarios -is [System.Array]) { $RawScenarios } else { @($RawScenarios) }
        $TypedScenarios = @()
        foreach ($Scenario in $ScenarioList) {
            $TypedScenarios += New-ProbeScenario $Scenario
        }
        $Cases = [RimeProbe]::CaptureScenariosWithIdentity(
            $Shared,
            $User,
            $Build,
            $Schema,
            [string[]]$Modules,
            [RimeProbe+ProbeScenario[]]$TypedScenarios,
            $Identity
        )
        Write-JsonFile $CasesOutput $Cases
    } elseif ($InternalUserDbProbe) {
        $Result = [RimeProbe]::ProbeUserDictExportWithIdentity(
            $Shared,
            $User,
            $Build,
            $Schema,
            [string[]]$Modules,
            $TrainingInput,
            $DictName,
            $ExportPath,
            $Identity
        )
        Write-JsonFile $CasesOutput $Result
    } else {
        $Result = [RimeProbe]::CaptureImportedUserDictWithIdentity(
            $Shared,
            $User,
            $Build,
            $Schema,
            [string[]]$Modules,
            $DictName,
            $ImportPath,
            [string[]]$Inputs,
            $Identity
        )
        Write-JsonFile $CasesOutput $Result
    }
    exit 0
}

Assert-OracleInputs
$SchemaCommit = Get-SchemaCommit
New-Item -ItemType Directory -Force -Path $CaptureRoot | Out-Null

if ($Fixture -eq "All") {
    foreach ($Name in @("Smoke", "Options", "CompletionCorrection", "SchemaMenu", "UserDb", "PreferUserPhrase", "LetterToTone", "StateLabels", "PredictionRanking", "M21Closeout")) {
        & $PSCommandPath -Fixture $Name -OracleRoot $OracleRoot
        if ($LASTEXITCODE -ne 0) {
            throw "fixture capture failed for $Name with exit code $LASTEXITCODE"
        }
    }
    exit 0
}

if ([string]::IsNullOrWhiteSpace($Output)) {
    $OutputName = switch ($Fixture) {
        "Smoke" { "jyut6ping3-m14-smoke.json" }
        "Options" { "jyut6ping3-m14-options.json" }
        "CompletionCorrection" { "jyut6ping3-m14-completion-correction.json" }
        "SchemaMenu" { "jyut6ping3-m14-schema-menu.json" }
        "UserDb" { "jyut6ping3-m14-userdb.json" }
        "PreferUserPhrase" { "jyut6ping3-fork-parity-02-prefer-user-phrase.json" }
        "LetterToTone" { "jyut6ping3-fork-parity-06-letter-to-tone.json" }
        "StateLabels" { "jyut6ping3-fork-parity-07-state-labels.json" }
        "PredictionRanking" { "jyut6ping3-m21-prediction-ranking.json" }
        "M21Closeout" { "jyut6ping3-m21-closeout.json" }
    }
    $Output = Join-Path $RepoRoot (Join-Path "crates\yune-core\tests\fixtures\typeduck-v1.1.2" $OutputName)
}
$Output = FullPath $Output

if ($Fixture -eq "Smoke") {
    if ($null -eq $Inputs -or $Inputs.Count -eq 0) {
        $Inputs = [string[]]@("nei")
    }
    Invoke-Deployer $UserBase $SharedBase $BuildBase
    $Variant = [ordered]@{
        name = "smoke"
        schema = "jyut6ping3_mobile"
        patches = @()
        inputs = $Inputs
        root = Join-Path $CaptureRoot "smoke"
        shared = $SharedBase
        user = $UserBase
        build = $BuildBase
        default_custom_mode = "Keep"
    }
    New-Item -ItemType Directory -Force -Path $Variant.root | Out-Null
    $Cases = Invoke-ChildCapture $Variant
    $FixtureBody = New-Fixture "typeduck_v112_binary_smoke" @("jyut6ping3_mobile") @("smoke") $Cases ([ordered]@{ input_sequence = $Inputs })
    $FixtureBody["input_sequence"] = $Inputs
    Write-JsonFile $Output $FixtureBody
} elseif ($Fixture -eq "Options") {
    $Grave = [string][char]96
    $Variants = @(
        (New-Variant -Group "options" -Name "combine_candidates_default" -VariantSchema "jyut6ping3_mobile" -Patches ([string[]]@()) -VariantInputs ([string[]]@("hou", "nei"))),
        (New-Variant -Group "options" -Name "combine_candidates_separate" -VariantSchema "jyut6ping3_mobile" -Patches ([string[]]@("common:/separate_candidates")) -VariantInputs ([string[]]@("hou", "nei"))),
        (New-Variant -Group "options" -Name "enable_sentence_default" -VariantSchema "jyut6ping3_mobile" -Patches ([string[]]@()) -VariantInputs ([string[]]@("ngohaigo", "ngohaige"))),
        (New-Variant -Group "options" -Name "enable_sentence_disabled" -VariantSchema "jyut6ping3_mobile" -Patches ([string[]]@("common:/disable_sentence")) -VariantInputs ([string[]]@("ngohaigo", "ngohaige"))),
        (New-Variant -Group "options" -Name "show_full_code_default" -VariantSchema "jyut6ping3" -Patches ([string[]]@()) -VariantInputs ([string[]]@(($Grave + "ca"), ($Grave + "cam"), ($Grave + "cd"))) -DefaultCustomMode "Remove"),
        (New-Variant -Group "options" -Name "show_full_code_enabled" -VariantSchema "jyut6ping3" -Patches ([string[]]@("common:/show_full_code")) -VariantInputs ([string[]]@(($Grave + "ca"), ($Grave + "cam"), ($Grave + "cd"))) -DefaultCustomMode "Remove")
    )
    $Cases = foreach ($Variant in $Variants) { Invoke-ChildCapture $Variant }
    $FixtureBody = New-Fixture "typeduck_v112_deploy_time_option_variants" @("jyut6ping3_mobile", "jyut6ping3") @("combine_candidates", "show_full_code", "enable_sentence") $Cases ([ordered]@{})
    Write-JsonFile $Output $FixtureBody
} elseif ($Fixture -eq "CompletionCorrection") {
    $Variants = @(
        (New-Variant -Group "completion-correction" -Name "completion_default" -VariantSchema "jyut6ping3_mobile" -Patches ([string[]]@()) -VariantInputs ([string[]]@("n", "ne", "ng"))),
        (New-Variant -Group "completion-correction" -Name "completion_disabled" -VariantSchema "jyut6ping3_mobile" -Patches ([string[]]@("common:/disable_completion")) -VariantInputs ([string[]]@("n", "ne", "ng"))),
        (New-Variant -Group "completion-correction" -Name "correction_default" -VariantSchema "jyut6ping3_mobile" -Patches ([string[]]@()) -VariantInputs ([string[]]@("nri", "mgoi", "ngoi"))),
        (New-Variant -Group "completion-correction" -Name "correction_enabled" -VariantSchema "jyut6ping3_mobile" -Patches ([string[]]@("common:/enable_correction")) -VariantInputs ([string[]]@("nri", "mgoi", "ngoi")))
    )
    $Cases = foreach ($Variant in $Variants) { Invoke-ChildCapture $Variant }
    $FixtureBody = New-Fixture "typeduck_v112_deploy_time_completion_correction_variants" @("jyut6ping3_mobile") @("enable_completion", "enable_correction") $Cases ([ordered]@{})
    Write-JsonFile $Output $FixtureBody
} elseif ($Fixture -eq "SchemaMenu") {
    $Variants = @(
        (New-Variant -Group "schema-menu" -Name "one_schema_default" -VariantSchema "jyut6ping3" -Patches ([string[]]@()) -VariantInputs ([string[]]@("nei")) -DefaultCustomMode "Remove"),
        (New-Variant -Group "schema-menu" -Name "mobile_multi_schema_custom" -VariantSchema "jyut6ping3_mobile" -Patches ([string[]]@()) -VariantInputs ([string[]]@("nei")) -DefaultCustomMode "Keep")
    )
    $Cases = foreach ($Variant in $Variants) { [pscustomobject](Invoke-ChildSchemaList $Variant) }
    $Finding = [ordered]@{
        oracle_observable_surface = "RimeGetSchemaList emits selected schema rows; hide_lone_schema/hide_caret are switcher/frontend decoration, not candidate ABI rows"
        m16_delivery = "assert TypeDuck-Web UI behavior against one-schema and multi-schema emitted lists"
    }
    $FixtureBody = New-Fixture "typeduck_v112_schema_list_emitted_surface" @("jyut6ping3", "jyut6ping3_mobile") @("schema_menu_hiding") $Cases $Finding
    Write-JsonFile $Output $FixtureBody
} elseif ($Fixture -eq "UserDb") {
    $Modules = [string[]]@("default", "dictionary_lookup", "levers")
    $Variant = New-Variant -Group "userdb" -Name "levers_export_probe" -VariantSchema "jyut6ping3_mobile" -Patches ([string[]]@()) -VariantInputs ([string[]]@($TrainingInput))
    $Result = Invoke-ChildUserDbProbe $Variant
    $Cases = @([ordered]@{
        variant = $Variant.name
        schema_id = $Variant.schema
        training_input = $TrainingInput
        probe = $Result
    })
    $Finding = [ordered]@{
        feasibility_spike = "trained one highlighted candidate, called RimeSyncUserData, then tried RimeFindModule('levers') and RimeLeversApi.export_user_dict"
        dict_name = $DictName
    }
    $FixtureBody = New-Fixture "typeduck_v112_userdb_levers_export_probe" @("jyut6ping3_mobile") @("per_entry_userdb_pronunciation") $Cases $Finding
    Write-JsonFile $Output $FixtureBody
} elseif ($Fixture -eq "PreferUserPhrase") {
    $Modules = [string[]]@("default", "dictionary_lookup", "levers")
    $CaseSpecs = @(
        [ordered]@{
            name = "equal_code_low_commit_user_phrase"
            inputs = [string[]]@("nei")
            import_text = "YUNELOW`tnei5`t1`n"
        },
        [ordered]@{
            name = "equal_code_high_commit_user_phrase"
            inputs = [string[]]@("nei")
            import_text = "YUNEHIGH`tnei5`t100000000`n"
        },
        [ordered]@{
            name = "longer_code_user_phrase"
            inputs = [string[]]@("neihou")
            import_text = "YUNELONG`tnei5 hou2`t1`n"
        }
    )
    $Cases = foreach ($Spec in $CaseSpecs) {
        $Variant = New-Variant `
            -Group "prefer-user-phrase" `
            -Name $Spec.name `
            -VariantSchema "jyut6ping3_mobile" `
            -Patches ([string[]]@()) `
            -VariantInputs ([string[]]$Spec.inputs) `
            -SchemaCustomPatchLines ([string[]]@("translator/enable_user_dict: true", "translator/encode_commit_history: true"))
        $Result = Invoke-ChildUserDbImportCapture $Variant $Spec.import_text
        [ordered]@{
            variant = $Variant.name
            schema_id = $Variant.schema
            patches = $Variant.patches
            default_custom_mode = $Variant.default_custom_mode
            schema_custom_patch_lines = $Variant.schema_custom_patch_lines
            import_text = $Spec.import_text
            probe = $Result
        }
    }
    $Finding = [ordered]@{
        import_path = "RimeLeversApi.import_user_dict"
        dict_name = $DictName
        expected_behavior = "User phrases are visible through userdb, longer-code phrases are preferred, and equal-code phrases are not preferred by code length alone."
    }
    $FixtureBody = New-Fixture "typeduck_v112_prefer_user_phrase_weighted_gate" @("jyut6ping3_mobile") @("prefer_user_phrase") $Cases $Finding
    Write-JsonFile $Output $FixtureBody
} elseif ($Fixture -eq "LetterToTone") {
    if ($null -eq $Inputs -or $Inputs.Count -eq 0) {
        $Inputs = [string[]]@("neiv", "neivv", "neix", "neixx", "neiq", "neiqq")
    }
    $Variant = New-Variant `
        -Group "fork-parity-06" `
        -Name "letter_to_tone_mobile" `
        -VariantSchema "jyut6ping3_mobile" `
        -Patches ([string[]]@()) `
        -VariantInputs $Inputs
    $Cases = Invoke-ChildCapture $Variant
    $Finding = [ordered]@{
        input_sequence = $Inputs
        oracle_observable_surface = "RimeGetContext composition preedit maps TypeDuck v/x/q tone letters to Jyutping tone digits while RimeGetInput preserves raw letters"
    }
    $FixtureBody = New-Fixture "typeduck_v112_letter_to_tone_preedit" @("jyut6ping3_mobile") @("letter_to_tone") $Cases $Finding
    $FixtureBody["input_sequence"] = $Inputs
    Write-JsonFile $Output $FixtureBody
} elseif ($Fixture -eq "StateLabels") {
    $Variant = New-Variant `
        -Group "fork-parity-07" `
        -Name "state_labels_mobile" `
        -VariantSchema "jyut6ping3_mobile" `
        -Patches ([string[]]@()) `
        -VariantInputs ([string[]]@())
    $Cases = Invoke-ChildStateLabels $Variant
    $Finding = [ordered]@{
        oracle_observable_surface = "RimeGetStateLabel full_shape returns TypeDuck Traditional labels"
        schema_source_file = "TypeDuck-HK/schema/template.yaml"
        deployed_schema_file = "jyut6ping3_mobile.schema.yaml"
    }
    $FixtureBody = New-Fixture "typeduck_v112_full_shape_state_labels" @("jyut6ping3_mobile") @("state_labels") $Cases $Finding
    Write-JsonFile $Output $FixtureBody
} elseif ($Fixture -eq "PredictionRanking") {
    if ($null -eq $Inputs -or $Inputs.Count -eq 0) {
        $Inputs = [string[]]@("santai", "sigin", "gwongdung", "hoenggong")
    }
    $Variant = New-Variant `
        -Group "m21-prediction-ranking" `
        -Name "prediction_ranking_mobile" `
        -VariantSchema "jyut6ping3_mobile" `
        -Patches ([string[]]@()) `
        -VariantInputs $Inputs
    $Cases = Invoke-ChildCapture $Variant
    $Finding = [ordered]@{
        input_sequence = $Inputs
        oracle_observable_surface = "RimeGetContext selected_candidates records the TypeDuck v1.1.2 long-prediction interleave with leading single-character matches"
        prediction_source = "TypeDuck-HK/librime v1.1.2 script_translator Dictionary::lookup_table and PrepareCandidate prediction path"
        prediction_threshold = "kPredictionThreshold = log(100)"
    }
    $FixtureBody = New-Fixture "typeduck_v112_prediction_count_interleave" @("jyut6ping3_mobile") @("prediction_ranking", "prefix_fallback") $Cases $Finding
    $FixtureBody["input_sequence"] = $Inputs
    Write-JsonFile $Output $FixtureBody
} elseif ($Fixture -eq "M21Closeout") {
    if ($null -eq $Inputs -or $Inputs.Count -eq 0) {
        $Inputs = [string[]]@("nei", "ngo", "m", "mgoi", "ngohaigo", "hou", "neivv")
    }
    $CombinedVariant = New-Variant `
        -Group "m21-closeout" `
        -Name "default_combined" `
        -VariantSchema "jyut6ping3_mobile" `
        -Patches ([string[]]@()) `
        -VariantInputs $Inputs
    $Cases = @(Invoke-ChildCapture $CombinedVariant)

    $SimplificationVariant = New-Variant `
        -Group "m21-closeout" `
        -Name "simplification_on" `
        -VariantSchema "jyut6ping3_mobile" `
        -Patches ([string[]]@()) `
        -VariantInputs ([string[]]@())
    $Scenarios = @(
        [pscustomobject]@{
            name = "hk2s_ngohaigo_simplification_on"
            input = "ngohaigo"
            actions = @(
                [pscustomobject]@{ type = "set_option"; option = "simplification"; value = 1 },
                [pscustomobject]@{ type = "input"; text = "ngohaigo" },
                [pscustomobject]@{ type = "snapshot"; label = "simplification_on" }
            )
        }
    )
    $Cases += Invoke-ChildScenarioCapture $SimplificationVariant $Scenarios
    $Finding = [ordered]@{
        input_sequence = $Inputs
        scenario_sequence = @("hk2s_ngohaigo_simplification_on")
        oracle_observable_surface = "RimeGetContext selected_candidates closes the M21 product-comparison ledger rows against TypeDuck v1.1.2, with combine_candidates on and simplification on only for the hk2s scenario"
        settings_profile = "default_combined uses no common.custom patches so translator/combine_candidates stays true; simplification_on uses runtime RimeSetOption('simplification', 1)"
    }
    $FixtureBody = New-Fixture "typeduck_v112_m21_product_comparison_closeout" @("jyut6ping3_mobile") @("product_comparison_closeout", "hk2s") $Cases $Finding
    $FixtureBody["input_sequence"] = $Inputs
    Write-JsonFile $Output $FixtureBody
}

Write-Host "Wrote $Output"
