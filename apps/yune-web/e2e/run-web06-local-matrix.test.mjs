import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  artifactResponseGuard,
  prepareMatrixOutputPaths,
  proveExactMatrixRunnerSource,
  resolveMatrixContract,
} from "./run-web06-local-matrix.mjs";
import {
  WEB06_COLLECTOR_CONTRACT_SHA256,
} from "./web06-collector.mjs";
import {
  WEB06_BEHAVIOR_VERSION,
  WEB06_METRIC_VERSION,
  WEB06_SCENARIO_VERSION,
} from "./web06-suite-attestation.mjs";

const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
);

const role = (
  digit,
  selectedBranch,
  disposition = selectedBranch === "NONE"
    ? "DIAGNOSTIC"
    : "PRODUCTION_REDUCTION",
) => ({
  sourceCommit: digit.repeat(40),
  sourceTree: ((Number(digit) + 1) % 10).toString().repeat(40),
  sourceTreeState: "clean",
  archiveSha256: digit.repeat(64),
  artifactManifestSha256: ((Number(digit) + 1) % 10).toString().repeat(64),
  buildInfoSha256: ((Number(digit) + 2) % 10).toString().repeat(64),
  selectedBranch,
  disposition,
});

function identity(roles) {
  return JSON.stringify({
    version: "web06-target-identities-v1",
    metricContractVersion: WEB06_METRIC_VERSION,
    scenarioRegistryVersion: WEB06_SCENARIO_VERSION,
    behaviorPredicateVersion: WEB06_BEHAVIOR_VERSION,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    roles,
  });
}

test("OBSERVER launches simultaneous pinned PRODUCT and BASE origins", () => {
  const contract = resolveMatrixContract({
    YUNE_WEB06_EXPECTATION: "OBSERVER",
    YUNE_WEB06_SELECTED_BRANCH: "NONE",
    YUNE_WEB06_DISPOSITION: "DIAGNOSTIC",
    YUNE_WEB06_EVIDENCE_ROOT: "/tmp/web06-observer-test",
    YUNE_WEB06_RUN_ID: "observer-1",
    YUNE_WEB06_RUN_ENVIRONMENT_JSON: "{}",
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: identity({
      PRODUCT: role("1", "NONE"),
      BASE: role("2", "NONE"),
    }),
    YUNE_WEB06_PRODUCT_ARCHIVE: "/tmp/product.tar.gz",
    YUNE_WEB06_BASE_ARCHIVE: "/tmp/base.tar.gz",
  });
  assert.deepEqual(contract.targetOrder, ["PRODUCT", "BASE_MINIMAL", "BASE_FULL"]);
  assert.deepEqual(contract.scenarios, ["rapid-long-jyutping"]);
  assert.notEqual(contract.roles.PRODUCT.port, contract.roles.BASE.port);
});

test("BASELINE is exact BASE/full and cannot select a production branch", () => {
  const contract = resolveMatrixContract({
    YUNE_WEB06_EXPECTATION: "BASELINE",
    YUNE_WEB06_SELECTED_BRANCH: "NONE",
    YUNE_WEB06_DISPOSITION: "SOURCE_CURRENT_BASELINE",
    YUNE_WEB06_EVIDENCE_ROOT: "/tmp/web06-baseline-test",
    YUNE_WEB06_RUN_ID: "baseline-1",
    YUNE_WEB06_RUN_ENVIRONMENT_JSON: "{}",
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: identity({
      BASE: role("2", "NONE", "SOURCE_CURRENT_BASELINE"),
    }),
    YUNE_WEB06_BASE_ARCHIVE: "/tmp/base.tar.gz",
  });
  assert.deepEqual(contract.targetOrder, ["BASE_FULL"]);
  assert.equal(contract.scenarios.length, 14);
  assert.throws(
    () => resolveMatrixContract({
      YUNE_WEB06_EXPECTATION: "BASELINE",
      YUNE_WEB06_SELECTED_BRANCH: "A",
      YUNE_WEB06_DISPOSITION: "PRODUCTION_REDUCTION",
      YUNE_WEB06_EVIDENCE_ROOT: "/tmp/web06-bad",
      YUNE_WEB06_RUN_ID: "bad",
      YUNE_WEB06_RUN_ENVIRONMENT_JSON: "{}",
      YUNE_WEB06_IDENTITY_MANIFEST_JSON: identity({
        BASE: role("2", "NONE", "SOURCE_CURRENT_BASELINE"),
      }),
      YUNE_WEB06_BASE_ARCHIVE: "/tmp/base.tar.gz",
    }),
    /requires selected branch NONE/,
  );
});

