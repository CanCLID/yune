from __future__ import annotations

import ctypes
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent
SCRIPT = SCRIPTS / "benchmark-native-rime-inprocess.ps1"


def ps_quote(value: Path | str) -> str:
    return "'" + str(value).replace("'", "''") + "'"


class NativeBenchmarkScriptTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = SCRIPT.read_text(encoding="utf-8-sig")

    def run_function_harness(self, body: str) -> subprocess.CompletedProcess[str]:
        powershell = shutil.which("powershell")
        if powershell is None:
            self.skipTest("Windows PowerShell is unavailable")
        harness = f"""
$ErrorActionPreference = 'Stop'
$Tokens = $null
$Errors = $null
$Ast = [System.Management.Automation.Language.Parser]::ParseFile(
    {ps_quote(SCRIPT)},
    [ref]$Tokens,
    [ref]$Errors
)
if ($Errors.Count -gt 0) {{ throw ($Errors -join "`n") }}
$Wanted = @(
    'Assert-PlainFileSystemPath',
    'Get-FinalPhysicalPath',
    'Get-CanonicalSafePath',
    'Assert-BenchmarkSourcePolicy',
    'Assert-Path',
    'Build-NativeBenchmarkExecutable',
    'Invoke-NativeBenchmarkLogged',
    'File-Sha256',
    'Bytes-Sha256',
    'Tree-Sha256',
    'Get-RepositorySourceSnapshot',
    'Assert-RepositorySourceSnapshot',
    'Test-PathWithinOrEqual',
    'Assert-ExplicitRootOutsideRepo',
    'Clear-DirectoryUnder',
    'Initialize-BenchmarkRoot'
)
$Functions = $Ast.FindAll({{
    param($Node)
    $Node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $Wanted -contains $Node.Name
}}, $true)
foreach ($Function in $Functions) {{ Invoke-Expression $Function.Extent.Text }}
if ($Functions.Count -ne $Wanted.Count) {{
    throw "expected $($Wanted.Count) safety functions, found $($Functions.Count)"
}}
$RepoRoot = {ps_quote(REPO_ROOT)}
{body}
"""
        return subprocess.run(
            [powershell, "-NoProfile", "-Command", "-"],
            input=harness,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_script_parses_and_defaults_to_current_product_tree(self) -> None:
        powershell = shutil.which("powershell")
        if powershell is not None:
            command = f"""
$Errors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
    {ps_quote(SCRIPT)},
    [ref]$null,
    [ref]$Errors
) | Out-Null
if ($Errors.Count -gt 0) {{ $Errors | ForEach-Object {{ $_.ToString() }}; exit 1 }}
"""
            result = subprocess.run(
                [powershell, "-NoProfile", "-Command", "-"],
                input=command,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

        self.assertIn('[string]$ProductSchemaRoot', self.source)
        self.assertIn('[string]$WorkRoot', self.source)
        self.assertIn('apps\\yune-web\\public\\schema', self.source)
        self.assertNotIn('apps\\yune-web\\source\\public\\schema', self.source)

    def test_external_roots_are_create_new_and_legacy_cleanup_is_bounded(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            output = base / "external-output"
            legacy = base / "legacy"
            legacy.mkdir()
            legacy_child = legacy / "run"
            legacy_child.mkdir()
            marker = legacy_child / "stale.txt"
            marker.write_text("stale", encoding="utf-8")
            body = f"""
Initialize-BenchmarkRoot {ps_quote(output)} {ps_quote(legacy)} $true 'OutputRoot'
if (-not (Test-Path -LiteralPath {ps_quote(output)} -PathType Container)) {{
    throw 'explicit external root was not created'
}}
$ReuseRejected = $false
try {{
    Initialize-BenchmarkRoot {ps_quote(output)} {ps_quote(legacy)} $true 'OutputRoot'
}}
catch {{
    $ReuseRejected = $_.Exception.Message -like '*must be a new path*'
}}
if (-not $ReuseRejected) {{ throw 'explicit root reuse was not rejected' }}
Initialize-BenchmarkRoot {ps_quote(legacy_child)} {ps_quote(legacy)} $false 'WorkRoot'
if (Test-Path -LiteralPath {ps_quote(marker)}) {{ throw 'legacy child was not cleared' }}
"""
            result = self.run_function_harness(body)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_explicit_roots_must_be_disjoint_from_repo(self) -> None:
        self.assertIn(
            'Assert-ExplicitRootOutsideRepo $OutputRoot $OutputRootWasProvided "OutputRoot"',
            self.source,
        )
        self.assertIn(
            'Assert-ExplicitRootOutsideRepo $WorkRoot $WorkRootWasProvided "WorkRoot"',
            self.source,
        )
        body = """
$Rejected = 0
foreach ($Unsafe in @($RepoRoot, (Join-Path $RepoRoot 'target\\benchmark'), (Split-Path -Parent $RepoRoot))) {
    try {
        Assert-ExplicitRootOutsideRepo $Unsafe $true 'OutputRoot'
    }
    catch {
        if ($_.Exception.Message -like '*must be disjoint*') { $Rejected += 1 }
    }
}
if ($Rejected -ne 3) { throw "expected three unsafe roots to be rejected, got $Rejected" }
Assert-ExplicitRootOutsideRepo (Join-Path $RepoRoot 'target\\native-inprocess') $false 'WorkRoot'
"""
        result = self.run_function_harness(body)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_device_path_alias_and_reparse_ancestor_are_rejected(self) -> None:
        body = r"""
$Extended = "\\?\" + (Join-Path $RepoRoot 'target\benchmark-escape')
$ExtendedRejected = $false
try { Get-CanonicalSafePath $Extended 'OutputRoot' | Out-Null }
catch { $ExtendedRejected = $_.Exception.Message -like '*device or extended path*' }
if (-not $ExtendedRejected) { throw 'extended path alias was not rejected' }
"""
        result = self.run_function_harness(body)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            target = base / "junction-target"
            link = base / "junction-link"
            target.mkdir()
            created = subprocess.run(
                ["cmd", "/c", "mklink", "/J", str(link), str(target)],
                capture_output=True,
                text=True,
                check=False,
            )
            if created.returncode != 0:
                self.skipTest(f"cannot create test junction: {created.stderr}")
            try:
                body = f"""
$Rejected = $false
try {{ Get-CanonicalSafePath {ps_quote(link / 'child')} 'WorkRoot' | Out-Null }}
catch {{ $Rejected = $_.Exception.Message -like '*reparse-point ancestor*' }}
if (-not $Rejected) {{ throw 'junction ancestor was not rejected' }}
"""
                result = self.run_function_harness(body)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            finally:
                if link.exists():
                    link.rmdir()

    def test_short_name_alias_canonicalizes_before_repo_disjointness(self) -> None:
        buffer = ctypes.create_unicode_buffer(32768)
        length = ctypes.windll.kernel32.GetShortPathNameW(
            str(REPO_ROOT), buffer, len(buffer)
        )
        self.assertGreater(length, 0)
        short_repo = buffer.value
        if not short_repo or short_repo.casefold() == str(REPO_ROOT).casefold():
            self.skipTest("8.3 short-name alias is unavailable")
        candidate = Path(short_repo) / "target" / "benchmark-escape"
        body = f"""
$Canonical = Get-CanonicalSafePath {ps_quote(candidate)} 'OutputRoot'
$Rejected = $false
try {{ Assert-ExplicitRootOutsideRepo $Canonical $true 'OutputRoot' }}
catch {{ $Rejected = $_.Exception.Message -like '*must be disjoint*' }}
if (-not $Rejected) {{ throw '8.3 repo alias bypassed disjointness' }}
"""
        result = self.run_function_harness(body)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_subst_volume_alias_canonicalizes_before_repo_disjointness(self) -> None:
        drive = next(
            (
                f"{letter}:"
                for letter in reversed("DEFGHIJKLMNOPQRSTUVWXYZ")
                if not Path(f"{letter}:/").exists()
            ),
            None,
        )
        if drive is None:
            self.skipTest("no free drive letter for SUBST regression")
        created = subprocess.run(
            ["subst", drive, str(REPO_ROOT)],
            capture_output=True,
            text=True,
            check=False,
        )
        if created.returncode != 0:
            self.skipTest(f"cannot create SUBST alias: {created.stderr}")
        try:
            candidate = Path(f"{drive}/target/benchmark-escape")
            body = f"""
$Canonical = Get-CanonicalSafePath {ps_quote(candidate)} 'OutputRoot'
$Rejected = $false
try {{ Assert-ExplicitRootOutsideRepo $Canonical $true 'OutputRoot' }}
catch {{ $Rejected = $_.Exception.Message -like '*must be disjoint*' }}
if (-not $Rejected) {{ throw "SUBST repo alias bypassed disjointness: $Canonical" }}
"""
            result = self.run_function_harness(body)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        finally:
            subprocess.run(
                ["subst", drive, "/D"],
                capture_output=True,
                text=True,
                check=False,
            )

    def test_tree_hash_is_stable_and_rejects_reparse_descendants(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            tree = base / "tree"
            target = base / "junction-target"
            link = tree / "linked"
            tree.mkdir()
            target.mkdir()
            (tree / "b.txt").write_text("b", encoding="utf-8")
            (tree / "a.txt").write_text("a", encoding="utf-8")
            body = f"""
$First = Tree-Sha256 {ps_quote(tree)}
$Second = Tree-Sha256 {ps_quote(tree)}
if ($First -ne $Second -or $First -notmatch '^[0-9a-f]{{64}}$') {{
    throw 'tree hash is not stable SHA-256'
}}
"""
            result = self.run_function_harness(body)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

            created = subprocess.run(
                ["cmd", "/c", "mklink", "/J", str(link), str(target)],
                capture_output=True,
                text=True,
                check=False,
            )
            if created.returncode != 0:
                self.skipTest(f"cannot create test junction: {created.stderr}")
            try:
                body = f"""
$Rejected = $false
try {{ Tree-Sha256 {ps_quote(tree)} | Out-Null }}
catch {{ $Rejected = $_.Exception.Message -like '*contains a reparse point*' }}
if (-not $Rejected) {{ throw 'tree hash traversed a reparse descendant' }}
"""
                result = self.run_function_harness(body)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            finally:
                if link.exists():
                    link.rmdir()

    def test_dirty_source_is_diagnostic_only_and_unsigned(self) -> None:
        body = """
Assert-BenchmarkSourcePolicy @() $false ''
Assert-BenchmarkSourcePolicy @(' M engine.rs') $true ''
$DirtyRejected = $false
try { Assert-BenchmarkSourcePolicy @(' M engine.rs') $false '' }
catch { $DirtyRejected = $_.Exception.Message -like '*source must be clean*' }
if (-not $DirtyRejected) { throw 'dirty signed source was not rejected' }
$ThresholdRejected = $false
try { Assert-BenchmarkSourcePolicy @() $true 'signed-thresholds.csv' }
catch { $ThresholdRejected = $_.Exception.Message -like '*diagnostic-only*' }
if (-not $ThresholdRejected) { throw 'AllowDirty with thresholds was not rejected' }
"""
        result = self.run_function_harness(body)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_source_snapshot_rejects_post_build_or_final_content_drift(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            tracked = repo / "tracked.txt"
            untracked = repo / "untracked.txt"
            tracked.write_text("before\n", encoding="utf-8")
            commands = (
                ["git", "init", "--quiet"],
                ["git", "config", "user.name", "Yune Test"],
                ["git", "config", "user.email", "yune@example.invalid"],
                ["git", "add", "tracked.txt"],
                ["git", "commit", "--quiet", "-m", "fixture"],
            )
            for command in commands:
                result = subprocess.run(
                    command,
                    cwd=repo,
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            untracked.write_text("untracked-before\n", encoding="utf-8")
            body = f"""
$RepoRoot = {ps_quote(repo)}
$Expected = Get-RepositorySourceSnapshot
Assert-RepositorySourceSnapshot $Expected 'unchanged'
[System.IO.File]::AppendAllText({ps_quote(tracked)}, "after`n")
$Rejected = $false
try {{ Assert-RepositorySourceSnapshot $Expected 'post-build' }}
catch {{ $Rejected = $_.Exception.Message -like '*source drifted at post-build*' }}
if (-not $Rejected) {{ throw 'tracked content drift was not rejected' }}
& git -C $RepoRoot checkout -- tracked.txt
if ($LASTEXITCODE -ne 0) {{ throw 'failed to restore tracked fixture' }}
$ExpectedUntracked = Get-RepositorySourceSnapshot
$ExpectedUntrackedBinding = $ExpectedUntracked.ContentBindingSha256
$BeforeUntrackedHash = File-Sha256 {ps_quote(untracked)}
[System.IO.File]::AppendAllText({ps_quote(untracked)}, "untracked-after`n")
$AfterUntrackedHash = File-Sha256 {ps_quote(untracked)}
$ObservedUntracked = Get-RepositorySourceSnapshot
$UntrackedRejected = $false
$UntrackedMessage = ''
try {{ Assert-RepositorySourceSnapshot $ExpectedUntracked 'final' }}
catch {{
    $UntrackedMessage = $_.Exception.Message
    $UntrackedRejected = $UntrackedMessage -like '*source drifted at final*'
}}
if (-not $UntrackedRejected) {{
    throw "untracked content drift was not rejected: $UntrackedMessage binding=$ExpectedUntrackedBinding/$($ObservedUntracked.ContentBindingSha256) file=$BeforeUntrackedHash/$AfterUntrackedHash status=$($ExpectedUntracked.StatusRows -join ',')/$($ObservedUntracked.StatusRows -join ',')"
}}
"""
            result = self.run_function_harness(body)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

        self.assertIn(
            'Assert-RepositorySourceSnapshot $InitialSourceSnapshot "post-build"',
            self.source,
        )
        self.assertIn(
            'Assert-RepositorySourceSnapshot $InitialSourceSnapshot "final"',
            self.source,
        )

    def test_supplied_dll_skips_release_build_and_is_immutable(self) -> None:
        self.assertEqual(
            self.source.count(
                'Invoke-Logged "cargo-build-release-yune-rime-api"'
            ),
            1,
        )
        build_guard = re.compile(
            r"\$BuildPerformed\s*=\s*-not\s+\$YuneDllWasProvided\s*"
            r"if\s*\(\$BuildPerformed\)\s*\{.*?"
            r'Invoke-Logged\s+"cargo-build-release-yune-rime-api"',
            re.DOTALL,
        )
        self.assertRegex(self.source, build_guard)
        self.assertIn(
            'if ((File-Sha256 $YuneDll) -ne $YuneDllSha256)', self.source
        )
        self.assertIn('"yune_dll_supplied=$YuneDllWasProvided"', self.source)
        self.assertIn('"build_performed=$BuildPerformed"', self.source)

    def test_benchmark_is_compiled_once_then_fixed_executable_is_invoked(self) -> None:
        self.assertEqual(
            self.source.count("Build-NativeBenchmarkExecutable "),
            1,
        )
        self.assertEqual(
            self.source.count("Invoke-NativeBenchmarkLogged \"$OutputName"),
            2,
        )
        self.assertNotIn(
            '"bench", "-p", "yune-rime-api", "--bench", '
            '"native_inprocess_benchmark", "--"',
            self.source,
        )
        self.assertIn(
            'if ((File-Sha256 $NativeBenchmarkExecutable) -ne '
            '$NativeBenchmarkExecutableSha256)',
            self.source,
        )

    def test_cargo_json_resolves_one_exact_benchmark_executable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fake_bin = root / "bin"
            fake_bin.mkdir()
            executable = root / "native_inprocess_benchmark.exe"
            executable.write_bytes(b"fixed-benchmark")
            direct = root / "direct-benchmark.cmd"
            direct.write_text("@echo off\r\necho %*\r\n", encoding="ascii")
            executable_json = executable.as_posix()
            (fake_bin / "cargo.cmd").write_text(
                "@echo off\r\n"
                "echo {\"reason\":\"compiler-artifact\","
                "\"target\":{\"name\":\"native_inprocess_benchmark\","
                "\"kind\":[\"bench\"]},"
                f'\"executable\":\"{executable_json}\"}}\r\n',
                encoding="ascii",
            )
            body = f"""
$PreviousCargoTarget = $env:CARGO_TARGET_DIR
$env:PATH = {ps_quote(fake_bin)} + ';' + $env:PATH
$BenchmarkCargoTargetRoot = {ps_quote(root / 'cargo-target')}
$Resolved = Build-NativeBenchmarkExecutable {ps_quote(root / 'build.log')}
if ([System.IO.Path]::GetFullPath([string]$Resolved) -ne [System.IO.Path]::GetFullPath({ps_quote(executable)})) {{
    throw "wrong benchmark executable: $Resolved"
}}
if ($env:CARGO_TARGET_DIR -ne $PreviousCargoTarget) {{
    throw 'CARGO_TARGET_DIR was not restored'
}}
$NativeBenchmarkExecutable = {ps_quote(direct)}
$PreviousPath = $env:PATH
Invoke-NativeBenchmarkLogged 'direct-smoke' @('--engine', 'yune') {ps_quote(root / 'direct.log')}
if ($env:PATH -ne $PreviousPath) {{ throw 'PATH was not restored' }}
$DirectLog = Get-Content -LiteralPath {ps_quote(root / 'direct.log')} -Raw
if ($DirectLog -notlike '*--engine yune*') {{ throw 'fixed benchmark executable was not invoked directly' }}
"""
            result = self.run_function_harness(body)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_upstream_trees_and_terminal_status_are_fail_closed(self) -> None:
        for field in (
            "upstream_shared_tree_sha256",
            "upstream_build_tree_sha256",
        ):
            self.assertIn(f'"{field}=', self.source)
        self.assertIn(
            "if ((Tree-Sha256 $SharedSource) -ne $UpstreamSharedTreeSha256)",
            self.source,
        )
        self.assertIn(
            "if ((Tree-Sha256 $BuildSource) -ne $UpstreamBuildTreeSha256)",
            self.source,
        )
        self.assertIn('Set-RunStatus "in-progress"', self.source)
        self.assertIn('Set-RunStatus "complete"', self.source)
        self.assertIn('Set-RunStatus "failed" $FailureMessage', self.source)

    def test_replay_command_and_provenance_name_every_fixed_input(self) -> None:
        for flag in (
            '"-OutputRoot"',
            '"-WorkRoot"',
            '"-UpstreamOracleRoot"',
            '"-ProductSchemaRoot"',
            '"-YuneDll"',
        ):
            self.assertIn(flag, self.source)

        for field in (
            "source_commit",
            "source_tree",
            "source_content_binding_sha256",
            "measured_yune_dll_sha256",
            "upstream_rime_dll_sha256",
            "product_schema_tree_sha256",
            "upstream_shared_tree_sha256",
            "upstream_build_tree_sha256",
            "native_benchmark_executable_sha256",
            "benchmark_script_sha256",
            "actual_invocation",
            "output_root",
            "work_root",
            "product_schema_root",
            "upstream_oracle_root",
            "yune_dll",
        ):
            self.assertIn(f'"{field}=', self.source)

        self.assertIn('"actual-invocation.txt"', self.source)
        self.assertIn('"external-provenance.txt"', self.source)


if __name__ == "__main__":
    unittest.main()
