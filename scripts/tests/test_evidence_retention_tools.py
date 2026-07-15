from __future__ import annotations

import csv
import hashlib
import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path, PurePosixPath


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


retention = load_module("evidence_retention", SCRIPTS / "evidence_retention.py")


def git(root: Path, *arguments: str) -> None:
    completed = subprocess.run(
        ["git", "-C", str(root), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise AssertionError(completed.stdout + completed.stderr)


class ExternalEvidencePathTests(unittest.TestCase):
    def test_default_is_user_level_and_timestamped(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary)
            path = retention.default_external_output_path(
                "native-rime-inprocess",
                home=home,
                timestamp="20260715T010203Z",
                environment={},
            )
            self.assertEqual(
                path,
                (home / ".yune/evidence/native-rime-inprocess/20260715T010203Z").resolve(),
            )

    def test_rejects_main_and_linked_worktrees_but_accepts_external_path(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repository = root / "repository"
            linked = root / "linked"
            external = root / "external with spaces" / "run"
            repository.mkdir()
            git(repository, "init", "--quiet")
            git(repository, "config", "user.name", "Yune Test")
            git(repository, "config", "user.email", "yune@example.invalid")
            (repository / "tracked.txt").write_text("tracked\n", encoding="utf-8")
            git(repository, "add", "tracked.txt")
            git(repository, "commit", "--quiet", "-m", "fixture")
            git(repository, "worktree", "add", "--quiet", "--detach", str(linked), "HEAD")

            for unsafe in (
                repository / "target/raw-evidence",
                linked / "raw-evidence",
                root,
            ):
                with self.assertRaisesRegex(
                    retention.EvidencePolicyError,
                    "disjoint from Git worktree",
                ):
                    retention.validate_external_output_path(repository, unsafe)
            self.assertEqual(
                retention.validate_external_output_path(repository, external),
                external.resolve(),
            )

    def test_symlink_alias_into_worktree_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repository = root / "repository"
            repository.mkdir()
            git(repository, "init", "--quiet")
            alias = root / "alias"
            try:
                alias.symlink_to(repository, target_is_directory=True)
            except OSError as error:
                self.skipTest(f"symlinks unavailable: {error}")
            with self.assertRaises(retention.EvidencePolicyError):
                retention.validate_external_output_path(repository, alias / "output")


class CompactEvidenceCuratorTests(unittest.TestCase):
    def test_copies_only_sorted_allowlist_and_writes_deterministic_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            (source / "nested").mkdir()
            (source / "z.txt").write_bytes(b"z\n")
            (source / "nested/a.json").write_bytes(b'{"a":1}\n')
            (source / "unlisted.log").write_bytes(b"do not copy\n")
            allowlist = root / "allowlist.txt"
            allowlist.write_text("z.txt\nnested/a.json\n", encoding="utf-8")

            first = root / "first"
            second = root / "second"
            first_rows = retention.curate_compact_evidence(source, first, allowlist)
            second_rows = retention.curate_compact_evidence(source, second, allowlist)
            self.assertEqual(first_rows, second_rows)
            self.assertEqual(
                sorted(path.relative_to(first).as_posix() for path in first.rglob("*") if path.is_file()),
                ["nested/a.json", "packet-manifest.csv", "z.txt"],
            )
            self.assertEqual(
                (first / "packet-manifest.csv").read_bytes(),
                (second / "packet-manifest.csv").read_bytes(),
            )
            with (first / "packet-manifest.csv").open(newline="", encoding="utf-8") as handle:
                rows = list(csv.DictReader(handle))
            self.assertEqual([row["path"] for row in rows], ["nested/a.json", "z.txt"])
            for row in rows:
                payload = (first / row["path"]).read_bytes()
                self.assertEqual(int(row["size_bytes"]), len(payload))
                self.assertEqual(row["sha256"], hashlib.sha256(payload).hexdigest())

    def test_rejects_traversal_duplicates_symlinks_and_existing_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            (source / "file.txt").write_text("fixture\n", encoding="utf-8")
            outside = root / "outside.txt"
            outside.write_text("outside\n", encoding="utf-8")

            cases = ("../outside.txt\n", "file.txt\nfile.txt\n")
            for index, content in enumerate(cases):
                allowlist = root / f"bad-{index}.txt"
                allowlist.write_text(content, encoding="utf-8")
                with self.assertRaises(retention.EvidencePolicyError):
                    retention.curate_compact_evidence(
                        source,
                        root / f"output-{index}",
                        allowlist,
                    )

            link = source / "link.txt"
            try:
                link.symlink_to(outside)
            except OSError as error:
                self.skipTest(f"symlinks unavailable: {error}")
            allowlist = root / "symlink.txt"
            allowlist.write_text("link.txt\n", encoding="utf-8")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "symlink"):
                retention.curate_compact_evidence(source, root / "link-output", allowlist)

            output = root / "existing"
            output.mkdir()
            allowlist.write_text("file.txt\n", encoding="utf-8")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "new path"):
                retention.curate_compact_evidence(source, output, allowlist)

    def test_rejects_casefold_collisions_and_manifest_alias(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            (source / "Case.txt").write_text("one\n", encoding="utf-8")
            alias = source / "case.txt"
            if not alias.exists():
                alias.write_text("two\n", encoding="utf-8")
            allowlist = root / "allowlist.txt"
            allowlist.write_text("Case.txt\ncase.txt\n", encoding="utf-8")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "case-insensitive"):
                retention.curate_compact_evidence(
                    source, root / "collision-output", allowlist
                )

            manifest_alias = source / "Packet-Manifest.csv"
            manifest_alias.write_text("payload\n", encoding="utf-8")
            allowlist.write_text("Packet-Manifest.csv\n", encoding="utf-8")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "generated manifest"):
                retention.curate_compact_evidence(
                    source, root / "manifest-output", allowlist
                )

    def test_rejects_symlink_source_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            (source / "receipt.txt").write_text("receipt\n", encoding="utf-8")
            alias = root / "source-alias"
            try:
                alias.symlink_to(source, target_is_directory=True)
            except OSError as error:
                self.skipTest(f"symlinks unavailable: {error}")
            allowlist = root / "allowlist.txt"
            allowlist.write_text("receipt.txt\n", encoding="utf-8")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "source root"):
                retention.curate_compact_evidence(
                    alias, root / "output", allowlist
                )

    def test_rejects_symlink_ancestor_of_source_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            real = root / "real"
            source = real / "source"
            source.mkdir(parents=True)
            (source / "receipt.txt").write_text("receipt\n", encoding="utf-8")
            alias = root / "alias"
            try:
                alias.symlink_to(real, target_is_directory=True)
            except OSError as error:
                self.skipTest(f"symlinks unavailable: {error}")
            allowlist = root / "allowlist.txt"
            allowlist.write_text("receipt.txt\n", encoding="utf-8")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "traverses a symlink"):
                retention.curate_compact_evidence(
                    alias / "source", root / "output", allowlist
                )

    def test_rejects_packet_over_ten_mib(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            payload = source / "large.fixture"
            with payload.open("wb") as handle:
                handle.truncate(retention.MAX_CURATED_PACKET_BYTES + 1)
            allowlist = root / "allowlist.txt"
            allowlist.write_text("large.fixture\n", encoding="utf-8")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "10 MiB"):
                retention.curate_compact_evidence(
                    source,
                    root / "output",
                    allowlist,
                    fixture_exceptions=(PurePosixPath("large.fixture"),),
                )

    def test_rejects_raw_classes_and_large_files_without_exception(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            allowlist = root / "allowlist.txt"
            for index, name in enumerate(
                (
                    "m37_metrics.csv",
                    "samples.csv",
                    "startup_session_trace.csv",
                    "model.marisa",
                )
            ):
                (source / name).write_text("raw\n", encoding="utf-8")
                allowlist.write_text(f"{name}\n", encoding="utf-8")
                with self.assertRaisesRegex(retention.EvidencePolicyError, "raw evidence"):
                    retention.curate_compact_evidence(
                        source,
                        root / f"raw-output-{index}",
                        allowlist,
                        fixture_exceptions=(PurePosixPath(name),),
                    )

            large = source / "oracle.fixture"
            with large.open("wb") as handle:
                handle.truncate(retention.MAX_NEW_FILE_BYTES + 1)
            allowlist.write_text("oracle.fixture\n", encoding="utf-8")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "5 MiB"):
                retention.curate_compact_evidence(source, root / "large-output", allowlist)
            rows = retention.curate_compact_evidence(
                source,
                root / "excepted-output",
                allowlist,
                fixture_exceptions=(PurePosixPath("oracle.fixture"),),
            )
            self.assertEqual(rows[0][1], retention.MAX_NEW_FILE_BYTES + 1)