test("FINAL branch B alone adds the extended scheduler scenario", () => {
  const branchB = resolveMatrixContract({
    YUNE_WEB06_EXPECTATION: "FINAL",
    YUNE_WEB06_SELECTED_BRANCH: "B",
    YUNE_WEB06_DISPOSITION: "PRODUCTION_REDUCTION",
    YUNE_WEB06_EVIDENCE_ROOT: "/tmp/web06-final-test",
    YUNE_WEB06_RUN_ID: "final-1",
    YUNE_WEB06_RUN_ENVIRONMENT_JSON: "{}",
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: identity({ FINAL: role("3", "B") }),
    YUNE_WEB06_FINAL_ARCHIVE: "/tmp/final.tar.gz",
  });
  assert.deepEqual(branchB.targetOrder, ["FINAL_FULL"]);
  assert.deepEqual(branchB.scenarios.slice(-2), [
    "extended-scheduler-barriers@jyut6ping3",
    "extended-scheduler-barriers@luna_pinyin",
  ]);
  assert.equal(branchB.scenarios.length, 16);
});

test("FINAL measured no-go uses NONE and cannot invent Branch D", () => {
  const noGo = resolveMatrixContract({
    YUNE_WEB06_EXPECTATION: "FINAL",
    YUNE_WEB06_SELECTED_BRANCH: "NONE",
    YUNE_WEB06_DISPOSITION: "MEASURED_NO_GO",
    YUNE_WEB06_EVIDENCE_ROOT: "/tmp/web06-final-no-go-test",
    YUNE_WEB06_RUN_ID: "final-no-go-1",
    YUNE_WEB06_RUN_ENVIRONMENT_JSON: "{}",
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: identity({
      FINAL: role("3", "NONE", "MEASURED_NO_GO"),
    }),
    YUNE_WEB06_FINAL_ARCHIVE: "/tmp/final.tar.gz",
  });
  assert.equal(noGo.selectedBranch, "NONE");
  assert.equal(noGo.disposition, "MEASURED_NO_GO");
  assert.throws(
    () => resolveMatrixContract({
      YUNE_WEB06_EXPECTATION: "FINAL",
      YUNE_WEB06_SELECTED_BRANCH: "D",
      YUNE_WEB06_DISPOSITION: "PRODUCTION_REDUCTION",
      YUNE_WEB06_EVIDENCE_ROOT: "/tmp/web06-final-d",
      YUNE_WEB06_RUN_ID: "final-d",
      YUNE_WEB06_RUN_ENVIRONMENT_JSON: "{}",
      YUNE_WEB06_IDENTITY_MANIFEST_JSON: identity({ FINAL: role("3", "D") }),
      YUNE_WEB06_FINAL_ARCHIVE: "/tmp/final.tar.gz",
    }),
    /A, B, C, or NONE/,
  );
});

