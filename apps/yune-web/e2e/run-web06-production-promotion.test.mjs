import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { promisify } from "node:util";

import {
  main,
  proveExactCurrentSource,
  resolveNewProductionDeployment,
  resolveProductionContract,
  validateProductionApprovalPayload,
} from "./run-web06-production-promotion.mjs";
import {
  WEB06_COLLECTOR_CONTRACT_SHA256,
} from "./web06-collector.mjs";
import {
  WEB06_BEHAVIOR_VERSION,
  WEB06_METRIC_VERSION,
  WEB06_SCENARIO_VERSION,
} from "./web06-suite-attestation.mjs";

const execFileAsync = promisify(execFile);
const sourceCommit = "1".repeat(40);
const sourceTree = "2".repeat(40);
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const wranglerInstallIdentity = Object.freeze({
  nodeExecutableSha256: "1".repeat(64),
  npmCliSha256: "2".repeat(64),
  npmPackageManifestSha256: "3".repeat(64),
  npmVersion: "11.16.0",
});
const wranglerRuntimeIdentity = Object.freeze({
  version: "4.111.0",
  resolved: "https://registry.npmjs.org/wrangler/-/wrangler-4.111.0.tgz",
  integrity:
    "sha512-bffpI9EyrnpKkF/1S+RaIv8oRD93GtbsA7TlfWwOsGJGB7VO3jVbdGzpC9TU7Bqom3z7jUxcte4Z9MPhaQ4HoQ==",
  rootPackageSha256: "4".repeat(64),
  lockfileSha256: "5".repeat(64),
  packageManifestSha256: "6".repeat(64),
  entrypointSha256: "7".repeat(64),
  nodeExecutableSha256: "1".repeat(64),
  installedClosureFileCount: 42,
  installedClosureBytes: 4096,
  installedClosureSha256: "8".repeat(64),
  nodeVersion: "v22.16.0",
});

function fakeWranglerDependencies(overrides = {}) {
  return {
    installPinnedWranglerRuntime: async ({ installationRoot }) => {
      await mkdir(installationRoot, { mode: 0o700 });
      return {
        root: installationRoot,
        identity: wranglerInstallIdentity,
      };
    },
    preparePinnedWranglerRuntime: async () => ({
      command: "/trusted/node",
      argumentsPrefix: ["/trusted/wrangler.js"],
      identity: wranglerRuntimeIdentity,
      assertCurrent: async () => {},
    }),
    ...overrides,
  };
}

