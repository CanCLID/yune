import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.classify_yune_web_release import changed_paths, output_safe, requires_release


class YuneWebReleasePolicyTests(unittest.TestCase):
    def test_github_output_reason_escapes_line_breaks_and_percent(self):
        self.assertEqual(output_safe("a%\r\nb"), "a%25%0D%0Ab")

    def test_product_engine_and_gate_paths_require_release(self):
        for path in (
            "apps/yune-web/src/worker.ts",
            "apps/yune-web/public/schema/luna_pinyin.table.bin",
            "apps/yune-web/public/release-note.md",
            "apps/yune-web/e2e/yune-web-input-latency.spec.ts",
            "apps/yune-web/public-demo/cloudflare-pages-build.sh",
            "packages/yune-web-runtime/src/index.ts",
            "crates/yune-core/src/lib.rs",
            "crates/yune-rime-api/src/lib.rs",
            "scripts/yune-web-wasm-build.sh",
            "Cargo.lock",
            ".github/workflows/deploy-yune-web.yml",
            ".gitattributes",
        ):
            with self.subTest(path=path):
                self.assertTrue(requires_release(path))

    def test_copied_public_markdown_requires_release(self):
        for path in (
            "apps/yune-web/public-demo/README.md",
            "apps/yune-web/public-demo/PROVENANCE.md",
            "apps/yune-web/public-demo/asset-manifest.md",
            "apps/yune-web/public-demo/cache-policy.md",
        ):
            with self.subTest(path=path):
                self.assertTrue(requires_release(path))

    def test_documentation_and_historical_results_are_noops(self):
        for path in (
            "docs/roadmap.md",
            "README.md",
            "apps/yune-web/AGENTS.md",
            "apps/yune-web/src/yune-integration/README.md",
            "apps/yune-web/e2e/results/old/samples.json",
            "apps/yune-web/patches/yune-web-runtime.patch",
        ):
            with self.subTest(path=path):
                self.assertFalse(requires_release(path))

    def test_rename_out_of_release_tree_keeps_deleted_source_path(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
            subprocess.run(
                ["git", "config", "user.email", "release-policy@example.invalid"],
                cwd=repo,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Release Policy Test"],
                cwd=repo,
                check=True,
            )
            source = repo / "apps/yune-web/src/renamed.ts"
            source.parent.mkdir(parents=True)
            source.write_text("export {};\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "source"], cwd=repo, check=True)
            base = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=repo, text=True
            ).strip()
            destination = repo / "docs/renamed.md"
            destination.parent.mkdir()
            source.rename(destination)
            subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "rename"], cwd=repo, check=True)
            head = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=repo, text=True
            ).strip()

            paths = changed_paths(repo, base, head)
            self.assertIn("apps/yune-web/src/renamed.ts", paths)
            self.assertIn("docs/renamed.md", paths)
            self.assertTrue(any(requires_release(path) for path in paths))

    def test_newline_path_is_nul_parsed_and_output_safe(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
            subprocess.run(
                ["git", "config", "user.email", "release-policy@example.invalid"],
                cwd=repo,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Release Policy Test"],
                cwd=repo,
                check=True,
            )
            placeholder = repo / "placeholder"
            placeholder.write_text("baseline\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "baseline"], cwd=repo, check=True)
            base = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=repo, text=True
            ).strip()
            source = repo / "apps/yune-web/src/line\nbreak.ts"
            source.parent.mkdir(parents=True)
            source.write_text("export {};\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "newline"], cwd=repo, check=True)
            head = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=repo, text=True
            ).strip()

            paths = changed_paths(repo, base, head)
            self.assertEqual(paths, ["apps/yune-web/src/line\nbreak.ts"])
            self.assertTrue(requires_release(paths[0]))


if __name__ == "__main__":
    unittest.main()
