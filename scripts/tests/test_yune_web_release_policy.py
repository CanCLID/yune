import json
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.classify_yune_web_release import changed_paths, output_safe, requires_release


REPO_ROOT = Path(__file__).resolve().parents[2]


def workflow_job(workflow: str, name: str) -> str:
    match = re.search(
        rf"^  {re.escape(name)}:\n(?P<body>.*?)(?=^  [a-zA-Z0-9_-]+:\n|\Z)",
        workflow,
        re.MULTILINE | re.DOTALL,
    )
    if match is None:
        raise AssertionError(f"workflow job not found: {name}")
    return match.group(0)


class YuneWebReleasePolicyTests(unittest.TestCase):
    def test_workflow_does_not_override_reserved_github_environment_names(self):
        workflow = (
            REPO_ROOT / ".github" / "workflows" / "deploy-yune-web.yml"
        ).read_text(encoding="utf-8")
        overridden = re.findall(r"^\s+(GITHUB_[A-Z0-9_]+):\s", workflow, re.MULTILINE)
        self.assertEqual(overridden, [])

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

    def test_web06_release_plumbing_paths_require_release(self):
        for path in (
            "apps/yune-web/e2e/yune-web06-smoothness.spec.ts",
            "apps/yune-web/e2e/playwright.web06.config.ts",
            "apps/yune-web/e2e/run-public-web06-gate.mjs",
            "apps/yune-web/e2e/run-public-web06-gate.test.mjs",
            "apps/yune-web/e2e/verify_archive_dist_identity.py",
            "apps/yune-web/e2e/test_verify_archive_dist_identity.py",
            "apps/yune-web/public-demo/certify-public-release.sh",
        ):
            with self.subTest(path=path):
                self.assertTrue(requires_release(path))

    def test_release_seals_once_then_certifies_both_gates_over_extracted_bytes(self):
        workflow = (
            REPO_ROOT / ".github" / "workflows" / "deploy-yune-web.yml"
        ).read_text(encoding="utf-8")
        build = workflow_job(workflow, "build-certify")
        build_step = build.index("- name: Build pinned public artifact")
        seal_step = build.index("- name: Seal public artifact once before measurement")
        extract_step = build.index("- name: Safely extract and reconcile sealed bytes")
        certify_step = build.index(
            "- name: Run unchanged WEB03-11 and distinct WEB06 certification over sealed bytes"
        )
        self.assertLess(build_step, seal_step)
        self.assertLess(seal_step, extract_step)
        self.assertLess(extract_step, certify_step)
        self.assertEqual(
            workflow.count(
                'tar -C apps/yune-web/public-demo/dist -czf "$EVIDENCE_ROOT/yune-web-dist.tar.gz" .'
            ),
            1,
        )
        self.assertNotIn("build-public-release.sh", build[seal_step + 1 :])
        self.assertIn("bundle.extractall(destination, members=members, filter=\"data\")", build)
        self.assertIn("node apps/yune-web/e2e/verify-local-artifact.mjs", build)
        self.assertIn(
            "YUNE_WEB_CERTIFIED_DIST_ROOT: ${{ runner.temp }}/certified-release-dist",
            build,
        )
        self.assertIn(
            "YUNE_WEB_CERTIFIED_ARCHIVE: ${{ runner.temp }}/yune-web-release-${{ github.sha }}/yune-web-dist.tar.gz",
            build,
        )
        self.assertIn(
            "YUNE_WEB_CERTIFIED_ARCHIVE_SHA256: ${{ steps.seal.outputs.archive_sha256 }}",
            build,
        )

        certify = (
            REPO_ROOT
            / "apps"
            / "yune-web"
            / "public-demo"
            / "certify-public-release.sh"
        ).read_text(encoding="utf-8")
        web03 = "npm --prefix apps/yune-web/e2e run test:e2e:input-latency:public"
        web06 = "npm --prefix apps/yune-web/e2e run test:e2e:web06:public"
        self.assertEqual(certify.count(web03), 1)
        self.assertEqual(certify.count(web06), 1)
        self.assertLess(certify.index(web03), certify.index(web06))
        self.assertIn('ln -s "$certified_dist_root" "$DEFAULT_DIST"', certify)
        self.assertIn("verify_archive_dist_identity.py", certify)
        self.assertIn('--archive "$configured_archive"', certify)
        self.assertIn('--dist "$certified_dist_input"', certify)
        self.assertIn(
            '--expected-archive-sha256 "$certified_archive_sha256"', certify
        )
        self.assertIn("archive-dist-identity.json", certify)
        self.assertLess(
            certify.index('mv -- "$DEFAULT_DIST" "$alias_temp/build-output-dist"'),
            certify.index("aliased_default_dist=true"),
        )
        self.assertLess(
            certify.index("aliased_default_dist=true"),
            certify.index('ln -s "$certified_dist_root" "$DEFAULT_DIST"'),
        )
        self.assertNotIn('export YUNE_WEB_WEB06_DIST_ROOT="$certified_dist_root"', certify)
        self.assertIn('export YUNE_WEB_CERTIFIED_ARCHIVE="$configured_archive"', certify)
        self.assertIn(
            'export YUNE_WEB_CERTIFIED_ARCHIVE_SHA256="$certified_archive_sha256"',
            certify,
        )
        self.assertNotIn("build-public-release.sh", certify)

    def test_preview_stops_and_manual_production_reuses_prior_run_bytes(self):
        workflow = (
            REPO_ROOT / ".github" / "workflows" / "deploy-yune-web.yml"
        ).read_text(encoding="utf-8")
        preview_canary = workflow_job(workflow, "preview-canary")
        classify = workflow_job(workflow, "classify")
        authorize_production = workflow_job(workflow, "authorize-production")
        validate_approved_preview = workflow_job(
            workflow, "validate-approved-preview"
        )
        deploy_preview = workflow_job(workflow, "deploy-preview")
        deploy_production = workflow_job(workflow, "deploy-production")
        verify_production = workflow_job(workflow, "verify-production")

        self.assertIn("- build-certify", preview_canary)
        self.assertIn("- deploy-preview", preview_canary)
        self.assertIn(
            "YUNE_WEB_CERTIFIED_ARCHIVE_SHA256: ${{ needs.build-certify.outputs.archive_sha256 }}",
            preview_canary,
        )
        self.assertIn("run test:e2e:input-latency", preview_canary)
        self.assertIn("run test:e2e:web06:preview-reconcile", preview_canary)
        self.assertIn("run test:e2e:web06:preview-canary", preview_canary)
        self.assertIn("web06ExistingNormalGuardRapidJyutpingStatus", preview_canary)
        self.assertIn(
            "name: ${{ needs.build-certify.outputs.artifact_name }}", preview_canary
        )
        self.assertNotIn("YUNE_WEB_WEB06_DIST_ROOT", preview_canary)
        self.assertIn(
            "YUNE_WEB_CERTIFIED_ARCHIVE=%s/preview-release/yune-web-dist.tar.gz",
            preview_canary,
        )
        self.assertIn("node apps/yune-web/e2e/verify-local-artifact.mjs", preview_canary)
        self.assertLess(
            preview_canary.index("run test:e2e:web06:preview-reconcile"),
            preview_canary.index(
                "Run unchanged WEB03 and frozen WEB06 preview canaries once"
            ),
        )
        self.assertNotIn("needs: preview-canary", authorize_production)
        self.assertIn(
            "if: github.event_name == 'push' || inputs.operation == 'setup-retry'",
            classify,
        )
        self.assertIn(
            "if: github.event_name == 'workflow_dispatch' && inputs.operation == 'production-promotion'",
            authorize_production,
        )
        self.assertIn("PROMOTE_EXACT_PREVIEW_BYTES", authorize_production)
        self.assertIn("must contain non-whitespace text", authorize_production)
        self.assertIn("production_approval_record", workflow)
        self.assertIn(
            'gh api "repos/$GITHUB_REPOSITORY/actions/runs/$APPROVED_PREVIEW_RUN_ID"',
            authorize_production,
        )
        self.assertIn('.conclusion == "success"', authorize_production)
        self.assertIn('.run_attempt == 1', authorize_production)
        self.assertIn('.head_sha == $sha', authorize_production)
        self.assertIn('.path == $workflow_path', authorize_production)
        self.assertIn(
            "artifact_name=yune-web-release-%s-%s-1", authorize_production
        )
        self.assertIn(
            "canary_artifact_name=yune-web-preview-canary-%s-%s-1",
            authorize_production,
        )
        self.assertIn(
            "preview_deployment_artifact_name=yune-web-preview-deployment-%s-%s-1",
            authorize_production,
        )

        self.assertIn("needs: authorize-production", validate_approved_preview)
        self.assertEqual(validate_approved_preview.count("run-id:"), 3)
        self.assertIn("approved-preview-deployment/deployment.json", validate_approved_preview)
        self.assertIn("approved-preview-deployment/project-interlock.json", validate_approved_preview)
        self.assertIn(
            "web06-preview-reconciliation-status.json", validate_approved_preview
        )
        self.assertIn("web06-public-gate-status.json", validate_approved_preview)
        self.assertIn('web03Status == "passed"', validate_approved_preview)
        self.assertIn(
            'web06ExistingNormalGuardRapidJyutpingStatus == "passed"',
            validate_approved_preview,
        )
        self.assertIn("node apps/yune-web/e2e/verify-local-artifact.mjs", validate_approved_preview)
        self.assertIn("environment: yune-web-production", deploy_production)

        self.assertIn(
            "name: ${{ needs.build-certify.outputs.artifact_name }}", deploy_preview
        )
        self.assertIn(
            "EXPECTED_ARCHIVE_SHA256: ${{ needs.build-certify.outputs.archive_sha256 }}",
            deploy_preview,
        )
        self.assertIn("- validate-approved-preview", deploy_production)
        self.assertNotIn("- build-certify", deploy_production)
        self.assertIn(
            "name: ${{ needs.authorize-production.outputs.artifact_name }}",
            deploy_production,
        )
        self.assertIn(
            "EXPECTED_ARCHIVE_SHA256: ${{ needs.authorize-production.outputs.archive_sha256 }}",
            deploy_production,
        )
        self.assertIn(
            "run-id: ${{ needs.authorize-production.outputs.preview_run_id }}",
            deploy_production,
        )
        self.assertNotIn(
            "${{ inputs.production_approval_record }}",
            deploy_production[deploy_production.index("    steps:") :],
        )
        for deployment in (deploy_preview, deploy_production):
            self.assertNotIn("build-public-release.sh", deployment)

        for production in (deploy_production, verify_production):
            self.assertNotIn("playwright", production.lower())
            self.assertNotIn("test:e2e", production)
        self.assertIn("node apps/yune-web/e2e/verify-deployed-artifact.mjs", verify_production)

    def test_local_fallback_retains_archive_and_evidence_outside_worktrees(self):
        certify = (
            REPO_ROOT
            / "apps"
            / "yune-web"
            / "public-demo"
            / "certify-public-release.sh"
        ).read_text(encoding="utf-8")
        self.assertIn("new_persistent_external_root", certify)
        self.assertIn("evidence-output-path.py default", certify)
        self.assertIn("evidence-output-path.py validate", certify)
        self.assertIn("YUNE_WEB_CERTIFICATION_ROOT", certify)
        self.assertIn("Retained WEB06 certification archive and evidence", certify)
        self.assertNotIn("certification_temp", certify)
        cleanup = certify[certify.index("cleanup() {") : certify.index("trap cleanup EXIT")]
        self.assertNotIn("persistent_certification_root", cleanup)

    def test_classifier_credentials_and_cloudflare_interlocks_remain_partitioned(self):
        workflow = (
            REPO_ROOT / ".github" / "workflows" / "deploy-yune-web.yml"
        ).read_text(encoding="utf-8")
        classify = workflow_job(workflow, "classify")
        self.assertIn("python3 -m unittest scripts.tests.test_yune_web_release_policy", classify)
        self.assertIn("python3 scripts/classify_yune_web_release.py", classify)

        secret_jobs = []
        for name in re.findall(r"^  ([a-zA-Z0-9_-]+):$", workflow, re.MULTILINE):
            if "secrets." in workflow_job(workflow, name):
                secret_jobs.append(name)
        self.assertEqual(secret_jobs, ["deploy-preview", "deploy-production"])

        for name in ("deploy-preview", "deploy-production"):
            deployment = workflow_job(workflow, name)
            self.assertIn("Assert Cloudflare Git auto-deploy remains disabled", deployment)
            self.assertIn("production_deployments_enabled == false", deployment)
            self.assertIn('preview_deployment_setting == "none"', deployment)
            self.assertIn("wrangler pages deploy", deployment)

    def test_web06_package_and_runner_expose_only_focused_release_scopes(self):
        package = json.loads(
            (REPO_ROOT / "apps" / "yune-web" / "e2e" / "package.json").read_text(
                encoding="utf-8"
            )
        )
        scripts = package["scripts"]
        self.assertEqual(
            scripts["test:e2e:input-latency:public"],
            "node run-public-latency-gate.mjs",
        )
        self.assertEqual(
            scripts["test:e2e:web06:public"], "node run-public-web06-gate.mjs"
        )
        self.assertEqual(
            scripts["test:e2e:web06:preview-reconcile"],
            "node run-public-web06-gate.mjs --scope preview-canary --verify-only",
        )
        self.assertEqual(
            scripts["test:e2e:web06:preview-canary"],
            "node run-public-web06-gate.mjs --scope preview-canary",
        )
        self.assertEqual(
            scripts["test:web06-release-plumbing"],
            "node --test run-public-web06-gate.test.mjs public-artifact-verifier.test.mjs && python3 -B -m unittest test_verify_archive_dist_identity.py",
        )

        runner = (
            REPO_ROOT / "apps" / "yune-web" / "e2e" / "run-public-web06-gate.mjs"
        ).read_text(encoding="utf-8")
        self.assertIn('grep: "@web06-full"', runner)
        self.assertIn('grep: "@web06-preview-canary"', runner)
        self.assertIn('expectedPreviewScenarios: "existing-normal-guard,rapid-jyutping"', runner)
        self.assertIn('"playwright.web06.config.ts"', runner)
        self.assertIn('"yune-web06-smoothness.spec.ts"', runner)
        self.assertIn("supplied artifact roots are forbidden", runner)
        self.assertIn('requiredString(environment, "YUNE_WEB_CERTIFIED_ARCHIVE")', runner)
        self.assertIn("validateArchive", runner)
        self.assertIn("extractCertifiedArchive", runner)
        self.assertIn("safeExtractionProgram", runner)
        self.assertIn("validateLocalBundle", runner)
        self.assertIn("reconcileRemoteBundle", runner)
        self.assertIn("validateRemoteMetadata", runner)
        self.assertIn("validateRemoteFile", runner)
        self.assertIn("evidence-output-path.py", runner)
        self.assertIn("assertNoSymlinkComponents", runner)
        self.assertIn("assertStrictDescendant", runner)
        for forbidden in ("build-public-release.sh", "build:public", "wasm-build"):
            self.assertNotIn(forbidden, runner)

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