async function fixture(root) {
  const dist = path.join(root, "dist");
  await mkdir(path.join(dist, "assets"), { recursive: true });
  await mkdir(path.join(dist, "schema"), { recursive: true });
  const files = new Map([
    ["assets/app.js", Buffer.from("app\n")],
    ["index.html", Buffer.from("<main>Yune</main>\n")],
    ["schema/rare.bin", Buffer.from("rare exact bytes\n")],
    ["schema-asset-manifest.json", Buffer.from("{}\n")],
    ["worker.js", Buffer.from("self.onmessage=()=>{};\n")],
    ["yune-web.wasm", Buffer.from([0, 97, 115, 109])],
  ]);
  for (const [relative, bytes] of files) {
    await writeFile(path.join(dist, ...relative.split("/")), bytes);
  }
  const manifest = {
    generatedFor: "yune-web",
    version: "web03-public-artifact-v1",
    files: [...files].map(([relative, bytes]) => ({
      path: relative,
      bytes: bytes.byteLength,
      sha256: digest(bytes),
    })),
  };
  const manifestBytes = Buffer.from(`${JSON.stringify(manifest)}\n`);
  const buildInfo = {
    sourceCommit,
    sourceTreeState: "clean",
    wasmSha256: manifest.files.find((row) => row.path === "yune-web.wasm").sha256,
    schemaManifestSha256: manifest.files.find((row) => row.path === "schema-asset-manifest.json").sha256,
    publicArtifactManifestSha256: digest(manifestBytes),
    toolchain: {
      emsdkVersion: "4.0.23",
      emscriptenReleaseCommit: "aaa43392544d695232b70eda706d751f18980c2a",
      emsdkRepositoryCommit: "db04e88298d9916fc51fcd3743045ca3eb695127",
      emccVersion:
        "emcc (Emscripten gcc/clang-like replacement + linker emulating GNU ld) 4.0.23 (7a5d93b50f6a3a35e85a0d2fc9e667b8498e6aed)",
      rustcVersion: "rustc 1.96.1 (31fca3adb 2026-06-26)",
      nodeVersion: "v22.16.0",
    },
  };
  const buildInfoBytes = Buffer.from(`${JSON.stringify(buildInfo)}\n`);
  await writeFile(path.join(dist, "build-info.json"), buildInfoBytes);
  await writeFile(path.join(dist, "public-artifact-manifest.json"), manifestBytes);

  const archivePath = path.join(root, "final.tar.gz");
  await execFileAsync("tar", ["-C", dist, "-czf", archivePath, "."], {
    env: { ...process.env, COPYFILE_DISABLE: "1" },
  });
  const archiveSha256 = digest(await readFile(archivePath));
  await writeFile(`${archivePath}.sha256`, `${archiveSha256}\n`);
  const identity = {
    version: "web06-target-identities-v1",
    metricContractVersion: WEB06_METRIC_VERSION,
    scenarioRegistryVersion: WEB06_SCENARIO_VERSION,
    behaviorPredicateVersion: WEB06_BEHAVIOR_VERSION,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    roles: {
      FINAL: {
        sourceCommit,
        sourceTree,
        sourceTreeState: "clean",
        archiveSha256,
        artifactManifestSha256: digest(manifestBytes),
        buildInfoSha256: digest(buildInfoBytes),
        selectedBranch: "A",
        disposition: "PRODUCTION_REDUCTION",
      },
    },
  };
  const finalEvidenceRoot = path.join(root, "final-evidence");
  await mkdir(finalEvidenceRoot);
  const finalSuitePath = path.join(
    finalEvidenceRoot,
    "suite-attestation.json",
  );
  await writeFile(finalSuitePath, '{"fixture":"final-suite"}\n');
  const finalSuiteAttestationSha256 = digest(await readFile(finalSuitePath));
  const previewUrl = "https://preview-id.yune-web.pages.dev/";
  const canaryStatus = {
    version: "web06-public-gate-status-v1",
    sourceCommit,
    sourceTree,
    archiveSha256,
    artifactManifestSha256: digest(manifestBytes),
    finalSuiteAttestationSha256,
    previewSuiteAttestationSha256: "9".repeat(64),
    disposition: "PRODUCTION_REDUCTION",
    scope: "preview-canary",
    appUrl: previewUrl,
    canaryStarted: true,
    status: "passed",
  };
  const previewCanaryStatusPath = path.join(root, "preview-canary-status.json");
  const canaryBytes = Buffer.from(`${JSON.stringify(canaryStatus)}\n`);
  await writeFile(previewCanaryStatusPath, canaryBytes);
  const previewReceipt = {
    version: "web06-certified-preview-v1",
    operation: "preview-only-no-build",
    sourceCommit,
    sourceTree,
    archiveSha256,
    artifactManifestSha256: digest(manifestBytes),
    finalSuiteAttestationSha256,
    localCertificationReceiptSha256: "a".repeat(64),
    previewCanaryStatusSha256: digest(canaryBytes),
    previewSuiteAttestationSha256: canaryStatus.previewSuiteAttestationSha256,
    previewAuthorizationConsumptionSha256: "b".repeat(64),
    wranglerVersion: "4.111.0",
    wranglerInstallIdentity,
    wranglerRuntimeIdentity,
    previewBranch: `web06-preview-${sourceCommit.slice(0, 12)}`,
    previewUrl,
    previewDeployment: {
      id: "verified-preview-deployment",
      url: previewUrl,
      environment: "preview",
      latestStageStatus: "success",
      triggerType: "ad_hoc",
      branch: `web06-preview-${sourceCommit.slice(0, 12)}`,
      commitHash: sourceCommit,
      commitDirty: false,
    },
    projectInterlock: {
      productionBranch: "main",
      productionDeploymentsEnabled: false,
      previewDeploymentSetting: "none",
    },
    remoteManifestFileCount: files.size,
    previewMutationStarted: true,
    productionPromotionAttempted: false,
    peerPackageAlignment: "DATA_CONFOUNDED",
    peerRatioStatus: "OMITTED",
    fullSuccessEligible: false,
    milestoneDisposition: "PENDING_EXPLICIT_PARTIAL_APPROVAL",
    closeoutAuthorized: false,
    productionPromotionAuthorized: false,
    status: "passed",
  };
  const previewReceiptPath = path.join(root, "green-preview.json");
  const previewBytes = Buffer.from(`${JSON.stringify(previewReceipt)}\n`);
  await writeFile(previewReceiptPath, previewBytes);
  const approval = {
    version: "web06-production-approval-v1",
    sourceCommit,
    sourceTree,
    archiveSha256,
    previewReceiptSha256: digest(previewBytes),
    approvalRecord: "user-approved-web06-production",
    confirmation: "PROMOTE_EXACT_PREVIEW_BYTES",
    acceptedMilestoneDisposition: "PARTIAL",
    acceptedLimitations: [
      "PEER_DATA_CONFOUNDED",
      "PEER_RATIO_OMITTED",
    ],
    previewFullSuccessEligible: false,
    nonce: "c".repeat(64),
    approvedAt: "2026-07-22T03:30:00Z",
  };
  const approvalPath = path.join(root, "production-approval.json");
  const approvalBytes = Buffer.from(`${JSON.stringify(approval)}\n`);
  await writeFile(approvalPath, approvalBytes);
  const approvalLedgerRoot = path.join(root, "production-approval-ledger");
  await mkdir(approvalLedgerRoot, { mode: 0o700 });
  return {
    approvalLedgerRoot,
    approvalPath,
    approvalSha256: digest(approvalBytes),
    archivePath,
    archiveSha256,
    buildInfoBytes,
    files,
    finalEvidenceRoot,
    finalSuitePath,
    identity,
    manifestBytes,
    previewCanaryStatusPath,
    previewReceiptPath,
    previewReceiptSha256: digest(previewBytes),
  };
}

