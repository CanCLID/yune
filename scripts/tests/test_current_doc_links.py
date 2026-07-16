from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-current-doc-links.py"


class CurrentDocumentLinkTests(unittest.TestCase):
    @staticmethod
    def create_junction(link: Path, target: Path) -> None:
        if sys.platform != "win32":
            raise unittest.SkipTest("directory junctions require Windows")
        result = subprocess.run(
            ["cmd", "/c", "mklink", "/J", str(link), str(target)],
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode != 0:
            raise unittest.SkipTest(
                f"directory junctions unavailable: {result.stdout}{result.stderr}"
            )

    def run_audit(self, root: Path, paths_file: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--repo-root",
                str(root),
                "--paths-from",
                str(paths_file),
            ],
            text=True,
            capture_output=True,
            check=False,
        )

    def fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path, Path]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        (root / "docs").mkdir()
        (root / "target.txt").write_text("target\n", encoding="utf-8")
        document = root / "docs" / "current.md"
        document.write_text(
            "[local](../target.txt) [anchor](#here) [external](https://example.com)\n",
            encoding="utf-8",
        )
        paths = root / "paths.txt"
        paths.write_text("docs/current.md\n", encoding="utf-8")
        return temporary, root, paths

    def test_accepts_existing_repository_local_targets(self) -> None:
        temporary, root, paths = self.fixture()
        with temporary:
            result = self.run_audit(root, paths)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("1 local targets", result.stdout)

    def test_rejects_missing_local_target(self) -> None:
        temporary, root, paths = self.fixture()
        with temporary:
            (root / "docs" / "current.md").write_text("[missing](nope.md)\n", encoding="utf-8")
            result = self.run_audit(root, paths)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("missing local target", result.stderr)

    def test_rejects_escaping_local_target(self) -> None:
        temporary, root, paths = self.fixture()
        with temporary:
            outside = root.parent / f"{root.name}-outside.txt"
            outside.write_text("outside\n", encoding="utf-8")
            self.addCleanup(outside.unlink, missing_ok=True)
            (root / "docs" / "current.md").write_text(
                f"[escape](../../{outside.name})\n", encoding="utf-8"
            )
            result = self.run_audit(root, paths)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("escaping local target", result.stderr)

    def test_rejects_unsafe_local_target_and_touched_path(self) -> None:
        temporary, root, paths = self.fixture()
        with temporary:
            (root / "docs" / "current.md").write_text(
                "![unsafe](C:/outside.png)\n", encoding="utf-8"
            )
            unsafe_link = self.run_audit(root, paths)
            self.assertNotEqual(unsafe_link.returncode, 0)
            self.assertIn("unsafe local link target", unsafe_link.stderr)

            (root / "README.md").write_text("readme\n", encoding="utf-8")
            paths.write_text("docs/../README.md\n", encoding="utf-8")
            unsafe_path = self.run_audit(root, paths)
            self.assertNotEqual(unsafe_path.returncode, 0)
            self.assertIn("normalized repository-relative path", unsafe_path.stderr)

    def test_rejects_symbolic_link_target(self) -> None:
        temporary, root, paths = self.fixture()
        with temporary:
            link = root / "linked.txt"
            try:
                link.symlink_to(root / "target.txt")
            except OSError as error:
                self.skipTest(f"symbolic links unavailable: {error}")
            (root / "docs" / "current.md").write_text("[link](../linked.txt)\n", encoding="utf-8")
            result = self.run_audit(root, paths)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("symbolic link", result.stderr)

    def test_rejects_directory_junction_target(self) -> None:
        temporary, root, paths = self.fixture()
        with temporary:
            target = root / "junction-target"
            target.mkdir()
            (target / "target.txt").write_text("target\n", encoding="utf-8")
            junction = root / "junction"
            self.create_junction(junction, target)
            try:
                (root / "docs" / "current.md").write_text(
                    "[junction](../junction/target.txt)\n", encoding="utf-8"
                )
                result = self.run_audit(root, paths)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("junction", result.stderr)
            finally:
                os.rmdir(junction)

    def test_rejects_file_uri_instead_of_treating_it_as_external(self) -> None:
        temporary, root, paths = self.fixture()
        with temporary:
            (root / "docs" / "current.md").write_text(
                "[outside](file:///outside.md)\n", encoding="utf-8"
            )
            result = self.run_audit(root, paths)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("file URI is not an allowed external target", result.stderr)

    def test_requires_paths_file_and_nonempty_membership(self) -> None:
        temporary, root, paths = self.fixture()
        with temporary:
            missing = self.run_audit(root, root / "missing-paths.txt")
            self.assertNotEqual(missing.returncode, 0)
            paths.write_text("", encoding="utf-8")
            empty = self.run_audit(root, paths)
            self.assertNotEqual(empty.returncode, 0)
            self.assertIn("at least one", empty.stderr)


if __name__ == "__main__":
    unittest.main()
