from __future__ import annotations

import contextlib
import importlib.util
import io
import sys
import tempfile
import unittest
from pathlib import Path
from urllib.parse import quote


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent


def load_script(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


privacy = load_script("check-public-evidence-privacy.py")


class PublicEvidencePrivacyTests(unittest.TestCase):
    def setUp(self):
        target = REPO_ROOT / "target"
        target.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(
            prefix="m61-public-evidence-privacy-test-", dir=target
        )
        self.root = Path(self.temp.name)
        self.public = self.root / "public.txt"
        self.paths = self.root / "paths.txt"
        self.forbidden = self.root / "forbidden.txt"
        self.public.write_text("safe\n", encoding="utf-8")
        self.write_paths(self.public)
        self.forbidden.write_text("PrivateUser\nPrivateHost\n", encoding="utf-8")

    def tearDown(self):
        self.temp.cleanup()

    def relative(self, path: Path) -> str:
        return path.resolve().relative_to(REPO_ROOT).as_posix()

    def write_paths(self, *paths: Path):
        self.paths.write_text(
            "".join(f"{self.relative(path)}\n" for path in paths),
            encoding="utf-8",
        )

    def run_tool(self, *, paths=None, forbidden=None):
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(
            stderr
        ):
            result = privacy.main(
                [
                    "--paths-from",
                    str(self.paths if paths is None else paths),
                    "--forbid-literal-file",
                    str(self.forbidden if forbidden is None else forbidden),
                ]
            )
        return result, stdout.getvalue(), stderr.getvalue()

    def test_allowed_hardware_os_toolchain_and_hash_fields_pass(self):
        self.public.write_text(
            "model=MacBook Air\n"
            "chip=Apple M4\n"
            "ram_bytes=25769803776\n"
            "os=macOS 26.5.2 build 25F84\n"
            "rustc=rustc 1.90.0 (abcdef 2026-01-01)\n"
            "visual_studio=17.14.8\n"
            f"source_sha256={'a' * 64}\n"
            f"source_commit={'b' * 40}\n",
            encoding="utf-8",
        )
        result, stdout, stderr = self.run_tool()
        self.assertEqual((result, stderr), (0, ""))
        self.assertIn("public-evidence-privacy: pass", stdout)
        self.assertIn("files=1", stdout)
        self.assertIn("forbidden_literals=2", stdout)

    def test_bare_user_and_host_literals_are_rejected_case_insensitively(self):
        for value in ("prefix privateuser suffix", "PRIVATEHOST"):
            with self.subTest(value=value):
                self.public.write_text(value + "\n", encoding="utf-8")
                result, stdout, stderr = self.run_tool()
                self.assertEqual((result, stdout), (2, ""))
                self.assertIn("category=forbidden_literal", stderr)

    def test_normalized_slash_and_percent_encoded_literals_are_rejected(self):
        self.forbidden.write_text(
            "C:\\Users\\PrivateUser\nPrivate%20Machine\n",
            encoding="utf-8",
        )
        cases = (
            "profile=c:/users/privateuser/Desktop",
            "host=private machine",
        )
        for value in cases:
            with self.subTest(value=value):
                self.public.write_text(value + "\n", encoding="utf-8")
                result, _, stderr = self.run_tool()
                self.assertEqual(result, 2)
                self.assertIn("category=forbidden_literal", stderr)

    def test_generic_profile_email_and_secret_patterns_are_rejected(self):
        cases = (
            ("profile", "root=/Users/Someone/Desktop", "user_profile_path"),
            ("windows-profile", r"root=C:\Users\Someone", "user_profile_path"),
            ("email", "operator=person@example.net", "email"),
            ("secret", "api_key=abcd1234", "secret_assignment"),
            ("bearer", "header Bearer abcdefghijklmnop", "bearer_token"),
            (
                "known-token",
                "token=ghp_abcdefghijklmnopqrstuvwxyz",
                "known_token_prefix",
            ),
            (
                "project-token",
                "token=(sk-proj-abcdefghijklmnopqrstuvwxyz012345)",
                "known_token_prefix",
            ),
            (
                "private-key",
                "-----BEGIN PRIVATE KEY-----",
                "private_key",
            ),
        )
        for label, text, category in cases:
            with self.subTest(label=label):
                self.public.write_text(text + "\n", encoding="utf-8")
                result, stdout, stderr = self.run_tool()
                self.assertEqual((result, stdout), (2, ""))
                self.assertIn(f"category={category}", stderr)

    def test_punctuated_paths_bearer_and_credentialed_urls_are_rejected(self):
        cases = (
            (
                "markdown-user-path",
                "paths=(/Users/Someone/Desktop/raw.json),",
                "user_profile_path",
            ),
            (
                "json-private-path",
                '{"raw":"/private/tmp/web06/raw.json"},',
                "posix_absolute_path",
            ),
            (
                "bracketed-opt-path",
                "tool=[/opt/emsdk/emcc];",
                "posix_absolute_path",
            ),
            (
                "json-bearer",
                '{"Authorization":"Bearer abcdefghijklmnop"},',
                "bearer_token",
            ),
            (
                "credentialed-url",
                "preview=https://user:password@example.invalid/path",
                "credentialed_url",
            ),
            (
                "encoded-credentialed-url",
                "preview=https%3A%2F%2Fuser%3Apassword%40example.invalid/path",
                "credentialed_url",
            ),
            (
                "double-encoded-credentialed-url",
                "preview=https%253A%252F%252Fuser%253Apassword%2540example.invalid%252Fpath",
                "credentialed_url",
            ),
        )
        for label, text, category in cases:
            with self.subTest(label=label):
                self.public.write_text(text + "\n", encoding="utf-8")
                result, stdout, stderr = self.run_tool()
                self.assertEqual((result, stdout), (2, ""))
                self.assertIn(f"category={category}", stderr)

    def test_generic_absolute_and_file_url_paths_are_rejected(self):
        cases = (
            ("posix", "artifact=/private/tmp/web06/raw.json", "posix_absolute_path"),
            ("mac-user", "/Users/Someone/Desktop/raw.json", "user_profile_path"),
            ("linux-user", "/home/someone/raw.json", "user_profile_path"),
            ("drive", r"artifact=D:\bench\raw.json", "windows_absolute_path"),
            ("drive-slash", "artifact=d:/bench/raw.json", "windows_absolute_path"),
            ("unc", r"artifact=\\server\share\raw.json", "unc_path"),
            ("file-url", "artifact=file:///private/tmp/raw.json", "file_url"),
            (
                "encoded-file-url",
                "artifact=file:%2F%2F%2Fprivate%2Ftmp%2Fraw.json",
                "file_url",
            ),
        )
        for label, text, category in cases:
            with self.subTest(label=label):
                self.public.write_text(text + "\n", encoding="utf-8")
                result, stdout, stderr = self.run_tool()
                self.assertEqual((result, stdout), (2, ""))
                self.assertIn(f"category={category}", stderr)

    def test_public_url_paths_and_relative_repository_paths_remain_allowed(self):
        self.public.write_text(
            "request_path=/assets/app.js\n"
            "evidence=docs/reports/evidence/web06/README.md\n"
            "preview=https://candidate.pages.dev/index.html\n",
            encoding="utf-8",
        )
        result, stdout, stderr = self.run_tool()
        self.assertEqual((result, stderr), (0, ""))
        self.assertIn("tool_version=4", stdout)

    def test_malformed_and_overencoded_url_escapes_fail_closed(self):
        cases = [
            "preview=https%3A%2F%2Fuser%3Apassword%4@example.invalid",
            "preview=https%3A%2F%2Fuser%3Apassword%ZZexample.invalid",
            "preview=https%253A%252F%252Fuser%25253Apassword%2540example.invalid",
            "preview=https%FFexample.invalid",
        ]
        for secret in (
            "Bearer abcdefghijklmnop",
            "sk-proj-abcdefghijklmnopqrstuvwxyz012345",
        ):
            encoded = (
                secret.replace("-", "%2D")
                if secret.startswith("sk-proj-")
                else quote(secret, safe="")
            )
            for _ in range(3):
                encoded = quote(encoded, safe="")
            cases.append(f"payload={encoded}")
        for text in cases:
            with self.subTest(text=text):
                self.public.write_text(text + "\n", encoding="utf-8")
                result, stdout, stderr = self.run_tool()
                self.assertEqual((result, stdout), (2, ""))
                self.assertIn("category=malformed_url_encoding", stderr)

    def test_missing_empty_malformed_and_duplicate_inputs_fail_closed(self):
        missing = self.root / "missing.txt"
        cases = []

        self.paths.write_text("", encoding="utf-8")
        cases.append(("empty-paths", self.paths, self.forbidden))

        empty_forbidden = self.root / "empty-forbidden.txt"
        empty_forbidden.write_text("", encoding="utf-8")
        valid_paths = self.root / "valid-paths.txt"
        valid_paths.write_text(self.relative(self.public) + "\n", encoding="utf-8")
        cases.append(("empty-forbidden", valid_paths, empty_forbidden))
        cases.append(("missing-paths", missing, self.forbidden))
        cases.append(("missing-forbidden", valid_paths, missing))

        duplicate_paths = self.root / "duplicate-paths.txt"
        duplicate_paths.write_text(
            self.relative(self.public) + "\n" + self.relative(self.public) + "\n",
            encoding="utf-8",
        )
        cases.append(("duplicate-paths", duplicate_paths, self.forbidden))

        blank_forbidden = self.root / "blank-forbidden.txt"
        blank_forbidden.write_text("one\n\nthree\n", encoding="utf-8")
        cases.append(("blank-forbidden", valid_paths, blank_forbidden))

        for label, paths, forbidden in cases:
            with self.subTest(label=label):
                result, stdout, stderr = self.run_tool(
                    paths=paths, forbidden=forbidden
                )
                self.assertEqual((result, stdout), (2, ""))
                self.assertIn("public-evidence-privacy: fail", stderr)

    def test_absolute_escaping_backslash_and_missing_allowlist_paths_fail(self):
        bad_entries = (
            str(self.public.resolve()),
            "../outside.txt",
            r"target\somewhere.txt",
            "target/does-not-exist.txt",
        )
        for entry in bad_entries:
            with self.subTest(entry=entry):
                self.paths.write_text(entry + "\n", encoding="utf-8")
                result, _, stderr = self.run_tool()
                self.assertEqual(result, 2)
                self.assertIn("category=invalid_allowlist_path", stderr)

    def test_failure_verdict_never_echoes_forbidden_literal(self):
        secret_literal = "DoNotEchoThisHost"
        self.forbidden.write_text(secret_literal + "\n", encoding="utf-8")
        self.public.write_text(
            f"machine={secret_literal.lower()}\n", encoding="utf-8"
        )
        result, stdout, stderr = self.run_tool()
        self.assertEqual((result, stdout), (2, ""))
        rendered = stderr.casefold()
        self.assertNotIn(secret_literal.casefold(), rendered)
        self.assertNotIn(str(self.public).casefold(), rendered)
        self.assertIn("category=forbidden_literal", stderr)

    def test_multiple_allowlisted_files_are_all_scanned(self):
        second = self.root / "second.txt"
        second.write_text("PrivateHost\n", encoding="utf-8")
        self.write_paths(self.public, second)
        result, stdout, stderr = self.run_tool()
        self.assertEqual((result, stdout), (2, ""))
        self.assertIn("file_index=2", stderr)


if __name__ == "__main__":
    unittest.main()