const fixtureSuiteReader = (value) => async (file, expected) => {
  assert.equal(file, value.finalSuitePath);
  assert.equal(expected.expectation, "FINAL");
  assert.equal(expected.disposition, "PRODUCTION_REDUCTION");
  return {
    sha256: digest(await readFile(value.finalSuitePath)),
  };
};

function environment(value, evidenceRoot) {
  return {
    YUNE_WEB06_EXPECTED_SOURCE_COMMIT: sourceCommit,
    YUNE_WEB06_EXPECTED_SOURCE_TREE: sourceTree,
    YUNE_WEB06_CERTIFIED_ARCHIVE: value.archivePath,
    YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256: value.archiveSha256,
    YUNE_WEB06_FINAL_SUITE_ATTESTATION: value.finalSuitePath,
    YUNE_WEB06_FINAL_SUITE_EVIDENCE_ROOT: value.finalEvidenceRoot,
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: JSON.stringify(value.identity),
    YUNE_WEB06_SELECTED_BRANCH: "A",
    YUNE_WEB06_DISPOSITION: "PRODUCTION_REDUCTION",
    YUNE_WEB06_GREEN_PREVIEW_RECEIPT: value.previewReceiptPath,
    YUNE_WEB06_GREEN_PREVIEW_RECEIPT_SHA256: value.previewReceiptSha256,
    YUNE_WEB06_GREEN_PREVIEW_CANARY_STATUS: value.previewCanaryStatusPath,
    YUNE_WEB06_PRODUCTION_APPROVAL_FILE: value.approvalPath,
    YUNE_WEB06_PRODUCTION_APPROVAL_LEDGER_ROOT: value.approvalLedgerRoot,
    YUNE_WEB06_PRODUCTION_APPROVAL_SHA256: value.approvalSha256,
    YUNE_WEB06_PRODUCTION_APPROVAL_RECORD: "user-approved-web06-production",
    YUNE_WEB06_PRODUCTION_CONFIRMATION: "PROMOTE_EXACT_PREVIEW_BYTES",
    YUNE_WEB06_EVIDENCE_ROOT: evidenceRoot,
    CLOUDFLARE_PRODUCTION_ACCOUNT_ID: "production-account-id",
    CLOUDFLARE_PRODUCTION_API_TOKEN: "production-secret-token",
    CLOUDFLARE_ACCOUNT_ID: "preview-account-id",
    CLOUDFLARE_API_TOKEN: "preview-secret-token",
    CLOUDFLARE_API_KEY: "alternate-secret-key",
    CF_API_TOKEN: "alternate-secret-token",
  };
}