class EvidenceGrowthGuardTests(unittest.TestCase):
    def record(self, value: str, size: int):
        return retention.AddedEvidenceFile(PurePosixPath(value), size)

    def test_rejects_every_raw_class_even_with_exception(self) -> None:
        raw = (
            "docs/reports/evidence/new/m37_metrics.csv",
            "docs/reports/evidence/new/samples.csv",
            "docs/reports/evidence/new/startup_session_trace.csv",
            "docs/reports/evidence/new/model.marisa",
        )
        for value in raw:
            path = PurePosixPath(value)
            with self.assertRaisesRegex(retention.EvidencePolicyError, "raw evidence"):
                retention.validate_evidence_growth(
                    [retention.AddedEvidenceFile(path, 1)],
                    [path],
                )

    def test_large_file_requires_exact_fixture_exception(self) -> None:
        path = PurePosixPath("docs/reports/evidence/new/oracle.fixture.json")
        record = retention.AddedEvidenceFile(path, retention.MAX_NEW_FILE_BYTES + 1)
        with self.assertRaisesRegex(retention.EvidencePolicyError, "fixture exception"):
            retention.validate_evidence_growth([record])
        self.assertEqual(
            retention.validate_evidence_growth([record], [path]),
            (record,),
        )

    def test_packet_cap_is_aggregate_and_exceptions_do_not_bypass_it(self) -> None:
        records = [
            self.record(
                f"docs/reports/evidence/new/part-{index}.fixture",
                4 * 1024 * 1024,
            )
            for index in range(3)
        ]
        with self.assertRaisesRegex(retention.EvidencePolicyError, "packet exceeds 10 MiB"):
            retention.validate_evidence_growth(
                records,
                [record.path for record in records],
            )

    def test_full_index_packet_size_blocks_incremental_cap_bypass(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            git(repository, "init", "--quiet")
            git(repository, "config", "user.name", "Yune Test")
            git(repository, "config", "user.email", "yune@example.invalid")
            packet = repository / "docs/reports/evidence/incremental"
            packet.mkdir(parents=True)
            first = packet / "first.fixture"
            with first.open("wb") as handle:
                handle.truncate(6 * 1024 * 1024)
            git(repository, "add", ".")
            git(repository, "commit", "--quiet", "-m", "existing packet")

            second = packet / "second.fixture"
            with second.open("wb") as handle:
                handle.truncate(5 * 1024 * 1024)
            git(repository, "add", "docs/reports/evidence/incremental/second.fixture")
            records = retention.staged_added_evidence(repository)
            sizes = retention.staged_packet_sizes(repository, records)
            self.assertEqual(sizes, {"incremental": 11 * 1024 * 1024})
            with self.assertRaisesRegex(retention.EvidencePolicyError, "post-change"):
                retention.validate_evidence_growth(
                    records,
                    post_change_packet_sizes=sizes,
                )

    def test_staged_modification_and_raw_rename_are_guarded(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            git(repository, "init", "--quiet")
            git(repository, "config", "user.name", "Yune Test")
            git(repository, "config", "user.email", "yune@example.invalid")
            packet = repository / "docs/reports/evidence/changed"
            packet.mkdir(parents=True)
            receipt = packet / "receipt.csv"
            receipt.write_text("small\n", encoding="utf-8")
            git(repository, "add", ".")
            git(repository, "commit", "--quiet", "-m", "existing packet")

            with receipt.open("wb") as handle:
                handle.truncate(retention.MAX_CURATED_PACKET_BYTES + 1)
            git(repository, "add", receipt.relative_to(repository).as_posix())
            records = retention.staged_changed_evidence(repository)
            self.assertEqual([record.path.name for record in records], ["receipt.csv"])
            with self.assertRaisesRegex(retention.EvidencePolicyError, "5 MiB"):
                retention.validate_evidence_growth(
                    records,
                    post_change_packet_sizes=retention.staged_packet_sizes(
                        repository, records
                    ),
                )

            git(repository, "reset", "--hard", "HEAD")
            git(
                repository,
                "mv",
                "docs/reports/evidence/changed/receipt.csv",
                "docs/reports/evidence/changed/samples.csv",
            )
            records = retention.staged_changed_evidence(repository)
            self.assertEqual([record.path.name for record in records], ["samples.csv"])
            with self.assertRaisesRegex(retention.EvidencePolicyError, "raw evidence"):
                retention.validate_evidence_growth(records)

    def test_tree_diff_mode_checks_committed_changes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            git(repository, "init", "--quiet")
            git(repository, "config", "user.name", "Yune Test")
            git(repository, "config", "user.email", "yune@example.invalid")
            packet = repository / "docs/reports/evidence/committed"
            packet.mkdir(parents=True)
            receipt = packet / "receipt.csv"
            receipt.write_text("small\n", encoding="utf-8")
            git(repository, "add", ".")
            git(repository, "commit", "--quiet", "-m", "base")
            base = subprocess.run(
                ["git", "-C", str(repository), "rev-parse", "HEAD"],
                check=True,
                text=True,
                capture_output=True,
            ).stdout.strip()
            git(
                repository,
                "mv",
                "docs/reports/evidence/committed/receipt.csv",
                "docs/reports/evidence/committed/samples.csv",
            )
            git(repository, "commit", "--quiet", "-m", "raw rename")
            records = retention.tree_changed_evidence(repository, base)
            self.assertEqual([record.path.name for record in records], ["samples.csv"])
            self.assertEqual(
                retention.tree_packet_sizes(repository, records),
                {"committed": len(b"small\n")},
            )
            with self.assertRaisesRegex(retention.EvidencePolicyError, "raw evidence"):
                retention.validate_evidence_growth(records)

    def test_staged_and_committed_symlinks_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            git(repository, "init", "--quiet")
            git(repository, "config", "user.name", "Yune Test")
            git(repository, "config", "user.email", "yune@example.invalid")
            (repository / "README.md").write_text("base\n", encoding="utf-8")
            git(repository, "add", ".")
            git(repository, "commit", "--quiet", "-m", "base")
            base = subprocess.run(
                ["git", "-C", str(repository), "rev-parse", "HEAD"],
                check=True,
                text=True,
                capture_output=True,
            ).stdout.strip()
            packet = repository / "docs/reports/evidence"
            packet.mkdir(parents=True)
            link = packet / "linked"
            try:
                link.symlink_to("/tmp", target_is_directory=True)
            except OSError as error:
                self.skipTest(f"symlinks unavailable: {error}")
            git(repository, "add", link.relative_to(repository).as_posix())
            staged = retention.staged_changed_evidence(repository)
            self.assertEqual(staged[0].git_mode, "120000")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "regular Git file"):
                retention.validate_evidence_growth(staged)

            git(repository, "commit", "--quiet", "-m", "symlink")
            committed = retention.tree_changed_evidence(repository, base)
            self.assertEqual(committed[0].git_mode, "120000")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "regular Git file"):
                retention.validate_evidence_growth(committed)

            link.unlink()
            link.write_text("regular\n", encoding="utf-8")
            git(repository, "add", link.relative_to(repository).as_posix())
            git(repository, "commit", "--quiet", "-m", "regular file")
            regular_base = subprocess.run(
                ["git", "-C", str(repository), "rev-parse", "HEAD"],
                check=True,
                text=True,
                capture_output=True,
            ).stdout.strip()
            link.unlink()
            link.symlink_to("/tmp", target_is_directory=True)
            git(repository, "add", link.relative_to(repository).as_posix())
            staged_type_change = retention.staged_changed_evidence(repository)
            self.assertEqual(staged_type_change[0].git_mode, "120000")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "regular Git file"):
                retention.validate_evidence_growth(staged_type_change)

            git(repository, "commit", "--quiet", "-m", "type change")
            committed_type_change = retention.tree_changed_evidence(
                repository, regular_base
            )
            self.assertEqual(committed_type_change[0].git_mode, "120000")
            with self.assertRaisesRegex(retention.EvidencePolicyError, "regular Git file"):
                retention.validate_evidence_growth(committed_type_change)

    def test_paths_from_rejects_symlink_packet_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary) / "repository"
            repository.mkdir()
            git(repository, "init", "--quiet")
            outside = Path(temporary) / "outside"
            outside.mkdir()
            (outside / "receipt.csv").write_text("receipt\n", encoding="utf-8")
            evidence = repository / "docs/reports/evidence"
            evidence.mkdir(parents=True)
            packet = evidence / "linked"
            try:
                packet.symlink_to(outside, target_is_directory=True)
            except OSError as error:
                self.skipTest(f"symlinks unavailable: {error}")
            relative = "docs/reports/evidence/linked/receipt.csv"
            with self.assertRaisesRegex(retention.EvidencePolicyError, "symlink"):
                retention.files_from_paths(repository, [relative])

    def test_accepts_small_compact_receipts(self) -> None:
        records = [
            self.record("docs/reports/evidence/new/README.md", 2000),
            self.record("docs/reports/evidence/new/gate-verdict.csv", 5000),
            self.record("docs/reports/evidence/new/environment.txt", 3000),
        ]
        self.assertEqual(retention.validate_evidence_growth(records), tuple(records))


