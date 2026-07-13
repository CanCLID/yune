import json
import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
RUNNER = ROOT / "scripts" / "verify-m59-web04-native.ps1"
FIXTURE = (
    ROOT
    / "crates"
    / "yune-core"
    / "tests"
    / "fixtures"
    / "upstream-octagram"
    / "lotem-luna-pinyin-octagram.json"
)


class M59Web04NativeRunnerTests(unittest.TestCase):
    def test_runner_uses_the_pinned_fixture_and_fails_closed(self) -> None:
        source = RUNNER.read_text(encoding="utf-8")
        self.assertIn("lotem-luna-pinyin-octagram.json", source)
        self.assertIn("574c99d100f422766c433c601ed6efd642e881d69a30df9fffb6f1695be550e3", source)
        self.assertIn("$ExpectedModelBytes = 10513408", source)
        self.assertIn("$Fixture.observed_octagram_differences", source)
        self.assertIn("Get-OracleTop @($Fixture.null_grammar_control)", source)
        self.assertIn("Get-OracleTop @($Fixture.cases)", source)
        self.assertIn("cargo build --manifest-path", source)
        self.assertIn("--release -p yune-cli", source)
        self.assertIn("release/yune-cli.exe", source)
        self.assertIn("[ValidatePattern('^[0-9a-fA-F]{40}$')]", source)
        self.assertIn("if ($SourceCommit -ne $ExpectedSourceCommit)", source)
        self.assertIn("if ($GitStatus.Count -ne 0)", source)
        self.assertIn("source worktree must be clean", source)
        self.assertIn("$RepoRoot = $RepoRoot.TrimEnd([char[]]@(", source)
        self.assertIn("$RepoPrefix = $RepoRoot + [System.IO.Path]::DirectorySeparatorChar", source)
        self.assertIn("$ScratchPath.StartsWith($RepoPrefix", source)
        self.assertIn("must stay outside the tracked repository", source)
        self.assertIn("-ExpectedYuneCliSha256 is required", source)
        self.assertIn("yune-cli sha256 mismatch", source)
        self.assertIn("'source-built-release'", source)
        self.assertIn("'reused-expected'", source)
        self.assertIn("provenance_mode = $YuneCliProvenanceMode", source)
        self.assertIn("cargo_profile = $YuneCliCargoProfile", source)
        self.assertIn("source_clean = $true", source)
        self.assertIn("source_dirty = $false", source)
        mandatory_source = """[Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-fA-F]{40}$')]
    [string]$ExpectedSourceCommit"""
        self.assertIn(mandatory_source, source)
        clean_guard = source.index("if ($GitStatus.Count -ne 0)")
        scratch_guard = source.index("must stay outside the tracked repository")
        output_creation = source.index(
            "New-Item -ItemType Directory -Path $OutputRoot"
        )
        build_or_reuse = source.index("if (-not $YuneCli)")
        reused_hash_guard = source.index(
            "if ($ReusedYuneCli -and "
            "[string]::IsNullOrWhiteSpace($ExpectedYuneCliSha256))"
        )
        hash_comparison = source.index(
            "if ($ExpectedYuneCliSha256 -and "
            "$YuneCliSha256 -ne $ExpectedYuneCliSha256)"
        )
        self.assertLess(clean_guard, output_creation)
        self.assertLess(scratch_guard, output_creation)
        self.assertLess(reused_hash_guard, build_or_reuse)
        self.assertGreater(hash_comparison, source.index("$YuneCliSha256 = Get-Sha256"))
        self.assertIn("Write-Utf8NoBom (Join-Path $CaseUserRoot 'default.custom.yaml')", source)
        self.assertIn("- schema: luna_pinyin_octagram", source)
        self.assertIn("context.page_size -le 0", source)
        self.assertIn("$CaseUserRoot = Join-Path $UserRoot $Lane.Name", source)
        self.assertIn("one fresh root per lane", source)
        self.assertIn("ConvertTo-WindowsCommandLineArgument", source)
        self.assertIn("New-Object System.Diagnostics.ProcessStartInfo", source)
        self.assertIn("$StdoutTask = $Process.StandardOutput.ReadToEndAsync()", source)
        self.assertIn("$ExitCode = $Process.ExitCode", source)
        self.assertIn("$Process.Kill()", source)
        self.assertIn("-Raw -Encoding UTF8 | ConvertFrom-Json", source)
        self.assertIn("$ResultArray = $Results.ToArray()", source)
        self.assertIn("exit 1", source)

    def test_fixture_defines_four_distinct_plain_and_octagram_tops(self) -> None:
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        inputs = fixture["observed_octagram_differences"]
        self.assertEqual(len(inputs), 4)

        def tops(rows: list[dict]) -> dict[str, str]:
            return {
                row["input"]: row["selected_candidates"][0]["text"]
                for row in rows
                if row["input"] in inputs
            }

        plain = tops(fixture["null_grammar_control"])
        octagram = tops(fixture["cases"])
        self.assertEqual(set(plain), set(inputs))
        self.assertEqual(set(octagram), set(inputs))
        self.assertTrue(all(plain[input_] != octagram[input_] for input_ in inputs))


if __name__ == "__main__":
    unittest.main()