test("matrix evidence output is create-new and outside the repository", async () => {
  const parent = await mkdtemp(path.join(tmpdir(), "web06-matrix-output-"));
  try {
    const evidenceRoot = path.join(parent, "binding-evidence");
    const contract = {
      evidenceRoot,
      playwrightOutputDir: path.join(evidenceRoot, "playwright"),
    };
    await prepareMatrixOutputPaths(contract, process.env);
    assert.equal(
      path.relative(contract.evidenceRoot, contract.rawEvidenceRoot),
      "raw",
    );
    assert.equal(
      path.relative(contract.evidenceRoot, contract.playwrightOutputDir),
      "playwright",
    );
    assert.equal(
      path.relative(contract.rawEvidenceRoot, contract.collectorOutputPath),
      "collector-output.json",
    );
    assert.equal(
      path.relative(contract.rawEvidenceRoot, contract.independentRecomputePath),
      "independent-recompute.json",
    );
    assert.equal(
      path.relative(contract.rawEvidenceRoot, contract.suiteAttestationPath),
      "suite-attestation.json",
    );
  } finally {
    await rm(parent, { recursive: true, force: true });
  }

  const repositoryEvidenceRoot = path.resolve(
    repositoryRoot,
    "target",
    `web06-forbidden-${process.pid}`,
  );
  await assert.rejects(
    prepareMatrixOutputPaths(
      {
        evidenceRoot: repositoryEvidenceRoot,
        playwrightOutputDir: path.join(repositoryEvidenceRoot, "playwright"),
      },
      process.env,
    ),
    /disjoint from Git worktree|outside the repository|must not be inside/i,
  );
});

test("matrix runner checkout is clean and exactly pinned to its owning role", async () => {
  const makeContract = (expectation) => ({
    expectation,
    roles: {
      [expectation === "FINAL" ? "FINAL" : "BASE"]: {
        sourceCommit: "1".repeat(40),
        sourceTree: "2".repeat(40),
      },
    },
  });
  const execute = ({
    head = "1".repeat(40),
    tree = "2".repeat(40),
    status = "",
  } = {}) => async (command, arguments_, options) => {
    assert.equal(command, "git");
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
    if (key === "rev-parse HEAD") return { stdout: `${head}\n`, stderr: "" };
    if (key === "rev-parse HEAD^{tree}") return { stdout: `${tree}\n`, stderr: "" };
    if (key === "status --porcelain --untracked-files=all") {
      return { stdout: status, stderr: "" };
    }
    throw new Error(`unexpected git command: ${key}`);
  };

  for (const expectation of ["OBSERVER", "BASELINE", "FINAL"]) {
    const proof = await proveExactMatrixRunnerSource(
      makeContract(expectation),
      { CLOUDFLARE_API_TOKEN: "must-not-leak" },
      execute(),
    );
    assert.equal(proof.roleName, expectation === "FINAL" ? "FINAL" : "BASE");
  }
  await assert.rejects(
    proveExactMatrixRunnerSource(
      makeContract("BASELINE"),
      {},
      execute({ status: "?? untracked-parser.mjs\n" }),
    ),
    /clean exact BASE HEAD\/tree/,
  );
  await assert.rejects(
    proveExactMatrixRunnerSource(
      makeContract("FINAL"),
      {},
      execute({ head: "3".repeat(40) }),
    ),
    /clean exact FINAL HEAD\/tree/,
  );
  await assert.rejects(
    proveExactMatrixRunnerSource(
      makeContract("FINAL"),
      {},
      execute({ tree: "4".repeat(40) }),
    ),
    /clean exact FINAL HEAD\/tree/,
  );
});

test("each target carries the exact complete response inventory", () => {
  const buildInfoBytes = Buffer.from("build-info\n");
  const manifestBytes = Buffer.from("manifest\n");
  const memberBytes = Buffer.from("index\n");
  const guard = artifactResponseGuard({
    buildInfoBytes,
    manifestBytes,
    manifest: {
      files: [{
        path: "index.html",
        bytes: memberBytes.byteLength,
        sha256: digest(memberBytes),
      }],
    },
  });
  assert.deepEqual(guard, {
    version: "web06-artifact-response-guard-v1",
    rootDocumentPath: "index.html",
    entries: [
      {
        path: "build-info.json",
        bytes: buildInfoBytes.byteLength,
        sha256: digest(buildInfoBytes),
      },
      {
        path: "public-artifact-manifest.json",
        bytes: manifestBytes.byteLength,
        sha256: digest(manifestBytes),
      },
      {
        path: "index.html",
        bytes: memberBytes.byteLength,
        sha256: digest(memberBytes),
      },
    ],
  });
});
