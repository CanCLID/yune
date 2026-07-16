from __future__ import annotations

import csv
import os
import re
import shutil
import shlex
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent
POWERSHELL_WRAPPER = SCRIPTS / "benchmark-native-rime-inprocess.ps1"
MACOS_WRAPPER = SCRIPTS / "benchmark-native-rime-inprocess-macos.sh"
AGGREGATOR = SCRIPTS / "aggregate-native-ratchet.py"
BENCHMARK = (
    REPO_ROOT
    / "crates"
    / "yune-rime-api"
    / "benches"
    / "native_inprocess_benchmark.rs"
)


def ps_quote(value: Path | str) -> str:
    return "'" + str(value).replace("'", "''") + "'"


class M61NativeModeContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.powershell = POWERSHELL_WRAPPER.read_text(encoding="utf-8-sig")
        cls.macos = MACOS_WRAPPER.read_text(encoding="utf-8")
        cls.aggregator = AGGREGATOR.read_text(encoding="utf-8")
        cls.benchmark = BENCHMARK.read_text(encoding="utf-8")

    def test_selectors_have_the_frozen_names_and_defaults(self) -> None:
        self.assertIn(
            '[ValidateSet("production-default", "owned", "byte-backed")]',
            self.powershell,
        )
        self.assertIn(
            '[string]$TrackAStorageMode = "production-default"', self.powershell
        )
        self.assertIn("--track-a-storage-mode)", self.macos)
        self.assertIn('track_a_storage_mode="production-default"', self.macos)
        self.assertRegex(
            self.macos,
            r"production-default\|owned\|byte-backed\)",
        )
        for source in (self.powershell, self.macos, self.benchmark):
            self.assertIn("track_a_storage_mode", source)
        self.assertIn('"track_a_storage_mode",', self.aggregator)

    def test_wrappers_preserve_the_omitted_invocation_and_reject_ambiguity(
        self,
    ) -> None:
        self.assertIn(
            '$PSBoundParameters.ContainsKey("TrackAStorageMode")',
            self.powershell,
        )
        self.assertIn("track_a_storage_mode_explicit=0", self.macos)
        self.assertIn("track_a_storage_mode_explicit=1", self.macos)
        self.assertIn("YUNE_POET_BYTE_BACKED is inherited", self.powershell)
        self.assertIn("SetEnvironmentVariableW", self.powershell)
        self.assertIn("Win32Exception", self.powershell)
        self.assertIn(
            "YUNE_POET_BYTE_BACKED must be absent when "
            "--track-a-storage-mode is explicit",
            self.macos,
        )

    def test_environment_is_scoped_to_track_a_yune_and_restored(self) -> None:
        timing = re.search(
            r"function Invoke-TrackAYuneBench\(.*?\n\}",
            self.powershell,
            re.DOTALL,
        )
        self.assertIsNotNone(timing)
        timing_source = timing.group(0)
        self.assertIn('$TrackAStorageMode -eq "byte-backed"', timing_source)
        self.assertIn(
            'Set-PoetByteBackedEnvironmentState $true "1"',
            timing_source,
        )
        self.assertIn("finally {", timing_source)
        self.assertIn(
            "Restore-PoetByteBackedEnvironmentState",
            timing_source,
        )
        self.assertIn(
            'Assert-TrackAPoetEnvironmentRestored `\n'
            '            "Track A Yune timing"',
            timing_source,
        )
        self.assertLess(
            self.powershell.index(
                'Assert-TrackAPoetEnvironmentRestored `\n'
                '    "same-run librime comparison"'
            ),
            self.powershell.index(
                'Run-NativeBench "librime-1.17.0"'
            ),
        )
        self.assertLess(
            self.powershell.index(
                'Assert-TrackAPoetEnvironmentRestored `\n'
                '        "Track B product guard"'
            ),
            self.powershell.index(
                'Run-NativeBench "yune" "track-b-product"'
            ),
        )

        self.assertIn('local poet_byte_backed="${13:-0}"', self.macos)
        self.assertIn("assert_poet_byte_backed_restored", self.macos)
        self.assertIn("YUNE_POET_BYTE_BACKED=1 \\", self.macos)
        self.assertIn(
            'assert_poet_byte_backed_restored "before Track A librime timing"',
            self.macos,
        )
        self.assertIn(
            'assert_poet_byte_backed_restored "before Track B timing"',
            self.macos,
        )

    def test_powershell_restores_on_success_and_failure_when_available(
        self,
    ) -> None:
        powershell = shutil.which("powershell")
        if powershell is None:
            self.skipTest("Windows PowerShell is unavailable")
        harness = f"""
$ErrorActionPreference = 'Stop'
$Tokens = $null
$Errors = $null
$Ast = [System.Management.Automation.Language.Parser]::ParseFile(
    {ps_quote(POWERSHELL_WRAPPER)},
    [ref]$Tokens,
    [ref]$Errors
)
if ($Errors.Count -gt 0) {{ throw ($Errors -join "`n") }}
$Wanted = @(
    'Get-PoetByteBackedEnvironmentState',
    'Set-PoetByteBackedEnvironmentState',
    'Restore-PoetByteBackedEnvironmentState',
    'Assert-TrackAPoetEnvironmentRestored',
    'Invoke-TrackAPoetDeployPrep',
    'Invoke-TrackAYuneBench'
)
$Functions = $Ast.FindAll({{
    param($Node)
    $Node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $Wanted -contains $Node.Name
}}, $true)
foreach ($Function in $Functions) {{ Invoke-Expression $Function.Extent.Text }}
if ($Functions.Count -ne $Wanted.Count) {{
    throw 'missing Track A environment helper'
}}

$TrackAStorageModeWasProvided = $true
$TrackAStorageMode = 'byte-backed'
$TrackAInputs = 'n'
$script:FailTiming = $false
$script:FailDeploy = $false
$script:ObservedTimingValue = ''
$script:ObservedTimingPresent = $false
$script:ObservedDeployValue = ''
$script:ObservedDeployPresent = $false
function Run-NativeBench {{
    $State = Get-PoetByteBackedEnvironmentState
    $script:ObservedTimingValue = $State.Value
    $script:ObservedTimingPresent = $State.Present
    if ($script:FailTiming) {{ throw 'injected timing failure' }}
}}
function Invoke-DeployPrep {{
    $State = Get-PoetByteBackedEnvironmentState
    $script:ObservedDeployValue = $State.Value
    $script:ObservedDeployPresent = $State.Present
    if ($script:FailDeploy) {{ throw 'injected deploy failure' }}
}}
Set-PoetByteBackedEnvironmentState $false $null
Invoke-TrackAYuneBench 'run' 'path'
if (-not $script:ObservedTimingPresent -or
    $script:ObservedTimingValue -cne '1') {{
    throw 'byte-backed mode was not scoped'
}}
if ($null -ne [Environment]::GetEnvironmentVariable(
    'YUNE_POET_BYTE_BACKED', 'Process'
)) {{ throw 'success path leaked the selector' }}

$script:FailTiming = $true
$Failed = $false
try {{ Invoke-TrackAYuneBench 'run' 'path' }}
catch {{ $Failed = $_.Exception.Message -like '*injected timing failure*' }}
if (-not $Failed) {{ throw 'injected timing failure was not preserved' }}
if ($null -ne [Environment]::GetEnvironmentVariable(
    'YUNE_POET_BYTE_BACKED', 'Process'
)) {{ throw 'failure path leaked the selector' }}

$TrackAStorageModeWasProvided = $false
$TrackAStorageMode = 'production-default'
Set-PoetByteBackedEnvironmentState $true ''
$EmptyState = Get-PoetByteBackedEnvironmentState
if (-not $EmptyState.Present -or $EmptyState.Value -cne '') {{
    throw 'could not create present-empty environment state'
}}
Invoke-TrackAPoetDeployPrep 'run' 'path'
if (-not $script:ObservedDeployPresent -or
    $script:ObservedDeployValue -cne '1') {{
    throw 'deploy prep did not scope byte-backed mode'
}}
$EmptyState = Get-PoetByteBackedEnvironmentState
if (-not $EmptyState.Present -or $EmptyState.Value -cne '') {{
    throw 'deploy prep did not restore present-empty state'
}}
Invoke-TrackAYuneBench 'run' 'path'
if (-not $script:ObservedTimingPresent -or
    $script:ObservedTimingValue -cne '') {{
    throw 'timing did not preserve present-empty state'
}}
$EmptyState = Get-PoetByteBackedEnvironmentState
if (-not $EmptyState.Present -or $EmptyState.Value -cne '') {{
    throw 'timing did not restore present-empty state'
}}

Set-PoetByteBackedEnvironmentState $true '0'
$script:FailDeploy = $true
$DeployFailed = $false
try {{ Invoke-TrackAPoetDeployPrep 'run' 'path' }}
catch {{ $DeployFailed = $_.Exception.Message -like '*injected deploy failure*' }}
if (-not $DeployFailed) {{ throw 'injected deploy failure was not preserved' }}
$ZeroState = Get-PoetByteBackedEnvironmentState
if (-not $ZeroState.Present -or $ZeroState.Value -cne '0') {{
    throw 'deploy failure did not restore present non-activating state'
}}
$script:FailDeploy = $false

$TrackAStorageModeWasProvided = $true
$TrackAStorageMode = 'byte-backed'
Set-PoetByteBackedEnvironmentState $true ''
$InheritedRejected = $false
try {{ Invoke-TrackAYuneBench 'run' 'path' }}
catch {{ $InheritedRejected = $_.Exception.Message -like '*ambiguous*' }}
if (-not $InheritedRejected) {{
    throw 'explicit present-empty variable was accepted by timing'
}}
$InheritedRejected = $false
try {{ Invoke-TrackAPoetDeployPrep 'run' 'path' }}
catch {{ $InheritedRejected = $_.Exception.Message -like '*ambiguous*' }}
if (-not $InheritedRejected) {{
    throw 'explicit present-empty variable was accepted by deploy prep'
}}

$TopLevelRejected = $false
try {{
    & {ps_quote(POWERSHELL_WRAPPER)} -TrackAStorageMode owned
}}
catch {{
    $TopLevelRejected = $_.Exception.Message -like `
        '*Track A storage mode is ambiguous when YUNE_POET_BYTE_BACKED is inherited*'
}}
finally {{
    Set-PoetByteBackedEnvironmentState $false $null
}}
if (-not $TopLevelRejected) {{
    throw 'top-level wrapper accepted an explicit present-empty variable'
}}
"""
        result = subprocess.run(
            [powershell, "-NoProfile", "-Command", "-"],
            input=harness,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_macos_restores_on_success_and_failure_and_rejects_ambiguity(
        self,
    ) -> None:
        helpers_start = self.macos.index("die() {")
        helpers_end = self.macos.index("\nsha256_file() {", helpers_start)
        runner_start = self.macos.index("run_cargo_bench() {")
        runner_end = self.macos.index(
            "\nprepare_upstream_run() {", runner_start
        )
        helpers = self.macos[helpers_start:helpers_end]
        runner = self.macos[runner_start:runner_end]

        target = REPO_ROOT / "target"
        target.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(
            prefix="m61-macos-mode-contract-", dir=target
        ) as temporary:
            output_root = Path(temporary)
            observed = output_root / "observed-environment.txt"
            harness = f"""
set -euo pipefail
{helpers}

clear_dir_under() {{
  mkdir -p "$2"
}}

log() {{
  :
}}

cargo() {{
  printf '%s\\n' "${{YUNE_POET_BYTE_BACKED-absent}}" > "$observed_path"
  return "$cargo_status"
}}

{runner}

repo_root={shlex.quote(str(REPO_ROOT))}
output_root={shlex.quote(str(output_root))}
observed_path={shlex.quote(str(observed))}
track_a_storage_mode=byte-backed
iterations=9
session_iterations=60
key_iterations=80
inherited_poet_byte_backed_present=0
inherited_poet_byte_backed_value=""
unset YUNE_POET_BYTE_BACKED

cargo_status=0
run_cargo_bench "success" "yune" "track-a-comparison" "luna_pinyin" \
  "fake.dylib" "shared" "user" "build" "n" "lib" 0 0 1
test "$(cat "$observed_path")" = "1"
test "${{YUNE_POET_BYTE_BACKED+x}}" != "x"

cargo_status=23
if run_cargo_bench "failure" "yune" "track-a-comparison" "luna_pinyin" \
  "fake.dylib" "shared" "user" "build" "n" "lib" 0 0 1; then
  exit 91
else
  status=$?
fi
test "$status" = "23"
test "$(cat "$observed_path")" = "1"
test "${{YUNE_POET_BYTE_BACKED+x}}" != "x"
"""
            result = subprocess.run(
                ["bash"],
                input=harness,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(
                result.returncode, 0, result.stdout + result.stderr
            )

        environment = os.environ.copy()
        environment["YUNE_POET_BYTE_BACKED"] = "0"
        ambiguous = subprocess.run(
            [
                "bash",
                str(MACOS_WRAPPER),
                "--track-a-storage-mode",
                "owned",
            ],
            env=environment,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(ambiguous.returncode, 1)
        self.assertIn(
            "YUNE_POET_BYTE_BACKED must be absent when "
            "--track-a-storage-mode is explicit",
            ambiguous.stderr,
        )

    def test_powershell_owner_shapes_and_private_receipt_when_available(
        self,
    ) -> None:
        powershell = shutil.which("powershell")
        if powershell is None:
            self.skipTest("Windows PowerShell is unavailable")
        harness = f"""
$ErrorActionPreference = 'Stop'
$Tokens = $null
$Errors = $null
$Ast = [System.Management.Automation.Language.Parser]::ParseFile(
    {ps_quote(POWERSHELL_WRAPPER)},
    [ref]$Tokens,
    [ref]$Errors
)
if ($Errors.Count -gt 0) {{ throw ($Errors -join "`n") }}
$Functions = $Ast.FindAll({{
    param($Node)
    $Node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $Node.Name -eq 'Assert-TrackAOwnerShape'
}}, $true)
foreach ($Function in $Functions) {{ Invoke-Expression $Function.Extent.Text }}
if ($Functions.Count -ne 1) {{ throw 'missing Track A owner-shape helper' }}

function New-OwnerRow {{
    param(
        [string]$OwnerId,
        [string]$MappingMode,
        [string]$StorageMode,
        [string]$RetainedEstimateBytes = '1'
    )
    [pscustomobject]@{{
        engine = 'yune'
        track = 'track-a-comparison'
        schema_id = 'luna_pinyin'
        track_a_storage_mode = $StorageMode
        owner_id = $OwnerId
        mapping_mode = $MappingMode
        retained_estimate_bytes = $RetainedEstimateBytes
    }}
}}

function Assert-Rejected {{
    param([scriptblock]$Action, [string]$Label)
    $Rejected = $false
    try {{ & $Action }} catch {{ $Rejected = $true }}
    if (-not $Rejected) {{ throw "expected rejection: $Label" }}
}}

$Owned = @(
    (New-OwnerRow 'process.owner_snapshot_private_bytes' 'process_memory' 'owned' '42'),
    (New-OwnerRow 'poet.abbreviation_vocabulary' 'Vec<ModelVocabularyEntry>' 'owned'),
    (New-OwnerRow 'poet.entries_by_code' 'Vec<ModelEntry>' 'owned'),
    (New-OwnerRow 'poet.lookup_index' 'SentenceLookupIndex' 'owned'),
    (New-OwnerRow 'poet.vocabulary' 'Vec<ModelVocabularyEntry>' 'owned')
)
$ByteBacked = @(
    (New-OwnerRow 'process.owner_snapshot_private_bytes' 'process_memory' 'byte-backed' '42'),
    (New-OwnerRow 'poet.abbreviation_vocabulary' 'poet_bin:byte_backed:mmap' 'byte-backed'),
    (New-OwnerRow 'poet.entries_by_code' 'poet_bin:byte_backed:mmap' 'byte-backed'),
    (New-OwnerRow 'poet.prefix_index' 'poet_bin:byte_backed:mmap' 'byte-backed'),
    (New-OwnerRow 'poet.vocabulary' 'poet_bin:byte_backed:mmap' 'byte-backed')
)
Assert-TrackAOwnerShape $Owned 'owned'
Assert-TrackAOwnerShape $ByteBacked 'byte-backed'

$Duplicate = @(
    $ByteBacked
    New-OwnerRow 'poet.entries_by_code' 'poet_bin:byte_backed:mmap' 'byte-backed'
)
Assert-Rejected {{ Assert-TrackAOwnerShape $Duplicate 'byte-backed' }} 'duplicate'

$Missing = @($ByteBacked | Where-Object {{ $_.owner_id -ne 'poet.prefix_index' }})
Assert-Rejected {{ Assert-TrackAOwnerShape $Missing 'byte-backed' }} 'missing'

$Extra = @(
    $ByteBacked
    New-OwnerRow 'poet.extra' 'poet_bin:byte_backed:mmap' 'byte-backed'
)
Assert-Rejected {{ Assert-TrackAOwnerShape $Extra 'byte-backed' }} 'extra'

$WrongMapping = @(
    $ByteBacked | Where-Object {{ $_.owner_id -ne 'poet.prefix_index' }}
    New-OwnerRow 'poet.prefix_index' 'owned' 'byte-backed'
)
Assert-Rejected {{ Assert-TrackAOwnerShape $WrongMapping 'byte-backed' }} 'wrong mapping'

$WithoutReceipt = @($Owned | Where-Object {{
    $_.owner_id -ne 'process.owner_snapshot_private_bytes'
}})
Assert-Rejected {{ Assert-TrackAOwnerShape $WithoutReceipt 'owned' }} 'missing receipt'
$DuplicateReceipt = @(
    $Owned
    New-OwnerRow 'process.owner_snapshot_private_bytes' 'process_memory' 'owned' '42'
)
Assert-Rejected {{ Assert-TrackAOwnerShape $DuplicateReceipt 'owned' }} 'duplicate receipt'
foreach ($InvalidValue in @(
    'unavailable',
    'not-a-number',
    '0',
    '18446744073709551616'
)) {{
    $InvalidReceipt = @(
        $WithoutReceipt
        New-OwnerRow 'process.owner_snapshot_private_bytes' 'process_memory' 'owned' $InvalidValue
    )
    Assert-Rejected {{
        Assert-TrackAOwnerShape $InvalidReceipt 'owned'
    }} "invalid receipt $InvalidValue"
}}
"""
        result = subprocess.run(
            [powershell, "-NoProfile", "-Command", "-"],
            input=harness,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_owner_shapes_are_exact_and_receipted(self) -> None:
        for owner in (
            "poet.entries_by_code",
            "poet.prefix_index",
            "poet.vocabulary",
            "poet.abbreviation_vocabulary",
            "poet.lookup_index",
        ):
            self.assertIn(owner, self.powershell)
            self.assertIn(owner, self.macos)
        for source in (self.powershell, self.macos):
            self.assertIn("poet_bin:byte_backed:mmap", source)
            self.assertIn("track-a-owner-shape.txt", source)
        self.assertIn(
            "process.owner_snapshot_private_bytes", self.powershell
        )
        self.assertIn("[UInt64]::TryParse", self.powershell)
        self.assertIn("strictly positive UInt64 receipt", self.powershell)

    def test_macos_owner_checker_rejects_duplicate_missing_and_extra_rows(
        self,
    ) -> None:
        match = re.search(
            r'python3 - "\$output_root/memory-owner-profile\.csv" '
            r'"\$track_a_storage_mode" <<\'PY\'\n(.*?)\nPY',
            self.macos,
            re.DOTALL,
        )
        self.assertIsNotNone(match)
        checker = match.group(1)

        owned = (
            ("poet.entries_by_code", "Vec<ModelEntry>"),
            ("poet.lookup_index", "SentenceLookupIndex"),
            ("poet.vocabulary", "Vec<ModelVocabularyEntry>"),
            ("poet.abbreviation_vocabulary", "Vec<ModelVocabularyEntry>"),
        )
        byte_backed = (
            ("poet.entries_by_code", "poet_bin:byte_backed:mmap"),
            ("poet.prefix_index", "poet_bin:byte_backed:mmap"),
            ("poet.vocabulary", "poet_bin:byte_backed:mmap"),
            (
                "poet.abbreviation_vocabulary",
                "poet_bin:byte_backed:mmap",
            ),
        )

        def run_checker(
            mode: str, rows: tuple[tuple[str, str], ...]
        ) -> subprocess.CompletedProcess[str]:
            with tempfile.TemporaryDirectory() as temporary:
                profile = Path(temporary) / "memory-owner-profile.csv"
                with profile.open("w", encoding="utf-8", newline="") as handle:
                    writer = csv.DictWriter(
                        handle,
                        fieldnames=[
                            "engine",
                            "track",
                            "schema_id",
                            "owner_id",
                            "mapping_mode",
                            "track_a_storage_mode",
                        ],
                        lineterminator="\n",
                    )
                    writer.writeheader()
                    for owner_id, mapping_mode in rows:
                        writer.writerow(
                            {
                                "engine": "yune",
                                "track": "track-a-comparison",
                                "schema_id": "luna_pinyin",
                                "owner_id": owner_id,
                                "mapping_mode": mapping_mode,
                                "track_a_storage_mode": mode,
                            }
                        )
                return subprocess.run(
                    ["python3", "-", str(profile), mode],
                    input=checker,
                    text=True,
                    capture_output=True,
                    check=False,
                )

        for mode, rows in (
            ("production-default", owned),
            ("owned", owned),
            ("byte-backed", byte_backed),
        ):
            with self.subTest(valid=mode):
                result = run_checker(mode, rows)
                self.assertEqual(
                    result.returncode, 0, result.stdout + result.stderr
                )

        invalid = (
            ("owned-duplicate", "owned", owned + (owned[0],)),
            ("owned-missing", "owned", owned[:-1]),
            (
                "byte-extra",
                "byte-backed",
                byte_backed + (("poet.extra", "poet_bin:byte_backed:mmap"),),
            ),
            (
                "byte-wrong-mapping",
                "byte-backed",
                byte_backed[:-1]
                + (("poet.abbreviation_vocabulary", "owned"),),
            ),
        )
        for label, mode, rows in invalid:
            with self.subTest(invalid=label):
                result = run_checker(mode, rows)
                self.assertNotEqual(result.returncode, 0)

    def test_owner_snapshot_is_phase_aligned_and_non_owner(self) -> None:
        sample = self.benchmark.index(
            "let owner_snapshot_memory = current_memory_sample();"
        )
        snapshot = self.benchmark.index("let rows = exports.snapshot();", sample)
        self.assertLess(sample, snapshot)
        between = self.benchmark[sample:snapshot]
        self.assertNotIn("run_benchmark", between)
        self.assertIn(
            '"process.owner_snapshot_private_bytes"',
            self.benchmark,
        )
        self.assertIn(
            "engine,track,schema_id,track_a_storage_mode,session_id,owner_id",
            self.benchmark,
        )
        self.assertIn('"unclassified"', self.benchmark)

    def test_aggregator_covers_both_fixed_binary_shapes_and_private_envelope(
        self,
    ) -> None:
        self.assertIn(
            'build_then_reuse = [("False", "True")] + '
            '[("True", "False")] *',
            self.aggregator,
        )
        self.assertIn(
            'all_prebuilt = [("True", "False")] * len(runs)',
            self.aggregator,
        )
        self.assertIn("observed_modes not in (build_then_reuse, all_prebuilt)", self.aggregator)
        self.assertIn("track_a_storage_mode", self.aggregator)
        self.assertIn("def _read_track_a_private_envelope(", self.aggregator)
        self.assertIn("median_private_bytes", self.aggregator)
        self.assertIn("return max(observations.values())", self.aggregator)

    def test_macos_wrapper_has_valid_shell_syntax(self) -> None:
        result = subprocess.run(
            ["bash", "-n", str(MACOS_WRAPPER)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
