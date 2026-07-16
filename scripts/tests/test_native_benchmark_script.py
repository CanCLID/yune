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
MACOS_SCRIPT = SCRIPTS / "benchmark-native-rime-inprocess-macos.sh"
POET_REBIND_TOOL = SCRIPTS / "rebind-m61-luna-poet-checksum.py"
OUTPUT_POLICY = SCRIPTS / "evidence-output-path.ps1"
OUTPUT_POLICY_TOOL = SCRIPTS / "evidence-output-path.py"
RETENTION_TOOL = SCRIPTS / "evidence_retention.py"


def ps_quote(value: Path | str) -> str:
    return "'" + str(value).replace("'", "''") + "'"


class NativeBenchmarkScriptTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = SCRIPT.read_text(encoding="utf-8-sig")
        cls.macos_source = MACOS_SCRIPT.read_text(encoding="utf-8")

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
    'Assert-FileOutsideRoot',
    'Resolve-PrebuiltNativeBenchmarkExecutable',
    'Resolve-PrebuiltNativeBenchmarkReceipt',
    'Write-NativeBenchmarkBuildReceipt',
    'Read-NativeBenchmarkBuildReceipt',
    'Assert-NativeBenchmarkBuildReceipt',
    'Select-NativeBenchmarkExecutable',
    'Invoke-PoetRebindLogged',
    'Assert-ExplicitRootOutsideRepo',
    'Clear-DirectoryUnder',
    'Initialize-BenchmarkRoot',
    'Initialize-CreateNewOutputRoot'
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

    def test_output_path_policy_does_not_write_python_bytecode_into_source(self) -> None:
        powershell = shutil.which("powershell")
        if powershell is None:
            self.skipTest("Windows PowerShell is unavailable")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repository = root / "repository"
            scripts = repository / "scripts"
            scripts.mkdir(parents=True)
            for source in (OUTPUT_POLICY, OUTPUT_POLICY_TOOL, RETENTION_TOOL):
                shutil.copy2(source, scripts / source.name)
            for arguments in (
                ("init", "--quiet"),
                ("config", "user.name", "M61 Test"),
                ("config", "user.email", "m61-test@example.invalid"),
                ("add", "."),
                ("commit", "--quiet", "-m", "fixture"),
            ):
                result = subprocess.run(
                    ["git", "-C", str(repository), *arguments],
                    text=True,
                    capture_output=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

            external = root / "external" / "run"
            command = f"""
$ErrorActionPreference = 'Stop'
. {ps_quote(scripts / OUTPUT_POLICY.name)}
$Result = Invoke-YuneEvidenceOutputPathPolicy @(
    'validate',
    '--repo-root', {ps_quote(repository)},
    '--path', {ps_quote(external)}
)
if ($Result -ne [System.IO.Path]::GetFullPath({ps_quote(external)})) {{
    throw "unexpected resolved path: $Result"
}}
"""
            result = subprocess.run(
                [
                    powershell,
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    "-",
                ],
                input=command,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertFalse(
                (scripts / "__pycache__").exists(),
                "the path-policy helper must not write Python bytecode into source",
            )
            status = subprocess.run(
                [
                    "git",
                    "-C",
                    str(repository),
                    "status",
                    "--porcelain=v1",
                    "--untracked-files=all",
                ],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(status.returncode, 0, status.stdout + status.stderr)
            self.assertEqual(status.stdout, "")

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

    def test_output_evidence_is_always_create_new_and_never_cleared(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "output"
            marker = output / "preserve.txt"
            body = f"""
Initialize-CreateNewOutputRoot {ps_quote(output)} 'OutputRoot'
[System.IO.File]::WriteAllText({ps_quote(marker)}, 'preserve')
$Rejected = $false
try {{ Initialize-CreateNewOutputRoot {ps_quote(output)} 'OutputRoot' }}
catch {{ $Rejected = $_.Exception.Message -like '*refusing to clear or reuse*' }}
if (-not $Rejected) {{ throw 'existing output evidence was reused' }}
if (-not (Test-Path -LiteralPath {ps_quote(marker)} -PathType Leaf)) {{
    throw 'existing output evidence was cleared'
}}
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

    @unittest.skipUnless(hasattr(ctypes, "windll"), "Windows path aliases are unavailable")
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

    @unittest.skipUnless(shutil.which("subst"), "Windows SUBST is unavailable")
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

    def test_prebuilt_mode_skips_build_and_build_mode_calls_it_once(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prebuilt = root / "stable harness with spaces.exe"
            built = root / "newly-built.exe"
            build_log = root / "build log with spaces.txt"
            prebuilt.write_bytes(b"prebuilt")
            body = f"""
$script:NativeBenchmarkBuildCount = 0
function Build-NativeBenchmarkExecutable([string]$LogPath) {{
    $script:NativeBenchmarkBuildCount += 1
    return {ps_quote(built)}
}}
$Reused = Select-NativeBenchmarkExecutable $false {ps_quote(prebuilt)} {ps_quote(build_log)}
if ($script:NativeBenchmarkBuildCount -ne 0) {{
    throw 'prebuilt reuse unexpectedly invoked the Cargo build'
}}
if ($Reused -ne {ps_quote(prebuilt)}) {{ throw "wrong reused executable: $Reused" }}
$Built = Select-NativeBenchmarkExecutable $true {ps_quote(prebuilt)} {ps_quote(build_log)}
if ($script:NativeBenchmarkBuildCount -ne 1) {{
    throw "build mode invoked the Cargo build $script:NativeBenchmarkBuildCount times"
}}
if ($Built -ne {ps_quote(built)}) {{ throw "wrong built executable: $Built" }}
"""
            result = self.run_function_harness(body)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_prebuilt_paths_must_be_external_existing_plain_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / "output"
            work = root / "work"
            stable = root / "stable files"
            output.mkdir()
            work.mkdir()
            stable.mkdir()
            executable = stable / "native benchmark with spaces.exe"
            receipt = stable / "native benchmark receipt with spaces.txt"
            executable.write_bytes(b"stable-prebuilt-harness")
            receipt.write_text("format_version=1\n", encoding="utf-8")
            output_file = output / executable.name
            work_file = work / executable.name
            output_file.write_bytes(b"output-owned")
            work_file.write_bytes(b"work-owned")
            directory = stable / "directory-not-file"
            directory.mkdir()
            missing = stable / "missing.exe"
            body = f"""
$Executable = Resolve-PrebuiltNativeBenchmarkExecutable `
    {ps_quote(executable)} {ps_quote(output)} {ps_quote(work)}
$Receipt = Resolve-PrebuiltNativeBenchmarkReceipt `
    {ps_quote(receipt)} {ps_quote(output)} {ps_quote(work)}
if ($Executable -ne [System.IO.Path]::GetFullPath({ps_quote(executable)})) {{
    throw "safe executable was not canonicalized: $Executable"
}}
if ($Receipt -ne [System.IO.Path]::GetFullPath({ps_quote(receipt)})) {{
    throw "safe receipt was not canonicalized: $Receipt"
}}
$Rejected = 0
foreach ($Unsafe in @(
    {ps_quote(output_file)},
    {ps_quote(work_file)},
    {ps_quote(directory)},
    {ps_quote(missing)},
    ({ps_quote(executable)} + ':stream')
)) {{
    try {{
        Resolve-PrebuiltNativeBenchmarkExecutable `
            $Unsafe {ps_quote(output)} {ps_quote(work)} | Out-Null
    }}
    catch {{ $Rejected += 1 }}
}}
if ($Rejected -ne 5) {{ throw "expected five unsafe paths to fail, got $Rejected" }}
"""
            result = self.run_function_harness(body)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(executable.read_bytes(), b"stable-prebuilt-harness")
            self.assertEqual(output_file.read_bytes(), b"output-owned")
            self.assertEqual(work_file.read_bytes(), b"work-owned")
            self.assertFalse(missing.exists())

    def test_build_receipt_is_deterministic_and_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            receipt = Path(temporary) / "native benchmark receipt.txt"
            body = f"""
$Fields = [ordered]@{{
    format_version = '1'
    source_commit = ('a' * 40)
    source_tree = ('b' * 40)
    source_clean = 'True'
    source_content_binding_sha256 = ('c' * 64)
    benchmark_script_sha256 = ('d' * 64)
    benchmark_rust_source_sha256 = ('e' * 64)
    cargo_lock_sha256 = ('f' * 64)
    rustc_identity_sha256 = ('1' * 64)
    cargo_identity_sha256 = ('2' * 64)
    cargo_command = 'cargo bench --no-run'
    native_benchmark_build_command = "`$env:CARGO_TARGET_DIR='C:\\stable target'; cargo bench --no-run"
    cargo_target_root = 'C:\\stable target'
    native_benchmark_executable_path = 'C:\\stable target\\bench with spaces.exe'
    native_benchmark_executable_sha256 = ('3' * 64)
}}
Write-NativeBenchmarkBuildReceipt {ps_quote(receipt)} $Fields
$Receipt = Read-NativeBenchmarkBuildReceipt {ps_quote(receipt)}
Assert-NativeBenchmarkBuildReceipt $Receipt $Fields
$MismatchRejected = $false
$ExpectedMismatch = [ordered]@{{
    native_benchmark_executable_sha256 = ('4' * 64)
}}
try {{ Assert-NativeBenchmarkBuildReceipt $Receipt $ExpectedMismatch }}
catch {{ $MismatchRejected = $_.Exception.Message -like '*mismatch*' }}
if (-not $MismatchRejected) {{ throw 'receipt hash mismatch was accepted' }}
$Receipt.Remove('cargo_lock_sha256')
$MissingRejected = $false
try {{ Assert-NativeBenchmarkBuildReceipt $Receipt $Fields }}
catch {{ $MissingRejected = $_.Exception.Message -like '*missing cargo_lock_sha256*' }}
if (-not $MissingRejected) {{ throw 'incomplete receipt was accepted' }}
$OverwriteRejected = $false
try {{ Write-NativeBenchmarkBuildReceipt {ps_quote(receipt)} $Fields }}
catch {{ $OverwriteRejected = $_.Exception.Message -like '*Refusing to overwrite*' }}
if (-not $OverwriteRejected) {{ throw 'receipt overwrite was accepted' }}
"""
            result = self.run_function_harness(body)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            receipt_bytes = receipt.read_bytes()
            self.assertFalse(receipt_bytes.startswith(b"\xef\xbb\xbf"))
            self.assertNotIn(b"\r", receipt_bytes)
            self.assertTrue(receipt_bytes.endswith(b"\n"))

    def test_prebuilt_receipt_provenance_and_final_hashes_are_recorded(self) -> None:
        for parameter in (
            "[string]$PrebuiltNativeBenchmarkExecutable",
            "[string]$PrebuiltNativeBenchmarkReceipt",
        ):
            self.assertIn(parameter, self.source)
        for field in (
            "native_benchmark_executable_prebuilt",
            "native_benchmark_build_performed",
            "native_benchmark_receipt_sha256",
        ):
            self.assertGreaterEqual(self.source.count(field), 3)
        for receipt_field in (
            "source_content_binding_sha256",
            "benchmark_rust_source_sha256",
            "cargo_lock_sha256",
            "rustc_identity_sha256",
            "cargo_identity_sha256",
            "native_benchmark_build_command",
            "cargo_target_root",
        ):
            self.assertIn(receipt_field, self.source)
        self.assertIn('"-PrebuiltNativeBenchmarkExecutable",', self.source)
        self.assertIn('"-PrebuiltNativeBenchmarkReceipt",', self.source)
        self.assertIn(
            "if ((File-Sha256 $NativeBenchmarkReceiptInput) -ne "
            "$NativeBenchmarkReceiptInputSha256)",
            self.source,
        )
        self.assertIn(
            "Prebuilt native benchmark build receipt changed during validation",
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

    def test_poet_sidecar_is_rebound_only_after_restoring_pinned_build(self) -> None:
        self.assertTrue(POET_REBIND_TOOL.is_file())
        for source in (self.source, self.macos_source):
            self.assertIn("rebind-m61-luna-poet-checksum.py", source)
            self.assertIn("poet_rebind_tool_sha256", source)
            self.assertIn("track-a-yune-poet-rebind.txt", source)
            self.assertIn("--dictionary-source", source)
            self.assertIn("--restored-table", source)
            self.assertIn("--generated-poet", source)
            self.assertIn("--output-poet", source)

        windows_restore = self.source.index(
            "Copy-DirectoryContents $TrackAOriginalBuild $TrackAYuneBuild"
        )
        windows_rebind = self.source.index(
            "Invoke-PoetRebindLogged $PoetRebindArgs $TrackAPoetRebindLog"
        )
        windows_timing = self.source.index(
            "Invoke-TrackAYuneBench $TrackAYuneRun $UpstreamDistLib",
            windows_restore,
        )
        self.assertLess(windows_restore, windows_rebind)
        self.assertLess(windows_rebind, windows_timing)
        self.assertIn('"-B", $PoetRebindTool', self.source)
        self.assertIn("track-a-yune-poet-rebind.log", self.source)
        self.assertIn(
            '"--dictionary-source", (Join-Path $TrackAYuneRun "shared\\luna_pinyin.dict.yaml")',
            self.source,
        )
        self.assertIn(
            '$InputDrift += "M61 Luna POET checksum rebind tool changed during the benchmark:',
            self.source,
        )

        windows_capture_start = self.source.index("function Invoke-PoetRebindLogged")
        windows_capture_end = self.source.index(
            "function Prepare-UpstreamRun", windows_capture_start
        )
        windows_capture = self.source[windows_capture_start:windows_capture_end]
        native_invoke = windows_capture.index(
            "& python @ArgumentList 1> $StdOut 2> $StdErr"
        )
        eap_restore = windows_capture.index(
            "$ErrorActionPreference = $PreviousErrorActionPreference", native_invoke
        )
        log_write = windows_capture.index("$LogPath,", eap_restore)
        failure = windows_capture.index("if ($ExitCode -ne 0)", log_write)
        self.assertLess(native_invoke, eap_restore)
        self.assertLess(eap_restore, log_write)
        self.assertLess(log_write, failure)
        self.assertIn('$ErrorActionPreference = "Continue"', windows_capture)
        self.assertIn("diagnostics: $LogPath", windows_capture)
        self.assertIn(
            "Remove-Item -LiteralPath $StdOut, $StdErr -Force -ErrorAction SilentlyContinue",
            windows_capture,
        )

        macos_restore = self.macos_source.index(
            'copy_dir_contents "$track_a_original_build" "$track_a_yune_run/user/build"'
        )
        macos_rebind = self.macos_source.index(
            'if python3 -B "$poet_rebind_tool"', macos_restore
        )
        macos_log_redirect = self.macos_source.index(
            '> "$track_a_poet_rebind_log" 2>&1', macos_rebind
        )
        macos_failure = self.macos_source.index(
            'die "M61 Luna POET checksum rebind failed', macos_log_redirect
        )
        macos_timing = self.macos_source.index(
            'run_cargo_bench "track-a-yune"', macos_restore
        )
        self.assertLess(macos_restore, macos_rebind)
        self.assertLess(macos_rebind, macos_log_redirect)
        self.assertLess(macos_log_redirect, macos_failure)
        self.assertLess(macos_rebind, macos_timing)
        self.assertIn(
            'cp "$track_a_poet_rebind_log" "$track_a_poet_rebind_receipt"',
            self.macos_source,
        )
        self.assertIn("diagnostics: $track_a_poet_rebind_log", self.macos_source)
        self.assertIn(
            'die "M61 Luna POET checksum rebind tool changed during macOS measurement."',
            self.macos_source,
        )

    def test_poet_rebind_failure_is_logged_under_powershell_stop_preference(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            log = root / "poet-rebind.log"
            receipt = root / "poet-rebind.txt"
            stdout_tmp = root / "track-a-yune-poet-rebind.stdout.tmp"
            stderr_tmp = root / "track-a-yune-poet-rebind.stderr.tmp"
            failure_script = root / "reject-poet.py"
            failure_script.write_text(
                'import sys\nsys.stderr.write("owned rejection diagnostics\\n")\n'
                "raise SystemExit(7)\n",
                encoding="utf-8",
            )
            body = f"""
$Before = $ErrorActionPreference
$Arguments = @('-B', {ps_quote(failure_script)})
$Caught = ''
try {{
    Invoke-PoetRebindLogged $Arguments {ps_quote(log)} {ps_quote(receipt)}
    throw 'expected rebind failure was not raised'
}}
catch {{
    $Caught = $_.Exception.Message
}}
if ($ErrorActionPreference -ne $Before) {{ throw 'ErrorActionPreference was not restored' }}
if (-not (Test-Path -LiteralPath {ps_quote(log)} -PathType Leaf)) {{ throw 'durable log missing' }}
$LogText = [System.IO.File]::ReadAllText({ps_quote(log)})
if ($LogText -notlike '*owned rejection diagnostics*') {{ throw "diagnostic missing: $LogText" }}
if ($Caught -notlike '*exit code 7*diagnostics:*') {{ throw "wrong failure: $Caught" }}
if (Test-Path -LiteralPath {ps_quote(receipt)}) {{ throw 'failed run wrote success receipt' }}
if (Test-Path -LiteralPath {ps_quote(stdout_tmp)}) {{ throw 'stdout temp survived' }}
if (Test-Path -LiteralPath {ps_quote(stderr_tmp)}) {{ throw 'stderr temp survived' }}
"""
            result = self.run_function_harness(body)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("owned rejection diagnostics", log.read_text(encoding="utf-8-sig"))
            self.assertFalse(receipt.exists())

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