test("production promotion reuses the exact green preview bytes once and never builds", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-production-promotion-"));
  try {
    const value = await fixture(root);
    const commands = [];
    let deployCalls = 0;
    const deploymentInventoryPages = [];
    const served = new Map([
      ["build-info.json", value.buildInfoBytes],
      ["public-artifact-manifest.json", value.manifestBytes],
      ...value.files,
    ]);
    const fetchImplementation = async (url, options) => {
      const parsed = new URL(url);
      if (parsed.hostname === "api.cloudflare.com") {
        assert.equal(
          options.headers.Authorization,
          "Bearer production-secret-token",
        );
        if (parsed.pathname.endsWith("/deployments")) {
          const page = Number(parsed.searchParams.get("page"));
          deploymentInventoryPages.push(page);
          const beforeMutation = deployCalls === 0;
          const result = beforeMutation
            ? [{ id: page === 1 ? "old-deployment" : "older-page-two-deployment" }]
            : [{
                id: "new-production-deployment",
                url: "https://new-production.yune-web.pages.dev",
                environment: "production",
                latest_stage: { status: "success" },
                deployment_trigger: {
                  type: "ad_hoc",
                  metadata: {
                    branch: "main",
                    commit_hash: sourceCommit,
                    commit_dirty: false,
                  },
                },
              }];
          return new Response(JSON.stringify({
            success: true,
            result,
            result_info: {
              page,
              total_pages: beforeMutation ? 2 : 1,
            },
          }));
        }
        return new Response(JSON.stringify({
          success: true,
          result: {
            production_branch: "main",
            source: {
              config: {
                production_deployments_enabled: false,
                preview_deployment_setting: "none",
              },
            },
          },
        }));
      }
      const relative = decodeURIComponent(parsed.pathname.replace(/^\//, ""));
      const bytes = served.get(relative);
      return bytes === undefined
        ? new Response("missing\n", { status: 404 })
        : new Response(bytes);
    };
    const runCommand = async (command, arguments_, options) => {
      commands.push({ command, arguments_, options });
      if (command === "/usr/bin/git") {
        assert.equal(
          Object.keys(options.env).some(
            (name) => name.startsWith("CLOUDFLARE_") || name.startsWith("CF_"),
          ),
          false,
        );
        const key = arguments_.join(" ");
        if (key === "rev-parse HEAD") return { stdout: `${sourceCommit}\n`, stderr: "" };
        if (key === "rev-parse HEAD^{tree}") return { stdout: `${sourceTree}\n`, stderr: "" };
        if (key === "status --porcelain --untracked-files=all") return { stdout: "", stderr: "" };
        if (
          key ===
          "ls-remote --exit-code https://github.com/CanCLID/yune.git refs/heads/main"
        ) {
          return { stdout: `${sourceCommit}\trefs/heads/main\n`, stderr: "" };
        }
        throw new Error(`unexpected git command: ${key}`);
      }
      assert.equal(command, "/trusted/node");
      deployCalls += 1;
      assert.deepEqual(arguments_.slice(0, 3), [
        "/trusted/wrangler.js",
        "pages",
        "deploy",
      ]);
      assert.equal(arguments_[arguments_.indexOf("--branch") + 1], "main");
      assert.equal(
        options.env.CLOUDFLARE_API_TOKEN,
        "production-secret-token",
      );
      assert.equal(
        options.env.CLOUDFLARE_ACCOUNT_ID,
        "production-account-id",
      );
      assert.equal(options.env.CLOUDFLARE_PRODUCTION_API_TOKEN, undefined);
      assert.equal(options.env.CLOUDFLARE_PRODUCTION_ACCOUNT_ID, undefined);
      assert.equal(options.env.CLOUDFLARE_API_KEY, undefined);
      assert.equal(options.env.CF_API_TOKEN, undefined);
      return { stdout: "uploaded\n", stderr: "" };
    };
    const result = await main(
      environment(value, path.join(root, "production-evidence")),
      {
        ...fakeWranglerDependencies(),
        readAndValidateSuiteAttestation: fixtureSuiteReader(value),
        fetch: fetchImplementation,
        runCommand,
        delay: async () => assert.fail("no retry delay expected"),
      },
    );
    assert.equal(result.deploymentId, "new-production-deployment");
    assert.equal(result.productionUrl, "https://yune-web.pages.dev/");
    assert.equal(deployCalls, 1);
    assert.deepEqual(deploymentInventoryPages, [1, 2, 1]);
    assert(!JSON.stringify(commands).includes("build-public-release"));
    assert(!JSON.stringify(commands).toLowerCase().includes("playwright"));
    const status = JSON.parse(
      await readFile(
        path.join(root, "production-evidence", "web06-production-promotion-status.json"),
        "utf8",
      ),
    );
    assert.equal(status.operation, "promote-exact-preview-bytes-no-build");
    assert.equal(status.buildInvoked, false);
    assert.equal(status.productionMutationStarted, true);
    assert.deepEqual(status.productionDeployment, {
      id: "new-production-deployment",
      url: "https://new-production.yune-web.pages.dev",
      environment: "production",
      latestStageStatus: "success",
      triggerType: "ad_hoc",
      branch: "main",
      commitHash: sourceCommit,
      commitDirty: false,
    });
    assert.equal(status.remoteManifestFileCount, value.files.size);
    assert.equal(status.milestoneDisposition, "PARTIAL");
    assert.equal(status.peerPackageAlignment, "DATA_CONFOUNDED");
    assert.equal(status.peerRatioStatus, "OMITTED");
    assert.equal(status.fullSuccessEligible, false);
    assert.deepEqual(status.wranglerInstallIdentity, wranglerInstallIdentity);
    assert.deepEqual(status.wranglerRuntimeIdentity, wranglerRuntimeIdentity);
    assert.match(status.approvalConsumptionSha256, /^[0-9a-f]{64}$/);
    const persistedEvidence = JSON.stringify(status);
    for (const secret of [
      "production-secret-token",
      "production-account-id",
      "preview-secret-token",
      "preview-account-id",
      "alternate-secret-key",
      "alternate-secret-token",
    ]) {
      assert.equal(persistedEvidence.includes(secret), false);
    }
    await readFile(`${value.approvalPath}.consumed`);

    await assert.rejects(
      main(environment(value, path.join(root, "second-production-evidence")), {
        ...fakeWranglerDependencies(),
        readAndValidateSuiteAttestation: fixtureSuiteReader(value),
        fetch: fetchImplementation,
        runCommand,
        delay: async () => {},
      }),
      /EEXIST|exist/i,
    );
    assert.equal(deployCalls, 1);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("production resolution rejects multiple matching new deployment identities", async () => {
  const contract = {
    accountId: "production-account-id",
    apiToken: "production-secret-token",
    projectName: "yune-web",
    sourceCommit,
  };
  const exact = (id) => ({
    id,
    url: `https://${id}.yune-web.pages.dev`,
    environment: "production",
    latest_stage: { status: "success" },
    deployment_trigger: {
      type: "ad_hoc",
      metadata: {
        branch: "main",
        commit_hash: sourceCommit,
        commit_dirty: false,
      },
    },
  });
  await assert.rejects(
    resolveNewProductionDeployment(
      contract,
      new Set(),
      async (value) => {
        assert.equal(new URL(value).searchParams.get("page"), "1");
        return new Response(JSON.stringify({
          success: true,
          result: [exact("first"), exact("second")],
          result_info: { page: 1, total_pages: 1 },
        }));
      },
      async () => assert.fail("ambiguous identity must fail without retry"),
    ),
    /multiple matching WEB06 production deployments/,
  );
});

test("production entrypoint has no default approval record or confirmation", () => {
  assert.throws(
    () => resolveProductionContract({
      YUNE_WEB06_EXPECTED_SOURCE_COMMIT: sourceCommit,
      YUNE_WEB06_EXPECTED_SOURCE_TREE: sourceTree,
      YUNE_WEB06_CERTIFIED_ARCHIVE: "/tmp/final.tar.gz",
      YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256: "3".repeat(64),
      YUNE_WEB06_GREEN_PREVIEW_RECEIPT_SHA256: "4".repeat(64),
      YUNE_WEB06_PRODUCTION_APPROVAL_SHA256: "5".repeat(64),
      YUNE_WEB06_SELECTED_BRANCH: "A",
      YUNE_WEB06_DISPOSITION: "PRODUCTION_REDUCTION",
    }),
    /PRODUCTION_CONFIRMATION|required/,
  );
});

test("production source proof uses canonical main with neutral Git configuration", async () => {
  const contract = { sourceCommit, sourceTree };
  const commands = [];
  const execute = async (command, arguments_, options) => {
    commands.push({ command, arguments_, options });
    assert.equal(command, "/usr/bin/git");
    assert.equal(options.env.GIT_CONFIG_GLOBAL, "/dev/null");
    assert.equal(options.env.GIT_CONFIG_NOSYSTEM, "1");
    assert.equal(options.env.GIT_CONFIG_COUNT, undefined);
    assert.equal(options.env.GIT_DIR, undefined);
    assert.equal(options.env.GIT_WORK_TREE, undefined);
    assert.equal(options.env.CLOUDFLARE_API_TOKEN, undefined);
    const key = arguments_.join(" ");
    if (key === "rev-parse HEAD") {
      assert.notEqual(options.cwd, "/");
      return { stdout: `${sourceCommit}\n`, stderr: "" };
    }
    if (key === "rev-parse HEAD^{tree}") {
      return { stdout: `${sourceTree}\n`, stderr: "" };
    }
    if (key === "status --porcelain --untracked-files=all") {
      return { stdout: "", stderr: "" };
    }
    if (
      key ===
      "ls-remote --exit-code https://github.com/CanCLID/yune.git refs/heads/main"
    ) {
      assert.equal(options.cwd, "/");
      return {
        stdout: `${sourceCommit}\trefs/heads/main\n`,
        stderr: "",
      };
    }
    throw new Error(`unexpected Git command: ${key}`);
  };
  await assert.doesNotReject(proveExactCurrentSource(contract, execute));
  assert.equal(
    commands.some((entry) => entry.arguments_.includes("origin")),
    false,
  );

  await assert.rejects(
    proveExactCurrentSource(contract, async (command, arguments_, options) => {
      const key = arguments_.join(" ");
      if (key === "rev-parse HEAD") return { stdout: `${sourceCommit}\n` };
      if (key === "rev-parse HEAD^{tree}") return { stdout: `${sourceTree}\n` };
      if (key === "status --porcelain --untracked-files=all") {
        return { stdout: "" };
      }
      assert.equal(command, "/usr/bin/git");
      assert.equal(options.env.GIT_CONFIG_COUNT, undefined);
      return {
        stdout: `${"f".repeat(40)}\trefs/heads/main\n`,
      };
    }),
    /canonical main equality/,
  );
});

test("production approval explicitly accepts Partial DATA_CONFOUNDED/no-ratio", () => {
  const expected = {
    sourceCommit,
    sourceTree,
    archiveSha256: "3".repeat(64),
    previewReceiptSha256: "4".repeat(64),
    approvalRecord: "user-approved-web06-production",
  };
  const payload = {
    version: "web06-production-approval-v1",
    ...expected,
    confirmation: "PROMOTE_EXACT_PREVIEW_BYTES",
    acceptedMilestoneDisposition: "PARTIAL",
    acceptedLimitations: [
      "PEER_DATA_CONFOUNDED",
      "PEER_RATIO_OMITTED",
    ],
    previewFullSuccessEligible: false,
    nonce: "5".repeat(64),
    approvedAt: "2026-07-22T03:30:00Z",
  };
  assert.doesNotThrow(() => validateProductionApprovalPayload(payload, expected));
  for (const mutate of [
    (value) => { value.acceptedMilestoneDisposition = "FULL"; },
    (value) => { value.acceptedLimitations = ["PEER_DATA_CONFOUNDED"]; },
    (value) => { value.previewFullSuccessEligible = true; },
  ]) {
    const changed = structuredClone(payload);
    mutate(changed);
    assert.throws(
      () => validateProductionApprovalPayload(changed, expected),
      /not bound to this exact promotion/,
    );
  }
});

test("preview credentials alone cannot authorize production", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-production-credentials-"));
  try {
    const value = await fixture(root);
    const env = environment(value, path.join(root, "production-evidence"));
    delete env.CLOUDFLARE_PRODUCTION_ACCOUNT_ID;
    delete env.CLOUDFLARE_PRODUCTION_API_TOKEN;
    await assert.rejects(
      main(env, {
        fetch: async () => assert.fail("network must not begin"),
        runCommand: async () => assert.fail("commands must not begin"),
      }),
      /CLOUDFLARE_PRODUCTION_ACCOUNT_ID is required/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("production failures redact every Cloudflare credential from errors and receipts", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-production-redaction-"));
  try {
    const value = await fixture(root);
    const evidenceRoot = path.join(root, "production-evidence");
    const env = environment(value, evidenceRoot);
    let reported;
    await assert.rejects(
      main(env, {
        ...fakeWranglerDependencies({
          preparePinnedWranglerRuntime: async () => {
            throw new Error(
              "failed production-secret-token preview-secret-token alternate-secret-key",
            );
          },
        }),
        readAndValidateSuiteAttestation: fixtureSuiteReader(value),
        runCommand: async (command, arguments_, options) => {
          if (command === "/usr/bin/git") {
            const key = arguments_.join(" ");
            if (key === "rev-parse HEAD") return { stdout: `${sourceCommit}\n`, stderr: "" };
            if (key === "rev-parse HEAD^{tree}") return { stdout: `${sourceTree}\n`, stderr: "" };
            if (key === "status --porcelain --untracked-files=all") return { stdout: "", stderr: "" };
            if (
              key ===
              "ls-remote --exit-code https://github.com/CanCLID/yune.git refs/heads/main"
            ) {
              return { stdout: `${sourceCommit}\trefs/heads/main\n`, stderr: "" };
            }
          }
          assert.fail(`unexpected command ${command}`);
        },
        fetch: async () => assert.fail("network must not begin"),
      }),
      (error) => {
        reported = error.message;
        return /failed \[REDACTED\] \[REDACTED\] \[REDACTED\]/.test(error.message);
      },
    );
    const failure = await readFile(
      path.join(evidenceRoot, "web06-production-promotion-failure.json"),
      "utf8",
    );
    for (const secret of [
      "production-secret-token",
      "production-account-id",
      "preview-secret-token",
      "preview-account-id",
      "alternate-secret-key",
      "alternate-secret-token",
    ]) {
      assert.equal(failure.includes(secret), false);
      assert.equal(reported.includes(secret), false);
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