class BenchmarkScriptPolicyWiringTests(unittest.TestCase):
    def test_all_native_benchmark_entrypoints_use_external_path_policy(self) -> None:
        modern = (SCRIPTS / "benchmark-native-rime-inprocess.ps1").read_text(
            encoding="utf-8-sig"
        )
        legacy = (SCRIPTS / "benchmark-yune-vs-librime.ps1").read_text(
            encoding="utf-8-sig"
        )
        macos = (SCRIPTS / "benchmark-native-rime-inprocess-macos.sh").read_text(
            encoding="utf-8"
        )
        for source in (modern, legacy):
            self.assertIn("evidence-output-path.ps1", source)
            self.assertIn('"validate",', source)
            self.assertNotIn("docs\\reports\\evidence", source)
        self.assertIn("evidence-output-path.py", macos)
        self.assertIn("--kind native-rime-inprocess-macos", macos)
        self.assertNotIn("docs/reports/evidence", macos)
        self.assertIn("must be a new external path", macos)
        self.assertIn(
            'Initialize-CreateNewOutputRoot $OutputRoot "OutputRoot"',
            modern,
        )
        self.assertNotIn(
            "Initialize-BenchmarkRoot $OutputRoot",
            modern,
        )


if __name__ == "__main__":
    unittest.main()
