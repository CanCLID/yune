import hashlib
import json
import os
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("verify_archive_dist_identity.py")


class ArchiveDistIdentityTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.dist = self.root / "dist"
        (self.dist / "assets").mkdir(parents=True)
        (self.dist / "index.html").write_text("<main>Yune</main>\n", encoding="utf-8")
        (self.dist / "assets" / "app.js").write_text("export {};\n", encoding="utf-8")
        self.archive = self.root / "yune-web-dist.tar.gz"
        with tarfile.open(self.archive, "w:gz") as bundle:
            bundle.add(self.dist, arcname=".")
        self.archive_sha256 = hashlib.sha256(self.archive.read_bytes()).hexdigest()

    def tearDown(self):
        self.temporary.cleanup()

    def run_verifier(self, dist: Path, *, receipt: Path | None = None):
        command = [
            "python3",
            str(SCRIPT),
            "--archive",
            str(self.archive),
            "--dist",
            str(dist),
            "--expected-archive-sha256",
            self.archive_sha256,
        ]
        if receipt is not None:
            command.extend(("--receipt", str(receipt)))
        return subprocess.run(
            command,
            text=True,
            capture_output=True,
            env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
            check=False,
        )

    def test_exact_archive_tree_matches_and_writes_create_new_receipt(self):
        receipt = self.root / "evidence" / "archive-dist-identity.json"
        completed = self.run_verifier(self.dist, receipt=receipt)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        result = json.loads(receipt.read_text(encoding="utf-8"))
        self.assertEqual(result["status"], "matched")
        self.assertEqual(result["fileCount"], 2)
        repeated = self.run_verifier(self.dist, receipt=receipt)
        self.assertNotEqual(repeated.returncode, 0)
        self.assertEqual(json.loads(receipt.read_text(encoding="utf-8")), result)

    def test_separately_valid_directory_cannot_stand_in_for_archive_bytes(self):
        other = self.root / "other-valid-build"
        (other / "assets").mkdir(parents=True)
        (other / "index.html").write_text("<main>Other</main>\n", encoding="utf-8")
        (other / "assets" / "app.js").write_text("export {};\n", encoding="utf-8")
        completed = self.run_verifier(other)
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("archive/dist bytes differ: index.html", completed.stderr)

    def test_wrong_certified_archive_identity_is_rejected_before_comparison(self):
        self.archive_sha256 = "0" * 64
        completed = self.run_verifier(self.dist)
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("archive changed after its certified SHA-256", completed.stderr)

    def test_extra_file_and_symlink_are_rejected(self):
        (self.dist / "extra.txt").write_text("extra\n", encoding="utf-8")
        completed = self.run_verifier(self.dist)
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("file sets differ", completed.stderr)
        (self.dist / "extra.txt").unlink()
        (self.dist / "alias.js").symlink_to(self.dist / "assets" / "app.js")
        completed = self.run_verifier(self.dist)
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("contains a symbolic link", completed.stderr)


if __name__ == "__main__":
    unittest.main()
