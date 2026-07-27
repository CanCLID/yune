import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdir,
  mkdtemp,
  readFile,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { promisify } from "node:util";

import {
  assertNoPriorPreviewDeployment,
  consumePreviewAuthorization,
  main,
  proveExactPreviewSource,
  resolveNewPreviewDeployment,
  resolveCertifiedPreviewContract,
} from "./run-web06-certified-preview.mjs";
import {
  validateGreenPreviewReceipt,
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
  return {
    archivePath,
    archiveSha256,
    buildInfoBytes,
    files,
    finalEvidenceRoot,
    finalSuitePath,
    identity,
    manifestBytes,
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
    YUNE_WEB06_EVIDENCE_ROOT: evidenceRoot,
    YUNE_WEB06_RUN_ENVIRONMENT_JSON: "{}",
    CLOUDFLARE_ACCOUNT_ID: "account-id",
    CLOUDFLARE_API_TOKEN: "secret-token",
    CLOUDFLARE_API_KEY: "alternate-secret-key",
    CF_API_TOKEN: "alternate-secret-token",
    WRANGLER_AUTH_DOMAIN: "alternate-secret-domain",
  };
}

test("certified preview uses exact sealed bytes, pinned Wrangler, and no build or production", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-certified-preview-"));
  const originalFetch = globalThis.fetch;
  try {
    const value = await fixture(root);
    const evidenceRoot = path.join(root, "preview-evidence");
    const calls = [];
    let deploymentInventoryCalls = 0;
    const served = new Map([
      ["build-info.json", value.buildInfoBytes],
      ["public-artifact-manifest.json", value.manifestBytes],
      ...value.files,
    ]);
    globalThis.fetch = async (url) => {
      const relative = decodeURIComponent(new URL(url).pathname.replace(/^\//, ""));
      const bytes = served.get(relative);
      return bytes === undefined
        ? new Response("missing\n", { status: 404 })
        : new Response(bytes);
    };
    const env = environment(value, evidenceRoot);
    const result = await main(env, {
      ...fakeWranglerDependencies(),
      readAndValidateSuiteAttestation: fixtureSuiteReader(value),
      fetch: async (url, options) => {
        const parsed = new URL(url);
        if (parsed.hostname !== "api.cloudflare.com") {
          return globalThis.fetch(url, options);
        }
        if (parsed.pathname.endsWith("/deployments")) {
          deploymentInventoryCalls += 1;
          return new Response(JSON.stringify({
            success: true,
            result: deploymentInventoryCalls === 1
              ? []
              : [{
                  id: "new-preview-deployment",
                  url: "https://preview-id.yune-web.pages.dev",
                  environment: "preview",
                  latest_stage: { status: "success" },
                  deployment_trigger: {
                    type: "ad_hoc",
                    metadata: {
                      branch: `web06-preview-${sourceCommit.slice(0, 12)}`,
                      commit_hash: sourceCommit,
                      commit_dirty: false,
                    },
                  },
                }],
            result_info: { page: 1, total_pages: 1 },
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
      },
      runCommand: async (command, arguments_, options) => {
        calls.push({ command, arguments_, options });
        if (command === "/usr/bin/git") {
          assert.equal(
            Object.keys(options.env).some(
              (name) =>
                name.startsWith("CLOUDFLARE_") ||
                name.startsWith("CF_") ||
                name.startsWith("WRANGLER_"),
            ),
            false,
          );
          const key = arguments_.join(" ");
          if (key === "rev-parse HEAD") return { stdout: `${sourceCommit}\n`, stderr: "" };
          if (key === "rev-parse HEAD^{tree}") return { stdout: `${sourceTree}\n`, stderr: "" };
          if (key === "status --porcelain --untracked-files=all") return { stdout: "", stderr: "" };
          throw new Error(`unexpected git command: ${key}`);
        }
        if (command === "bash") {
          assert.equal(
            Object.keys(options.env).some(
              (name) =>
                name.startsWith("CLOUDFLARE_") ||
                name.startsWith("CF_") ||
                name.startsWith("WRANGLER_"),
            ),
            false,
          );
          const distRoot = options.env.YUNE_WEB06_CERTIFIED_DIST_ROOT;
          assert.equal(
            await readFile(path.join(distRoot, "schema", "rare.bin"), "utf8"),
            "rare exact bytes\n",
          );
          const receipt = {
            version: "web06-local-release-certification-v1",
            operation: "local-no-build-certification",
            sourceCommit,
            sourceTree,
            archiveSha256: value.archiveSha256,
            artifactManifestSha256: digest(value.manifestBytes),
            finalSuiteAttestationSha256: digest(await readFile(value.finalSuitePath)),
            web03UnchangedStatus: "passed",
            defaultMinimalCompatibilityStatus: "passed",
            selectorPolicy: "omitted",
            buildInvoked: false,
            status: "passed",
          };
          await writeFile(
            arguments_[arguments_.indexOf("--receipt") + 1],
            `${JSON.stringify(receipt)}\n`,
            { flag: "wx" },
          );
          return { stdout: "", stderr: "" };
        }
        assert.equal(command, "/trusted/node");
        assert.deepEqual(arguments_.slice(0, 3), [
          "/trusted/wrangler.js",
          "pages",
          "deploy",
        ]);
        assert(!arguments_.includes("main"));
        assert(!arguments_.includes("production"));
        assert.equal(options.env.CLOUDFLARE_API_TOKEN, "secret-token");
        assert.equal(options.env.CLOUDFLARE_ACCOUNT_ID, "account-id");
        assert.equal(options.env.CLOUDFLARE_API_KEY, undefined);
        assert.equal(options.env.CF_API_TOKEN, undefined);
        assert.equal(options.env.WRANGLER_AUTH_DOMAIN, undefined);
        return {
          stdout: "Deployment complete! Take a peek over at https://preview-id.yune-web.pages.dev\n",
          stderr: "",
        };
      },
      runPublicGate: async (canaryEnvironment, arguments_) => {
        assert.equal(canaryEnvironment.YUNE_WEB_APP_URL, "https://preview-id.yune-web.pages.dev/");
        assert.equal(
          Object.keys(canaryEnvironment).some(
            (name) =>
              name.startsWith("CLOUDFLARE_") ||
              name.startsWith("CF_") ||
              name.startsWith("WRANGLER_"),
          ),
          false,
        );
        assert.deepEqual(arguments_, ["--scope=preview-canary"]);
        await mkdir(canaryEnvironment.YUNE_WEB06_EVIDENCE_ROOT, {
          recursive: true,
        });
        await writeFile(
          path.join(
            canaryEnvironment.YUNE_WEB06_EVIDENCE_ROOT,
            "web06-public-gate-status.json",
          ),
          `${JSON.stringify({
            version: "web06-public-gate-status-v1",
            sourceCommit,
            sourceTree,
            archiveSha256: value.archiveSha256,
            artifactManifestSha256: digest(value.manifestBytes),
            finalSuiteAttestationSha256: digest(
              await readFile(value.finalSuitePath),
            ),
            previewSuiteAttestationSha256: "9".repeat(64),
            disposition: "PRODUCTION_REDUCTION",
            scope: "preview-canary",
            appUrl: "https://preview-id.yune-web.pages.dev/",
            canaryStarted: true,
            status: "passed",
          })}\n`,
          { flag: "wx" },
        );
      },
    });
    assert.equal(result.status, "passed");
    assert.equal(result.previewUrl, "https://preview-id.yune-web.pages.dev/");
    assert.equal(calls.filter((call) => call.command === "bash").length, 1);
    assert.equal(calls.filter((call) => call.command === "/trusted/node").length, 1);
    assert.equal(calls.filter((call) => call.command === "npx").length, 0);
    assert(!JSON.stringify(calls).includes("build-public-release"));
    const receipt = JSON.parse(
      await readFile(path.join(evidenceRoot, "web06-certified-preview-status.json"), "utf8"),
    );
    assert.equal(receipt.productionPromotionAttempted, false);
    assert.equal(receipt.previewMutationStarted, true);
    assert.deepEqual(receipt.previewDeployment, {
      id: "new-preview-deployment",
      url: "https://preview-id.yune-web.pages.dev/",
      environment: "preview",
      latestStageStatus: "success",
      triggerType: "ad_hoc",
      branch: `web06-preview-${sourceCommit.slice(0, 12)}`,
      commitHash: sourceCommit,
      commitDirty: false,
    });
    assert.match(
      receipt.previewAuthorizationConsumptionSha256,
      /^[0-9a-f]{64}$/,
    );
    assert.equal(receipt.remoteManifestFileCount, 6);
    assert.match(receipt.localCertificationReceiptSha256, /^[0-9a-f]{64}$/);
    assert.match(receipt.previewCanaryStatusSha256, /^[0-9a-f]{64}$/);
    assert.equal(receipt.previewSuiteAttestationSha256, "9".repeat(64));
    assert.deepEqual(receipt.wranglerInstallIdentity, wranglerInstallIdentity);
    assert.deepEqual(receipt.wranglerRuntimeIdentity, wranglerRuntimeIdentity);
    const finalSuiteAttestationSha256 = digest(
      await readFile(value.finalSuitePath),
    );
    assert.doesNotThrow(() =>
      validateGreenPreviewReceipt(receipt, {
        sourceCommit,
        sourceTree,
        archiveSha256: value.archiveSha256,
        artifactManifestSha256: digest(value.manifestBytes),
        finalSuiteAttestationSha256,
      }),
    );
  } finally {
    globalThis.fetch = originalFetch;
    await rm(root, { recursive: true, force: true });
  }
});

test("measured no-go cannot enter the preview command", () => {
  assert.throws(
    () => resolveCertifiedPreviewContract({
      YUNE_WEB06_EXPECTED_SOURCE_COMMIT: sourceCommit,
      YUNE_WEB06_EXPECTED_SOURCE_TREE: sourceTree,
      YUNE_WEB06_CERTIFIED_ARCHIVE: "/tmp/final.tar.gz",
      YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256: "3".repeat(64),
      YUNE_WEB06_FINAL_SUITE_ATTESTATION: "/tmp/final-suite.json",
      YUNE_WEB06_FINAL_SUITE_EVIDENCE_ROOT: "/tmp/final-evidence",
      YUNE_WEB06_IDENTITY_MANIFEST_JSON: JSON.stringify({}),
      YUNE_WEB06_SELECTED_BRANCH: "NONE",
      YUNE_WEB06_DISPOSITION: "MEASURED_NO_GO",
      YUNE_WEB06_EVIDENCE_ROOT: "/tmp/preview-evidence",
      CLOUDFLARE_ACCOUNT_ID: "account",
      CLOUDFLARE_API_TOKEN: "token",
    }),
    /branch A, B, or C|measured no-go/,
  );
});

test("preview failures redact every Cloudflare credential from errors and receipts", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-preview-redaction-"));
  try {
    const value = await fixture(root);
    const evidenceRoot = path.join(root, "preview-evidence");
    let reported;
    await assert.rejects(
      main(environment(value, evidenceRoot), {
        ...fakeWranglerDependencies({
          preparePinnedWranglerRuntime: async () => {
            throw new Error(
              "failed secret-token alternate-secret-key alternate-secret-token",
            );
          },
        }),
        readAndValidateSuiteAttestation: fixtureSuiteReader(value),
        fetch: async () => new Response(JSON.stringify({
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
        })),
        runCommand: async (command, arguments_, options) => {
          if (command === "/usr/bin/git") {
            const key = arguments_.join(" ");
            if (key === "rev-parse HEAD") return { stdout: `${sourceCommit}\n`, stderr: "" };
            if (key === "rev-parse HEAD^{tree}") return { stdout: `${sourceTree}\n`, stderr: "" };
            if (key === "status --porcelain --untracked-files=all") return { stdout: "", stderr: "" };
            throw new Error(`unexpected git command: ${key}`);
          }
          if (command === "bash") {
            const receipt = {
              version: "web06-local-release-certification-v1",
              operation: "local-no-build-certification",
              sourceCommit,
              sourceTree,
              archiveSha256: value.archiveSha256,
              artifactManifestSha256: digest(value.manifestBytes),
              finalSuiteAttestationSha256: digest(
                await readFile(value.finalSuitePath),
              ),
              web03UnchangedStatus: "passed",
              defaultMinimalCompatibilityStatus: "passed",
              selectorPolicy: "omitted",
              buildInvoked: false,
              status: "passed",
            };
            await writeFile(
              arguments_[arguments_.indexOf("--receipt") + 1],
              `${JSON.stringify(receipt)}\n`,
              { flag: "wx" },
            );
            return { stdout: "", stderr: "" };
          }
          assert.fail(`unexpected command ${command}`);
        },
      }),
      (error) => {
        reported = error.message;
        return /failed \[REDACTED\] \[REDACTED\] \[REDACTED\]/.test(error.message);
      },
    );
    const failure = await readFile(
      path.join(evidenceRoot, "web06-certified-preview-failure.json"),
      "utf8",
    );
    for (const secret of [
      "secret-token",
      "account-id",
      "alternate-secret-key",
      "alternate-secret-token",
      "alternate-secret-domain",
    ]) {
      assert.equal(failure.includes(secret), false);
      assert.equal(reported.includes(secret), false);
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("preview source proof rejects dirty, wrong-commit, and wrong-tree runners", async () => {
  const contract = { sourceCommit, sourceTree };
  const execute = ({
    head = sourceCommit,
    tree = sourceTree,
    status = "",
  } = {}) => async (_command, arguments_) => {
    const key = arguments_.join(" ");
    if (key === "rev-parse HEAD") return { stdout: `${head}\n` };
    if (key === "rev-parse HEAD^{tree}") return { stdout: `${tree}\n` };
    if (key === "status --porcelain --untracked-files=all") {
      return { stdout: status };
    }
    throw new Error(`unexpected git command: ${key}`);
  };
  await assert.doesNotReject(proveExactPreviewSource(contract, execute()));
  for (const mutation of [
    { status: "?? untracked-release.mjs\n" },
    { head: "3".repeat(40) },
    { tree: "4".repeat(40) },
  ]) {
    await assert.rejects(
      proveExactPreviewSource(contract, execute(mutation)),
      /clean exact candidate HEAD\/tree/,
    );
  }
});

test("an existing source-and-branch preview consumes the only authorized attempt", async () => {
  const contract = {
    accountId: "account-id",
    apiToken: "secret-token",
    projectName: "yune-web",
    sourceCommit,
  };
  await assert.rejects(
    assertNoPriorPreviewDeployment(
      contract,
      `web06-preview-${sourceCommit.slice(0, 12)}`,
      async () => new Response(JSON.stringify({
        success: true,
        result: [{
          id: "prior-preview",
          environment: "preview",
          deployment_trigger: {
            metadata: {
              branch: `web06-preview-${sourceCommit.slice(0, 12)}`,
              commit_hash: sourceCommit,
            },
          },
        }],
        result_info: { page: 1, total_pages: 1 },
      })),
    ),
    /already attempted/,
  );
});

test("prior-preview detection searches every reported deployment page", async () => {
  const contract = {
    accountId: "account-id",
    apiToken: "secret-token",
    projectName: "yune-web",
    sourceCommit,
  };
  const requestedPages = [];
  await assert.rejects(
    assertNoPriorPreviewDeployment(
      contract,
      `web06-preview-${sourceCommit.slice(0, 12)}`,
      async (value) => {
        const page = Number(new URL(value).searchParams.get("page"));
        requestedPages.push(page);
        return new Response(JSON.stringify({
          success: true,
          result: page === 2
            ? [{
                id: "older-prior-preview",
                environment: "preview",
                deployment_trigger: {
                  metadata: {
                    branch: `web06-preview-${sourceCommit.slice(0, 12)}`,
                    commit_hash: sourceCommit,
                  },
                },
              }]
            : [],
          result_info: { page, total_pages: 2 },
        }));
      },
    ),
    /already attempted/,
  );
  assert.deepEqual(requestedPages, [1, 2]);
});

test("post-upload resolution requires the unique exact observed preview identity", async () => {
  const contract = {
    accountId: "account-id",
    apiToken: "secret-token",
    projectName: "yune-web",
    sourceCommit,
  };
  const branch = `web06-preview-${sourceCommit.slice(0, 12)}`;
  const previewUrl = "https://observed.yune-web.pages.dev/";
  let calls = 0;
  const selected = await resolveNewPreviewDeployment(
    contract,
    branch,
    previewUrl,
    new Set(["old"]),
    async () => {
      calls += 1;
      const row = {
        id: "new-preview",
        url: "https://observed.yune-web.pages.dev",
        environment: "preview",
        latest_stage: { status: "success" },
        deployment_trigger: {
          type: "ad_hoc",
          metadata: {
            branch,
            commit_hash: sourceCommit,
            commit_dirty: calls === 1,
          },
        },
      };
      return new Response(JSON.stringify({
        success: true,
        result: [row],
        result_info: { page: 1, total_pages: 1 },
      }));
    },
    async () => {},
  );
  assert.equal(calls, 2);
  assert.equal(selected.id, "new-preview");
});

test("preview authorization is an atomic one-shot adjacent to the sealed archive", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-preview-one-shot-"));
  try {
    const value = await fixture(root);
    const contract = {
      archivePath: value.archivePath,
      archiveSha256: value.archiveSha256,
      sourceCommit,
      sourceTree,
    };
    const branch = `web06-preview-${sourceCommit.slice(0, 12)}`;
    const first = await consumePreviewAuthorization(contract, branch);
    assert.match(first.sha256, /^[0-9a-f]{64}$/);
    const receipt = JSON.parse(await readFile(first.path, "utf8"));
    assert.equal(receipt.archiveSha256, value.archiveSha256);
    assert.equal(receipt.previewBranch, branch);
    await assert.rejects(
      consumePreviewAuthorization(contract, branch),
      { code: "EEXIST" },
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("a failed post-deploy canary cannot create a second preview attempt", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-preview-failed-canary-"));
  try {
    const value = await fixture(root);
    const served = new Map([
      ["build-info.json", value.buildInfoBytes],
      ["public-artifact-manifest.json", value.manifestBytes],
      ...value.files,
    ]);
    let deployCalls = 0;
    let deploymentInventoryCalls = 0;
    const dependencies = {
      ...fakeWranglerDependencies(),
      readAndValidateSuiteAttestation: fixtureSuiteReader(value),
      fetch: async (url) => {
        const parsed = new URL(url);
        if (parsed.hostname === "api.cloudflare.com") {
          if (parsed.pathname.endsWith("/deployments")) {
            deploymentInventoryCalls += 1;
            return new Response(JSON.stringify({
              success: true,
              result: deploymentInventoryCalls === 1
                ? []
                : [{
                    id: "failed-canary-preview",
                    url: "https://failed-canary.yune-web.pages.dev",
                    environment: "preview",
                    latest_stage: { status: "success" },
                    deployment_trigger: {
                      type: "ad_hoc",
                      metadata: {
                        branch: `web06-preview-${sourceCommit.slice(0, 12)}`,
                        commit_hash: sourceCommit,
                        commit_dirty: false,
                      },
                    },
                  }],
              result_info: { page: 1, total_pages: 1 },
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
        return new Response(served.get(relative) ?? "missing\n", {
          status: served.has(relative) ? 200 : 404,
        });
      },
      runCommand: async (command, arguments_, options) => {
        if (command === "/usr/bin/git") {
          const key = arguments_.join(" ");
          if (key === "rev-parse HEAD") return { stdout: `${sourceCommit}\n`, stderr: "" };
          if (key === "rev-parse HEAD^{tree}") return { stdout: `${sourceTree}\n`, stderr: "" };
          if (key === "status --porcelain --untracked-files=all") {
            return { stdout: "", stderr: "" };
          }
          throw new Error(`unexpected git command: ${key}`);
        }
        if (command === "bash") {
          await writeFile(
            arguments_[arguments_.indexOf("--receipt") + 1],
            `${JSON.stringify({
              version: "web06-local-release-certification-v1",
              operation: "local-no-build-certification",
              sourceCommit,
              sourceTree,
              archiveSha256: value.archiveSha256,
              artifactManifestSha256: digest(value.manifestBytes),
              finalSuiteAttestationSha256: digest(
                await readFile(value.finalSuitePath),
              ),
              web03UnchangedStatus: "passed",
              defaultMinimalCompatibilityStatus: "passed",
              selectorPolicy: "omitted",
              buildInvoked: false,
              status: "passed",
            })}\n`,
            { flag: "wx" },
          );
          return { stdout: "", stderr: "" };
        }
        assert.equal(command, "/trusted/node");
        deployCalls += 1;
        return {
          stdout: "https://failed-canary.yune-web.pages.dev\n",
          stderr: "",
        };
      },
      runPublicGate: async () => {
        throw new Error("binding preview canary failed");
      },
    };
    const firstEvidence = path.join(root, "first-evidence");
    await assert.rejects(
      main(environment(value, firstEvidence), dependencies),
      /binding preview canary failed/,
    );
    assert.equal(deployCalls, 1);
    const firstFailure = JSON.parse(
      await readFile(
        path.join(firstEvidence, "web06-certified-preview-failure.json"),
        "utf8",
      ),
    );
    assert.equal(firstFailure.previewMutationStarted, true);
    assert.match(
      firstFailure.previewAuthorizationConsumptionSha256,
      /^[0-9a-f]{64}$/,
    );

    const secondEvidence = path.join(root, "second-evidence");
    await assert.rejects(
      main(environment(value, secondEvidence), dependencies),
      { code: "EEXIST" },
    );
    assert.equal(deployCalls, 1);
    const secondFailure = JSON.parse(
      await readFile(
        path.join(secondEvidence, "web06-certified-preview-failure.json"),
        "utf8",
      ),
    );
    assert.equal(secondFailure.previewMutationStarted, false);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
