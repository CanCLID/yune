from __future__ import annotations

import csv
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPOSITORY_ROOT / "scripts" / "build-evidence-pruning-ledger.py"


class EvidencePruningLedgerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.repo = Path(self.temporary.name)
        self.git("init", "-q")
        self.git("config", "user.email", "test@example.com")
        self.git("config", "user.name", "Test")
        self.write("docs/summary.md", "retained summary\n")
        self.git("add", ".")
        self.git("commit", "-qm", "base")
        self.base_commit = self.git("rev-parse", "HEAD").stdout.strip()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def git(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", *args],
            cwd=self.repo,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def write(self, relative: str, content: str | bytes) -> None:
        path = self.repo / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(content, bytes):
            path.write_bytes(content)
        else:
            path.write_text(content, encoding="utf-8")

    def run_builder(
        self,
        *,
        recovery: str,
        allowlist: str,
        check: bool = False,
        dependency_treeish: str | None = None,
    ) -> subprocess.CompletedProcess[str]:
        allowlist_path = self.repo / "allowlist.csv"
        allowlist_path.write_text(allowlist, encoding="utf-8")
        command = [
            sys.executable,
            str(SCRIPT),
            "--repo-root",
            str(self.repo),
            "--recovery-commit",
            recovery,
            "--allowlist",
            str(allowlist_path),
            "--retained-summary",
            "docs/summary.md",
            "--ledger-out",
            str(self.repo / "ledger.csv"),
            "--dependency-out",
            str(self.repo / "dependencies.csv"),
        ]
        if dependency_treeish:
            command.extend(("--dependency-treeish", dependency_treeish))
        if check:
            command.append("--check")
        return subprocess.run(
            command,
            cwd=self.repo,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def populate_candidates(self) -> str:
        evidence_metric = "docs/reports/evidence/packet-a/raw/m37_metrics.csv"
        self.write(evidence_metric, "metric,value\nlatency,1\n")
        self.write(
            "docs/reports/evidence/packet-a/raw/rsmarisa-test.marisa", b"marisa-bytes"
        )
        fixture = "apps/yune-web/e2e/results/fixture-a/samples.csv"
        self.write(fixture, "sample,value\n1,2\n")
        self.write(
            "docs/reports/history/archived-receipts.md",
            f"Historical receipt: `{evidence_metric}`.\n"
            "Historical binary: `docs/reports/evidence/packet-a/raw/rsmarisa-test.marisa`.\n",
        )
        self.write(
            "docs/reports/evidence/packet-a/README.md", "Compact packet summary.\n"
        )
        self.write(
            "docs/reports/evidence/packet-a/packet-manifest.csv",
            "path,sha256\nraw/m37_metrics.csv,fixture-hash\n",
        )
        self.git("add", ".")
        self.git("commit", "-qm", "candidates")
        return self.git("rev-parse", "HEAD").stdout.strip()

    def test_builds_sorted_recovery_bound_ledger_and_dependency_scan(self) -> None:
        recovery = self.populate_candidates()
        allowlist = (
            "path,removal_class,reason,retained_summary_pointer\n"
            "apps/yune-web/e2e/results/fixture-a/samples.csv,retain-fixture,"
            "browser fixture,docs/summary.md\n"
        )
        result = self.run_builder(recovery=recovery, allowlist=allowlist)
        self.assertEqual(result.returncode, 0, result.stderr)
        with (self.repo / "ledger.csv").open(newline="", encoding="utf-8") as handle:
            rows = list(csv.DictReader(handle))
        self.assertEqual([row["path"] for row in rows], sorted(row["path"] for row in rows))
        self.assertEqual(len(rows), 3)
        by_path = {row["path"]: row for row in rows}
        marisa = by_path[
            "docs/reports/evidence/packet-a/raw/rsmarisa-test.marisa"
        ]
        self.assertEqual(marisa["removal_class"], "archive-remove")
        self.assertEqual(marisa["packet"], "packet-a")
        self.assertEqual(marisa["byte_size"], str(len(b"marisa-bytes")))
        self.assertEqual(marisa["recovery_commit"], recovery)
        self.assertEqual(
            by_path["docs/reports/evidence/packet-a/raw/m37_metrics.csv"][
                "removal_class"
            ],
            "archive-remove",
        )
        expected_oid = self.git(
            "rev-parse",
            f"{recovery}:docs/reports/evidence/packet-a/raw/rsmarisa-test.marisa",
        ).stdout.strip()
        self.assertEqual(marisa["git_blob_sha1"], expected_oid)
        with (self.repo / "dependencies.csv").open(
            newline="", encoding="utf-8"
        ) as handle:
            dependencies = list(csv.DictReader(handle))
        direct = [row for row in dependencies if row["candidate_path"]]
        self.assertEqual(len(direct), 3)
        self.assertEqual(
            len(dependencies),
            len({tuple(row.items()) for row in dependencies}),
        )
        self.assertEqual(
            {row["disposition"] for row in direct},
            {"historical-reference", "historical-manifest-reference"},
        )
        check = self.run_builder(recovery=recovery, allowlist=allowlist, check=True)
        self.assertEqual(check.returncode, 0, check.stderr)

    def test_rejects_recovery_commit_without_candidate_bytes(self) -> None:
        current = self.populate_candidates()
        allowlist = (
            "path,removal_class,reason,retained_summary_pointer\n"
            "apps/yune-web/e2e/results/fixture-a/samples.csv,retain-fixture,"
            "browser fixture,docs/summary.md\n"
        )
        result = self.run_builder(recovery=self.base_commit, allowlist=allowlist)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("recovery commit cannot restore", result.stderr)
        self.assertNotEqual(current, self.base_commit)

    def test_rejects_missing_recovery_commit(self) -> None:
        self.populate_candidates()
        allowlist = (
            "path,removal_class,reason,retained_summary_pointer\n"
            "apps/yune-web/e2e/results/fixture-a/samples.csv,retain-fixture,"
            "browser fixture,docs/summary.md\n"
        )
        result = self.run_builder(recovery="f" * 40, allowlist=allowlist)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not a commit present", result.stderr)

    def test_rejects_duplicate_or_unsorted_allowlist(self) -> None:
        recovery = self.populate_candidates()
        duplicate = (
            "path,removal_class,reason,retained_summary_pointer\n"
            "apps/yune-web/e2e/results/fixture-a/samples.csv,retain-fixture,a,\n"
            "apps/yune-web/e2e/results/fixture-a/samples.csv,retain-fixture,b,\n"
        )
        duplicate_result = self.run_builder(recovery=recovery, allowlist=duplicate)
        self.assertNotEqual(duplicate_result.returncode, 0)
        self.assertIn("duplicate allowlist path", duplicate_result.stderr)
        unsorted = (
            "path,removal_class,reason,retained_summary_pointer\n"
            "docs/reports/evidence/packet-a/raw/m37_metrics.csv,retain-dependency,b,\n"
            "apps/yune-web/e2e/results/fixture-a/samples.csv,retain-fixture,a,\n"
        )
        unsorted_result = self.run_builder(recovery=recovery, allowlist=unsorted)
        self.assertNotEqual(unsorted_result.returncode, 0)
        self.assertIn("must be sorted", unsorted_result.stderr)

    def test_rejects_unallowlisted_code_dependency(self) -> None:
        recovery = self.populate_candidates()
        candidate = "docs/reports/evidence/packet-a/raw/m37_metrics.csv"
        self.write("scripts/consumer.py", f'RECEIPT = "{candidate}"\n')
        self.git("add", ".")
        self.git("commit", "-qm", "code dependency")
        recovery = self.git("rev-parse", "HEAD").stdout.strip()
        allowlist = (
            "path,removal_class,reason,retained_summary_pointer\n"
            "apps/yune-web/e2e/results/fixture-a/samples.csv,retain-fixture,"
            "browser fixture,docs/summary.md\n"
        )
        result = self.run_builder(recovery=recovery, allowlist=allowlist)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("current dependencies require", result.stderr)

    def test_rejects_current_links_including_backslash_and_absolute_paths(self) -> None:
        recovery = self.populate_candidates()
        allowlist = (
            "path,removal_class,reason,retained_summary_pointer\n"
            "apps/yune-web/e2e/results/fixture-a/samples.csv,retain-fixture,"
            "browser fixture,docs/summary.md\n"
        )
        current = self.repo / "docs/reports/evidence/packet-a/README.md"
        current.write_text(
            "Windows leaf: `docs\\reports\\evidence\\packet-a\\raw\\m37_metrics.csv`.\n"
            "Absolute leaf: `/checkout/yune/docs/reports/evidence/packet-a/raw/rsmarisa-test.marisa`.\n",
            encoding="utf-8",
        )
        self.git("add", current.relative_to(self.repo).as_posix())
        dependency_tree = self.git("write-tree").stdout.strip()
        result = self.run_builder(
            recovery=recovery,
            allowlist=allowlist,
            dependency_treeish=dependency_tree,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("current dependencies require", result.stderr)

    def test_dependency_treeish_scans_post_pruning_tree(self) -> None:
        recovery = self.populate_candidates()
        allowlist = (
            "path,removal_class,reason,retained_summary_pointer\n"
            "apps/yune-web/e2e/results/fixture-a/samples.csv,retain-fixture,"
            "browser fixture,docs/summary.md\n"
        )
        self.write(
            "docs/reports/evidence/packet-a/README.md",
            "Current leaf: `raw/m37_metrics.csv`.\n",
        )
        self.git("add", "docs/reports/evidence/packet-a/README.md")
        blocked_tree = self.git("write-tree").stdout.strip()
        blocked = self.run_builder(
            recovery=recovery,
            allowlist=allowlist,
            dependency_treeish=blocked_tree,
        )
        self.assertNotEqual(blocked.returncode, 0)

        self.write(
            "docs/reports/evidence/packet-a/README.md",
            "Raw leaves are archived in docs/ledgers/evidence-pruning/current-ledger.csv.\n",
        )
        self.git("add", "docs/reports/evidence/packet-a/README.md")
        safe_tree = self.git("write-tree").stdout.strip()
        safe = self.run_builder(
            recovery=recovery,
            allowlist=allowlist,
            dependency_treeish=safe_tree,
        )
        self.assertEqual(safe.returncode, 0, safe.stderr)

    def test_ambiguous_generic_output_suffix_is_not_a_dependency(self) -> None:
        recovery = self.populate_candidates()
        second = "docs/reports/evidence/packet-b/raw/m37_metrics.csv"
        self.write(second, "metric,value\nlatency,2\n")
        self.write(
            "scripts/benchmark.py",
            'OUTPUT = "raw/m37_metrics.csv"\n',
        )
        self.git("add", ".")
        self.git("commit", "-qm", "ambiguous output shape")
        recovery = self.git("rev-parse", "HEAD").stdout.strip()
        allowlist = (
            "path,removal_class,reason,retained_summary_pointer\n"
            "apps/yune-web/e2e/results/fixture-a/samples.csv,retain-fixture,"
            "browser fixture,docs/summary.md\n"
        )
        result = self.run_builder(recovery=recovery, allowlist=allowlist)
        self.assertEqual(result.returncode, 0, result.stderr)
        with (self.repo / "dependencies.csv").open(
            newline="", encoding="utf-8"
        ) as handle:
            rows = list(csv.DictReader(handle))
        ambiguous = [
            row for row in rows if row["reference_kind"] == "ambiguous-path-suffix"
        ]
        self.assertTrue(ambiguous)
        self.assertTrue(all(not row["candidate_path"] for row in ambiguous))


if __name__ == "__main__":
    unittest.main()
