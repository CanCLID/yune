#!/usr/bin/env python3
"""Tests for the deterministic M59 schema-validation staging helper."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "stage-m59-schema-validation-overlay.ps1"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


@unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
class SchemaValidationOverlayTests(unittest.TestCase):
    def run_stage(
        self,
        source: Path,
        output: Path,
        overlay: Path,
        manifest: Path,
        schema_id: str = "renamed_cangjie",
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                shutil.which("powershell") or "powershell",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                str(SCRIPT),
                "-SourceSharedDataDir",
                str(source),
                "-OutputSharedDataDir",
                str(output),
                "-SchemaId",
                schema_id,
                "-SchemaCustomOverlay",
                str(overlay),
                "-ManifestOutput",
                str(manifest),
            ],
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=30,
            cwd=REPO_ROOT,
        )

    def test_stages_overlay_at_schema_derived_destination_and_binds_hashes(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            (source / "default.yaml").write_text(
                "schema_list:\n  - schema: renamed_cangjie\n",
                encoding="utf-8",
                newline="\n",
            )
            (source / "renamed_cangjie.schema.yaml").write_text(
                "schema:\n  schema_id: renamed_cangjie\ntranslator:\n  dictionary: cangjie5\n",
                encoding="utf-8",
                newline="\n",
            )
            overlay = root / "review-marker.yaml"
            overlay.write_text(
                "patch:\n  translator/yune_sentence_policy: upstream_script\n",
                encoding="utf-8",
                newline="\n",
            )
            output = root / "staged"
            manifest = root / "staging-manifest.json"
            source_schema = source / "renamed_cangjie.schema.yaml"
            source_schema_before = source_schema.read_bytes()
            default_before = (source / "default.yaml").read_bytes()

            completed = self.run_stage(source, output, overlay, manifest)
            self.assertEqual(completed.returncode, 0, completed.stderr or completed.stdout)
            staged_schema = output / "renamed_cangjie.schema.yaml"
            self.assertEqual(source_schema.read_bytes(), source_schema_before)
            self.assertEqual((output / "default.yaml").read_bytes(), default_before)
            self.assertIn(
                "translator:\n  yune_sentence_policy: upstream_script\n  dictionary: cangjie5\n",
                staged_schema.read_text(encoding="utf-8"),
            )
            self.assertFalse((output / "renamed_cangjie.custom.yaml").exists())
            self.assertFalse((output / "review-marker.yaml").exists())
            record = json.loads(manifest.read_text(encoding="utf-8"))
            self.assertEqual(record["schema_id"], "renamed_cangjie")
            self.assertEqual(
                record["patched_schema_destination"],
                "renamed_cangjie.schema.yaml",
            )
            self.assertEqual(record["schema_patch_overlay_sha256"], sha256(overlay))
            self.assertNotEqual(
                record["source_schema_sha256"], record["staged_schema_sha256"]
            )
            self.assertEqual(record["tool_sha256"], sha256(SCRIPT))
            self.assertEqual(record["tool_version"], "2")

    def test_manifest_parent_may_already_exist(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            (source / "default.yaml").write_text(
                "schema_list:\n  - schema: renamed_cangjie\n",
                encoding="utf-8",
                newline="\n",
            )
            (source / "renamed_cangjie.schema.yaml").write_text(
                "schema:\n  schema_id: renamed_cangjie\ntranslator:\n  dictionary: cangjie5\n",
                encoding="utf-8",
                newline="\n",
            )
            overlay = root / "overlay.yaml"
            overlay.write_text(
                "patch:\n  translator/yune_sentence_policy: upstream_script\n",
                encoding="utf-8",
                newline="\n",
            )
            output = root / "staged"
            # `root` already exists, just as a drive root does in a real run.
            manifest = root / "manifest.json"

            completed = self.run_stage(source, output, overlay, manifest)
            self.assertEqual(completed.returncode, 0, completed.stderr or completed.stdout)
            self.assertTrue(manifest.is_file())

    def test_accepts_block_translator_with_trailing_comment(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            (source / "renamed_cangjie.schema.yaml").write_text(
                "schema:\n  schema_id: renamed_cangjie\n"
                "translator:  # canonical block\n  dictionary: cangjie5\n",
                encoding="utf-8",
                newline="\n",
            )
            overlay = root / "overlay.yaml"
            overlay.write_text(
                "patch:\n  translator/yune_sentence_policy: upstream_script\n",
                encoding="utf-8",
                newline="\n",
            )
            output = root / "staged"
            manifest = root / "manifest.json"

            completed = self.run_stage(source, output, overlay, manifest)
            self.assertEqual(completed.returncode, 0, completed.stderr or completed.stdout)
            self.assertIn(
                "translator:  # canonical block\n"
                "  yune_sentence_policy: upstream_script\n"
                "  dictionary: cangjie5\n",
                (output / "renamed_cangjie.schema.yaml").read_text(
                    encoding="utf-8"
                ),
            )

    def test_accepts_crlf_schema_with_nested_schema_entries(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            (source / "renamed_cangjie.schema.yaml").write_bytes(
                b"schema:\r\n"
                b"  schema_id: renamed_cangjie\r\n"
                b"  author:\r\n"
                b"    - pinned upstream\r\n"
                b"translator:\r\n"
                b"  dictionary: cangjie5\r\n"
            )
            overlay = root / "overlay.yaml"
            overlay.write_text(
                "patch:\n  translator/yune_sentence_policy: upstream_script\n",
                encoding="utf-8",
                newline="\n",
            )
            output = root / "staged"
            manifest = root / "manifest.json"

            completed = self.run_stage(source, output, overlay, manifest)
            self.assertEqual(completed.returncode, 0, completed.stderr or completed.stdout)
            staged = (output / "renamed_cangjie.schema.yaml").read_bytes()
            self.assertIn(
                b"translator:\r\n"
                b"  yune_sentence_policy: upstream_script\r\n"
                b"  dictionary: cangjie5\r\n",
                staged,
            )

    def test_rejects_noncanonical_patch_before_staging(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            (source / "renamed_cangjie.schema.yaml").write_text(
                "schema:\n  schema_id: renamed_cangjie\ntranslator:\n  dictionary: cangjie5\n",
                encoding="utf-8",
                newline="\n",
            )
            overlay = root / "overlay.yaml"
            overlay.write_text(
                "patch:\n  translator/yune_sentence_policy: legacy\n",
                encoding="utf-8",
                newline="\n",
            )
            output = root / "staged"
            manifest = root / "manifest.json"

            completed = self.run_stage(source, output, overlay, manifest)
            self.assertNotEqual(completed.returncode, 0)
            self.assertFalse(output.exists())
            self.assertFalse(manifest.exists())

    def test_rejects_inline_translator_before_staging(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            (source / "renamed_cangjie.schema.yaml").write_text(
                "schema:\n  schema_id: renamed_cangjie\n"
                "translator: { dictionary: cangjie5 }\n",
                encoding="utf-8",
                newline="\n",
            )
            overlay = root / "overlay.yaml"
            overlay.write_text(
                "patch:\n  translator/yune_sentence_policy: upstream_script\n",
                encoding="utf-8",
                newline="\n",
            )
            output = root / "staged"
            manifest = root / "manifest.json"

            completed = self.run_stage(source, output, overlay, manifest)
            self.assertNotEqual(completed.returncode, 0)
            self.assertFalse(output.exists())
            self.assertFalse(manifest.exists())

    def test_rejects_mismatched_schema_id_before_staging(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            (source / "renamed_cangjie.schema.yaml").write_text(
                "schema:\n  schema_id: another_schema\n"
                "translator:\n  dictionary: cangjie5\n",
                encoding="utf-8",
                newline="\n",
            )
            overlay = root / "overlay.yaml"
            overlay.write_text(
                "patch:\n  translator/yune_sentence_policy: upstream_script\n",
                encoding="utf-8",
                newline="\n",
            )
            output = root / "staged"
            manifest = root / "manifest.json"

            completed = self.run_stage(source, output, overlay, manifest)
            self.assertNotEqual(completed.returncode, 0)
            self.assertFalse(output.exists())
            self.assertFalse(manifest.exists())

    def test_rejects_missing_or_duplicate_schema_id_before_staging(self):
        for schema_text in (
            "schema:\n  name: no id\ntranslator:\n  dictionary: cangjie5\n",
            "schema:\n  schema_id: renamed_cangjie\n"
            "  schema_id: renamed_cangjie\n"
            "translator:\n  dictionary: cangjie5\n",
        ):
            with self.subTest(schema_text=schema_text):
                with tempfile.TemporaryDirectory() as temp:
                    root = Path(temp)
                    source = root / "source"
                    source.mkdir()
                    (source / "renamed_cangjie.schema.yaml").write_text(
                        schema_text,
                        encoding="utf-8",
                        newline="\n",
                    )
                    overlay = root / "overlay.yaml"
                    overlay.write_text(
                        "patch:\n"
                        "  translator/yune_sentence_policy: upstream_script\n",
                        encoding="utf-8",
                        newline="\n",
                    )
                    output = root / "staged"
                    manifest = root / "manifest.json"

                    completed = self.run_stage(source, output, overlay, manifest)
                    self.assertNotEqual(completed.returncode, 0)
                    self.assertFalse(output.exists())
                    self.assertFalse(manifest.exists())

    def test_rejects_path_like_schema_id_before_creating_outputs(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            overlay = root / "overlay.yaml"
            overlay.write_text("patch: {}\n", encoding="utf-8", newline="\n")
            output = root / "staged"
            manifest = root / "manifest.json"

            completed = self.run_stage(
                source,
                output,
                overlay,
                manifest,
                schema_id="../cangjie5",
            )
            self.assertNotEqual(completed.returncode, 0)
            self.assertFalse(output.exists())
            self.assertFalse(manifest.exists())

    def test_refuses_existing_output_without_mutation(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            overlay = root / "overlay.yaml"
            overlay.write_text("patch: {}\n", encoding="utf-8", newline="\n")
            output = root / "staged"
            output.mkdir()
            sentinel = output / "sentinel.txt"
            sentinel.write_text("keep", encoding="utf-8")
            manifest = root / "manifest.json"

            completed = self.run_stage(source, output, overlay, manifest)
            self.assertNotEqual(completed.returncode, 0)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")
            self.assertFalse(manifest.exists())

    def test_refuses_manifest_inside_source_tree(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            overlay = root / "overlay.yaml"
            overlay.write_text("patch: {}\n", encoding="utf-8", newline="\n")
            output = root / "staged"
            manifest = source / "manifest.json"

            completed = self.run_stage(source, output, overlay, manifest)
            self.assertNotEqual(completed.returncode, 0)
            self.assertFalse(output.exists())
            self.assertFalse(manifest.exists())

    def test_canonical_path_check_rejects_junction_alias_inside_source(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            overlay = root / "overlay.yaml"
            overlay.write_text("patch: {}\n", encoding="utf-8", newline="\n")
            alias = root / "source-alias"
            created = subprocess.run(
                ["cmd", "/c", "mklink", "/J", str(alias), str(source)],
                check=False,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if created.returncode != 0 or not alias.exists():
                self.skipTest("junction creation is required for canonical-path coverage")
            try:
                output = alias / "staged"
                manifest = root / "manifest.json"
                completed = self.run_stage(source, output, overlay, manifest)
                self.assertNotEqual(completed.returncode, 0)
                self.assertFalse((source / "staged").exists())
                self.assertFalse(manifest.exists())
            finally:
                if alias.exists():
                    alias.rmdir()


if __name__ == "__main__":
    unittest.main()
