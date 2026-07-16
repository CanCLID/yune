from __future__ import annotations

import csv
import hashlib
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "verify-packet-manifest.py"


class PacketManifestTests(unittest.TestCase):
    def run_verify(
        self, root: Path, manifest: Path, *, treeish: str | None = None
    ) -> subprocess.CompletedProcess[str]:
        command = [sys.executable, str(SCRIPT), "--repo-root", str(root)]
        if treeish is not None:
            command.extend(["--treeish", treeish])
        command.append(str(manifest))
        return subprocess.run(
            command,
            text=True,
            capture_output=True,
            check=False,
        )

    def fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path, Path]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        packet = root / "packet"
        packet.mkdir()
        (packet / "a.txt").write_bytes(b"alpha\n")
        (packet / "nested").mkdir()
        (packet / "nested" / "b.txt").write_bytes(b"beta\n")
        manifest = packet / "packet-manifest.csv"
        self.write_manifest(manifest, packet, ["a.txt", "nested/b.txt"])
        return temporary, root, manifest

    @staticmethod
    def write_manifest(manifest: Path, packet: Path, paths: list[str]) -> None:
        with manifest.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.writer(handle, lineterminator="\n")
            writer.writerow(["path", "bytes", "sha256"])
            for path in paths:
                data = (packet / path).read_bytes()
                writer.writerow([path, len(data), hashlib.sha256(data).hexdigest()])

    def test_accepts_exact_bidirectional_membership_and_hashes(self) -> None:
        temporary, root, manifest = self.fixture()
        with temporary:
            result = self.run_verify(root, manifest)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("2 files", result.stdout)

    def test_accepts_exact_membership_from_committed_tree(self) -> None:
        temporary, root, manifest = self.fixture()
        with temporary:
            for command in (
                ["git", "init", "-q"],
                ["git", "config", "user.name", "M60 Test"],
                ["git", "config", "user.email", "m60@example.invalid"],
                ["git", "add", "packet"],
                ["git", "commit", "-q", "-m", "packet"],
            ):
                subprocess.run(command, cwd=root, check=True, capture_output=True)
            result = self.run_verify(root, manifest, treeish="HEAD")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("from HEAD: 2 files", result.stdout)

    def test_rejects_unlisted_and_missing_files(self) -> None:
        temporary, root, manifest = self.fixture()
        with temporary:
            packet = manifest.parent
            (packet / "extra.txt").write_text("extra\n", encoding="utf-8")
            extra = self.run_verify(root, manifest)
            self.assertNotEqual(extra.returncode, 0)
            self.assertIn("membership mismatch", extra.stderr)
            (packet / "extra.txt").unlink()
            (packet / "nested" / "b.txt").unlink()
            missing = self.run_verify(root, manifest)
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn("membership mismatch", missing.stderr)

    def test_rejects_duplicate_and_traversing_rows(self) -> None:
        temporary, root, manifest = self.fixture()
        with temporary:
            row = "a.txt,6," + hashlib.sha256(b"alpha\n").hexdigest() + "\n"
            manifest.write_text("path,bytes,sha256\n" + row + row, encoding="utf-8")
            duplicate = self.run_verify(root, manifest)
            self.assertNotEqual(duplicate.returncode, 0)
            self.assertIn("duplicate", duplicate.stderr)
            manifest.write_text(
                "path,bytes,sha256\n../outside,1," + "0" * 64 + "\n",
                encoding="utf-8",
            )
            traversal = self.run_verify(root, manifest)
            self.assertNotEqual(traversal.returncode, 0)
            self.assertIn("unsafe packet path", traversal.stderr)

            manifest.write_text(
                "path,bytes,sha256\nC:/outside,1," + "0" * 64 + "\n",
                encoding="utf-8",
            )
            absolute = self.run_verify(root, manifest)
            self.assertNotEqual(absolute.returncode, 0)
            self.assertIn("unsafe packet path", absolute.stderr)

            for unnormalized in ("nested//b.txt", "nested/./b.txt"):
                with self.subTest(path=unnormalized):
                    manifest.write_text(
                        f"path,bytes,sha256\n{unnormalized},1," + "0" * 64 + "\n",
                        encoding="utf-8",
                    )
                    result = self.run_verify(root, manifest)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("unsafe packet path", result.stderr)

    def test_rejects_byte_and_sha256_mismatches(self) -> None:
        temporary, root, manifest = self.fixture()
        with temporary:
            source = manifest.read_text(encoding="utf-8")
            manifest.write_text(source.replace("a.txt,6,", "a.txt,5,"), encoding="utf-8")
            size = self.run_verify(root, manifest)
            self.assertNotEqual(size.returncode, 0)
            self.assertIn("byte mismatch", size.stderr)
            self.write_manifest(manifest, manifest.parent, ["a.txt", "nested/b.txt"])
            source = manifest.read_text(encoding="utf-8")
            digest = hashlib.sha256(b"alpha\n").hexdigest()
            manifest.write_text(source.replace(digest, "0" * 64), encoding="utf-8")
            sha = self.run_verify(root, manifest)
            self.assertNotEqual(sha.returncode, 0)
            self.assertIn("SHA-256 mismatch", sha.stderr)

    def test_rejects_symbolic_links(self) -> None:
        temporary, root, manifest = self.fixture()
        with temporary:
            packet = manifest.parent
            link = packet / "linked.txt"
            try:
                link.symlink_to(packet / "a.txt")
            except OSError as error:
                self.skipTest(f"symbolic links unavailable: {error}")
            result = self.run_verify(root, manifest)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("symbolic link", result.stderr)

    def test_rejects_gitlinks_in_committed_packet_tree(self) -> None:
        temporary, root, manifest = self.fixture()
        with temporary:
            for command in (
                ["git", "init", "-q"],
                ["git", "config", "user.name", "M60 Test"],
                ["git", "config", "user.email", "m60@example.invalid"],
                ["git", "add", "packet"],
                ["git", "commit", "-q", "-m", "packet"],
            ):
                subprocess.run(command, cwd=root, check=True, capture_output=True)
            head = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=root,
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
            subprocess.run(
                ["git", "update-index", "--add", "--cacheinfo", "160000", head, "packet/vendor"],
                cwd=root,
                check=True,
                capture_output=True,
            )
            subprocess.run(
                ["git", "commit", "-q", "-m", "packet gitlink"],
                cwd=root,
                check=True,
                capture_output=True,
            )
            result = self.run_verify(root, manifest, treeish="HEAD")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("non-blob tree entry", result.stderr)


if __name__ == "__main__":
    unittest.main()
