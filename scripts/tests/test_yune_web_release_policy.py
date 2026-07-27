import json
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.classify_yune_web_release import (
    changed_paths,
    is_web06_governed_path,
    is_web06_handoff_marker,
    output_safe,
    requires_release,
)


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

    def test_web06_governed_inventory_and_unique_markers_are_exact(self):
        governed = (
            "apps/yune-web/src/App.tsx",
            "apps/yune-web/src/Candidate.tsx",
            "apps/yune-web/src/CandidatePanel.tsx",
            "apps/yune-web/src/YuneControlSurface.tsx",
            "apps/yune-web/src/YuneUserdbViewer.tsx",
            "apps/yune-web/src/rime.ts",
            "apps/yune-web/src/types.ts",
            "apps/yune-web/src/worker.ts",
            "apps/yune-web/src/yune-integration/adapter.ts",
            "apps/yune-web/src/yune-integration/private-protocol.ts",
            "apps/yune-web/yune-integration/private-protocol.test.ts",
            "apps/yune-web/yune-integration/web06-private-pipeline.test.ts",
            "apps/yune-web/e2e/web06-metric-contract.mjs",
            "apps/yune-web/e2e/web06-receipt-parser.test.mjs",
            "apps/yune-web/e2e/web06-collector.mjs",
            "apps/yune-web/e2e/yune-web06-smoothness.spec.ts",
            "apps/yune-web/e2e/playwright.web06-peer.config.ts",
            "apps/yune-web/e2e/run-web06-peer-lane.ts",
            "apps/yune-web/e2e/verify-web06-peer-evidence.ts",
            "apps/yune-web/e2e/startup-benchmark/web06-peer-artifacts.ts",
            "apps/yune-web/e2e/startup-benchmark/comparator-browser-endpoint.ts",
            "apps/yune-web/e2e/startup-benchmark/comparator-endpoint.ts",
            "packages/yune-web-runtime/src/observation.ts",
            "packages/yune-web-runtime/src/response.ts",
            "packages/yune-web-runtime/src/runtime.ts",
            "packages/yune-web-runtime/test/fake-module.ts",
            "packages/yune-web-runtime/test/observation.test.ts",
            "packages/yune-web-runtime/test/public-api.test.ts",
            "apps/yune-web/public-demo/certify-public-release.sh",
            "apps/yune-web/src/web06-observer.ts",
            "apps/yune-web/src/yune-integration/web06-main-protocol.ts",
            "apps/yune-web/src/yune-integration/web06-worker-collector.ts",
            "apps/yune-web/package.json",
            "apps/yune-web/package-lock.json",
            ".github/workflows/deploy-yune-web.yml",
        )
        for path in governed:
            with self.subTest(path=path):
                self.assertTrue(is_web06_governed_path(path))

        for marker in (
            "apps/yune-web/e2e/web06-collector.mjs",
            "apps/yune-web/e2e/run-web06-peer-lane.ts",
            "apps/yune-web/e2e/playwright.web06.config.ts",
            "apps/yune-web/e2e/yune-web06-smoothness.spec.ts",
        ):
            self.assertTrue(is_web06_handoff_marker(marker))
        self.assertFalse(is_web06_handoff_marker("apps/yune-web/src/App.tsx"))
        self.assertFalse(is_web06_handoff_marker(".github/workflows/deploy-yune-web.yml"))
        self.assertFalse(is_web06_governed_path("apps/yune-web/src/unrelated.ts"))
        self.assertFalse(is_web06_governed_path("apps/yune-web/e2e/run-web07-lane.ts"))

    def test_web06_push_is_a_pending_identity_noop_before_build(self):
        workflow = (
            REPO_ROOT / ".github" / "workflows" / "deploy-yune-web.yml"
        ).read_text(encoding="utf-8")
        classify = workflow_job(workflow, "classify")
        pending = workflow_job(workflow, "web06-measured-handoff-required")
        build = workflow_job(workflow, "build-certify")
        self.assertIn("web06_handoff_required", classify)
        self.assertIn(
            "needs.classify.outputs.web06_handoff_required == 'true'", pending
        )
        self.assertIn(
            "needs.classify.outputs.web06_handoff_required != 'true'", build
        )
        self.assertIn('operation: "measured-artifact-handoff-required"', pending)
        self.assertIn('deploymentIdentityStatus: "pending"', pending)
        self.assertIn("fullSuccessEligible: false", pending)
        self.assertIn("buildInvoked: false", pending)
        self.assertIn("cannot satisfy WEB06 Full", pending)
        dispatch_inputs = workflow[: workflow.index("permissions:")]
        self.assertNotIn("certification_mode:", dispatch_inputs)
        self.assertNotIn("measured-web06", dispatch_inputs)
        for forbidden in (
            "build-public-release.sh",
            "wrangler pages deploy",
            "playwright",
        ):
            self.assertNotIn(forbidden, pending)

    def test_web06_marker_activates_exact_owner_handoff_without_permanent_app_block(self):
        classifier = REPO_ROOT / "scripts" / "classify_yune_web_release.py"
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

            production_paths = (
                "apps/yune-web/src/worker.ts",
                "apps/yune-web/src/rime.ts",
                "apps/yune-web/src/CandidatePanel.tsx",
                "packages/yune-web-runtime/src/runtime.ts",
                "packages/yune-web-runtime/src/observation.ts",
                "apps/yune-web/e2e/startup-benchmark/comparator-browser-endpoint.ts",
                "scripts/classify_yune_web_release.py",
                "apps/yune-web/src/web06-observer.ts",
                "apps/yune-web/e2e/playwright.web06-peer.config.ts",
            )
            for relative in production_paths:
                target = repo / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text("owned\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "web06"], cwd=repo, check=True)
            web06_head = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=repo, text=True
            ).strip()
            output = repo / "web06-output.txt"
            subprocess.run(
                [
                    "python3",
                    str(classifier),
                    "--repo-root",
                    str(repo),
                    "--base",
                    base,
                    "--head",
                    web06_head,
                    "--github-output",
                    str(output),
                ],
                check=True,
                stdout=subprocess.PIPE,
                text=True,
            )
            classified = output.read_text(encoding="utf-8")
            self.assertIn("release_required=true\n", classified)
            self.assertIn("web06_handoff_required=true\n", classified)
            self.assertIn("milestone marker(s)", classified)

            # Once WEB06 itself is the deployed base, a later generic rime.ts
            # edit has no milestone-unique marker in the range and keeps the
            # legacy non-WEB06 release path available.
            rime = repo / "apps/yune-web/src/rime.ts"
            rime.write_text("later unrelated product edit\n", encoding="utf-8")
            subprocess.run(["git", "add", str(rime)], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "later-app"], cwd=repo, check=True)
            later_head = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=repo, text=True
            ).strip()
            later_output = repo / "later-output.txt"
            subprocess.run(
                [
                    "python3",
                    str(classifier),
                    "--repo-root",
                    str(repo),
                    "--base",
                    web06_head,
                    "--head",
                    later_head,
                    "--github-output",
                    str(later_output),
                ],
                check=True,
                stdout=subprocess.PIPE,
                text=True,
            )
            later = later_output.read_text(encoding="utf-8")
            self.assertIn("release_required=true\n", later)
            self.assertIn("web06_handoff_required=false\n", later)

    def test_release_seals_once_and_selects_legacy_without_web06_claims(self):
        workflow = (
            REPO_ROOT / ".github" / "workflows" / "deploy-yune-web.yml"
        ).read_text(encoding="utf-8")
        build = workflow_job(workflow, "build-certify")
        build_step = build.index("- name: Build pinned public artifact")
        seal_step = build.index("- name: Seal public artifact once before measurement")
        extract_step = build.index("- name: Safely extract and reconcile sealed bytes")
        certify_step = build.index(
            "- name: Run classifier-selected legacy exact-artifact certification"
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
            "YUNE_WEB06_CERTIFIED_DIST_ROOT: ${{ runner.temp }}/certified-release-dist",
            build,
        )
        self.assertIn(
            "YUNE_WEB06_CERTIFIED_ARCHIVE: ${{ runner.temp }}/yune-web-release-${{ github.sha }}/yune-web-dist.tar.gz",
            build,
        )
        self.assertIn(
            "YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256: ${{ steps.seal.outputs.archive_sha256 }}",
            build,
        )
        self.assertIn("YUNE_WEB_RELEASE_CERTIFICATION_MODE: legacy", build)
        self.assertIn("Select legacy certification internally", build)
        self.assertIn("A WEB06-governed range cannot select legacy", build)

        certify = (
            REPO_ROOT
            / "apps"
            / "yune-web"
            / "public-demo"
            / "certify-public-release.sh"
        ).read_text(encoding="utf-8")
        web03 = "npm --prefix apps/yune-web/e2e run test:e2e:input-latency:public"
        self.assertEqual(certify.count(web03), 1)
        self.assertEqual(certify.count("run-public-web06-gate.mjs"), 2)
        self.assertIn('case "$certification_mode" in', certify)
        self.assertIn("legacy | measured-web06", certify)
        self.assertIn('if [ "$certification_mode" = legacy ]', certify)
        self.assertIn('"web06EvidenceConsumed": False', certify)
        self.assertIn('"web06ClaimMade": False', certify)
        self.assertLess(
            certify.index('if [ "$certification_mode" = legacy ]'),
            certify.index("--scope release-certification --verify-only"),
        )
        self.assertLess(
            certify.index("--scope release-certification --verify-only"),
            certify.rindex("run_web03_gate"),
        )
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
        self.assertNotIn('export YUNE_WEB06_DIST_ROOT="$certified_dist_root"', certify)
        self.assertIn('export YUNE_WEB06_CERTIFIED_ARCHIVE="$configured_archive"', certify)
        self.assertIn('export YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256="$certified_archive_sha256"', certify)
        self.assertIn("YUNE_WEB06_FINAL_SUITE_ATTESTATION", certify)
        self.assertIn('"selectorPolicy": "omitted"', certify)
        self.assertIn('"buildInvoked": False', certify)
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
            "YUNE_WEB_RELEASE_CERTIFICATION_MODE: ${{ needs.build-certify.outputs.certification_mode }}",
            preview_canary,
        )
        self.assertIn("run test:e2e:input-latency", preview_canary)
        self.assertIn("node apps/yune-web/e2e/verify-deployed-artifact.mjs", preview_canary)
        self.assertIn('"web06Status": "not-run"', preview_canary)
        self.assertIn('"web06ClaimMade": false', preview_canary)
        self.assertNotIn("run test:e2e:web06:", preview_canary)
        self.assertIn(
            "name: ${{ needs.build-certify.outputs.artifact_name }}", preview_canary
        )
        self.assertNotIn("YUNE_WEB_WEB06_DIST_ROOT", preview_canary)
        self.assertIn("node apps/yune-web/e2e/verify-local-artifact.mjs", preview_canary)
        self.assertLess(
            preview_canary.index("node apps/yune-web/e2e/verify-deployed-artifact.mjs"),
            preview_canary.index(
                "Run the unchanged legacy WEB03 preview canary once"
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
        self.assertIn("deployed-artifact-verification.json", validate_approved_preview)
        self.assertIn('web03Status == "passed"', validate_approved_preview)
        self.assertIn('web06Status == "not-run"', validate_approved_preview)
        self.assertIn("web06ClaimMade == false", validate_approved_preview)
        self.assertNotIn("web06-public-gate-status.json", validate_approved_preview)
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

    def test_setup_retry_is_same_source_attempt_one_and_cannot_retry_a_red(self):
        workflow = (
            REPO_ROOT / ".github" / "workflows" / "deploy-yune-web.yml"
        ).read_text(encoding="utf-8")
        classify = workflow_job(workflow, "classify")
        build = workflow_job(workflow, "build-certify")
        self.assertIn('grep -Eq \'^[1-9][0-9]*$\'', classify)
        self.assertIn('gh api "repos/$GITHUB_REPOSITORY/actions/runs/$SETUP_RETRY_OF"', classify)
        self.assertIn('gh run download "$SETUP_RETRY_OF"', classify)
        self.assertIn("web06-setup-retry.mjs", classify)
        self.assertIn('runAttempt: $runAttempt', classify)
        self.assertIn("YUNE_WEB_WORKFLOW_MEASUREMENT_STARTED_PATH", build)
        self.assertIn("web06-workflow-attempt.mjs", build)
        self.assertIn('version: "web06-workflow-attempt-input-v1"', build)
        self.assertNotIn('CERTIFY_OUTCOME" = failure ]; then', build)
        self.assertNotIn("valid_red_observed=true", build)
        retry_validator = (
            REPO_ROOT / "apps" / "yune-web" / "e2e" / "web06-setup-retry.mjs"
        ).read_text(encoding="utf-8")
        self.assertIn("Prior failure occurred after measurement started", retry_validator)
        self.assertIn("A prior valid RED may never be retried", retry_validator)

    def test_release_rejects_branch_d_and_shared_full_matrix_rerun(self):
        for relative in (
            "apps/yune-web/e2e/run-public-web06-gate.mjs",
            "apps/yune-web/e2e/run-web06-local-matrix.mjs",
            "apps/yune-web/e2e/web06-suite-attestation.mjs",
        ):
            source = (REPO_ROOT / relative).read_text(encoding="utf-8")
            self.assertNotIn('"D"', source)
        public_runner = (
            REPO_ROOT / "apps/yune-web/e2e/run-public-web06-gate.mjs"
        ).read_text(encoding="utf-8")
        self.assertNotIn('@web06-full', public_runner)
        self.assertIn('grep: "@web06-preview-canary"', public_runner)
        self.assertIn('disposition !== "PRODUCTION_REDUCTION"', public_runner)

    def test_production_verifier_checks_every_manifest_member(self):
        verifier = (
            REPO_ROOT / "apps/yune-web/e2e/verify-deployed-artifact.mjs"
        ).read_text(encoding="utf-8")
        self.assertIn("productionManifestRows(local.manifest)", verifier)
        self.assertIn("verifyProductionManifestMember(expected, bytes)", verifier)
        self.assertNotIn("runtimePaths", verifier)

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

        certify = (
            REPO_ROOT
            / "apps"
            / "yune-web"
            / "public-demo"
            / "certify-public-release.sh"
        ).read_text(encoding="utf-8")
        self.assertIn(
            'CLOUDFLARE_* | CF_* | WRANGLER_*) unset "$environment_name"',
            certify,
        )

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
            scripts["test:e2e:web06-peer-contract"],
            "playwright test --config playwright.web06-peer.config.ts --grep @contract",
        )
        self.assertEqual(
            scripts["run:e2e:web06-peer"],
            "node --experimental-strip-types run-web06-peer-lane.ts",
        )
        self.assertEqual(
            scripts["preview:web06:certified"],
            "node run-web06-certified-preview.mjs",
        )
        self.assertEqual(
            scripts["promote:web06:production"],
            "node run-web06-production-promotion.mjs",
        )
        self.assertEqual(
            scripts["test:web06-release-plumbing"],
            "node --test web06-metric-contract.test.mjs web06-receipt-parser.test.mjs web06-collector.test.mjs web06-suite-attestation.test.mjs web06-sealed-artifact-server.test.mjs web06-workflow-attempt.test.mjs run-web06-local-matrix.test.mjs run-public-web06-gate.test.mjs run-web06-certified-preview.test.mjs run-web06-production-promotion.test.mjs web06-setup-retry.test.mjs production-artifact-verifier.test.mjs public-artifact-verifier.test.mjs && python3 -B -m unittest test_verify_archive_dist_identity.py && python3 -B ../../../scripts/tests/test_public_evidence_privacy.py",
        )

        runner = (
            REPO_ROOT / "apps" / "yune-web" / "e2e" / "run-public-web06-gate.mjs"
        ).read_text(encoding="utf-8")
        self.assertIn('grep: "@web06-preview-canary"', runner)
        self.assertIn("WEB06_PREVIEW_SCENARIOS", runner)
        self.assertIn('"playwright.web06.config.ts"', runner)
        self.assertIn('"yune-web06-smoothness.spec.ts"', runner)
        self.assertIn("supplied artifact roots are forbidden", runner)
        self.assertIn('requiredString(environment, "YUNE_WEB06_CERTIFIED_ARCHIVE")', runner)
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
        self.assertIn("loadSealedArtifactSnapshot", runner)
        self.assertIn("startSealedArtifactServer", runner)
        self.assertNotIn("vite preview", runner)
        matrix = (
            REPO_ROOT / "apps" / "yune-web" / "e2e" / "run-web06-local-matrix.mjs"
        ).read_text(encoding="utf-8")
        self.assertIn("loadSealedArtifactSnapshot", matrix)
        self.assertIn("startSealedArtifactServer", matrix)
        self.assertIn("artifactResponseGuard", matrix)
        self.assertNotIn("vite preview", matrix)
        for forbidden in ("build-public-release.sh", "build:public", "wasm-build"):
            self.assertNotIn(forbidden, runner)

        preview = (
            REPO_ROOT / "apps" / "yune-web" / "e2e" / "run-web06-certified-preview.mjs"
        ).read_text(encoding="utf-8")
        self.assertIn('wrangler@${WRANGLER_VERSION}', preview)
        self.assertIn('operation: "preview-only-no-build"', preview)
        self.assertIn("reconcileRemoteBundle", preview)
        self.assertIn("assertNoPriorPreviewDeployment", preview)
        self.assertIn("consumePreviewAuthorization", preview)
        self.assertIn("previewMutationStarted", preview)
        self.assertIn("productionPromotionAttempted: false", preview)
        for forbidden in ("build-public-release.sh", "build:public", "wasm-build"):
            self.assertNotIn(forbidden, preview)

        production = (
            REPO_ROOT
            / "apps"
            / "yune-web"
            / "e2e"
            / "run-web06-production-promotion.mjs"
        ).read_text(encoding="utf-8")
        self.assertIn("PROMOTE_EXACT_PREVIEW_BYTES", production)
        self.assertIn("YUNE_WEB06_PRODUCTION_APPROVAL_RECORD", production)
        self.assertIn('open(consumptionPath, "wx"', production)
        self.assertIn('["ls-remote", "origin", "refs/heads/main"]', production)
        self.assertIn("validateGreenPreviewReceipt", production)
        self.assertIn("validatePreviewCanaryStatus", production)
        self.assertIn("reconcileRemoteBundle", production)
        self.assertIn('"--branch",\n        "main"', production)
        self.assertIn("buildInvoked: false", production)
        self.assertIn("CLOUDFLARE_PRODUCTION_ACCOUNT_ID", production)
        self.assertIn("CLOUDFLARE_PRODUCTION_API_TOKEN", production)
        for forbidden in ("build-public-release.sh", "build:public", "wasm-build"):
            self.assertNotIn(forbidden, production)

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
