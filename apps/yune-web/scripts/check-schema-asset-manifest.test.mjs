import assert from "node:assert/strict";
import { cp, mkdtemp, mkdir, readFile, readdir, rm, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  assertManifestMatchesTree,
  discoverGitTrackedPaths,
  runReachabilityAudit,
  schemaTreeEntryKind,
  validateReachabilityFormalism,
} from "./check-schema-asset-manifest.mjs";

const scriptRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptRoot, "../../..");

async function allFiles(root) {
  const pending = [root];
  const files = [];
  while (pending.length > 0) {
    const directory = pending.pop();
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const fullPath = path.join(directory, entry.name);
      if (entry.isDirectory()) pending.push(fullPath);
      else if (entry.isFile()) files.push(path.relative(root, fullPath).replaceAll(path.sep, "/"));
    }
  }
  return files.sort((left, right) => left.localeCompare(right));
}

function cloneValidation(options) {
  return {
    ...options,
    coverage: structuredClone(options.coverage),
    auditResult: structuredClone(options.auditResult),
    trackedPaths: [...options.trackedPaths],
    trackedSchemaPaths: [...options.trackedSchemaPaths],
    productManifestAssets: [...options.productManifestAssets],
    asOfDate: new Date(options.asOfDate),
  };
}

test("manifest reconciliation rejects an undeclared non-schema tree asset", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "yune-schema-manifest-"));
  try {
    await writeFile(path.join(root, "alpha.schema.yaml"), "schema:\n  schema_id: alpha\n");
    await mkdir(path.join(root, "nested"));
    await writeFile(path.join(root, "nested", "stray.bin"), "undeclared\n");

    await assert.rejects(
      assertManifestMatchesTree(["alpha.schema.yaml"], root),
      /nested\/stray\.bin/,
    );
    await assert.doesNotReject(
      assertManifestMatchesTree(["alpha.schema.yaml", "nested/stray.bin"], root),
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("schema tree traversal fails closed on symbolic links", () => {
  const symbolicLink = {
    isDirectory: () => false,
    isFile: () => false,
    isSymbolicLink: () => true,
  };
  assert.throws(
    () => schemaTreeEntryKind(symbolicLink, "nested/external-link"),
    /symbolic link.*nested\/external-link/,
  );
});

test("M60 production validator consumes the real Rust audit and fails closed", async t => {
  const root = await mkdtemp(path.join(os.tmpdir(), "yune-m60-governance-"));
  const fixtureRelative = "crates/yune-rime-api/tests/fixtures/m60-schema-reachability-audit";
  const productRoot = `${fixtureRelative}/shared`;
  const userRoot = `${fixtureRelative}/user`;
  const schemaAsset = `${productRoot}/audit.schema.yaml`;
  const mirrorSchemaAsset = `${productRoot}/audit_mirror.schema.yaml`;
  const unresolvedSchemaAsset = `${productRoot}/unresolved.schema.yaml`;
  try {
    await t.test("production tracked discovery rejects execution outside a Git worktree", async () => {
      await assert.rejects(discoverGitTrackedPaths(root, "*.schema.yaml"), /failed with exit/i);
    });
    await cp(
      path.join(repoRoot, ...fixtureRelative.split("/")),
      path.join(root, ...fixtureRelative.split("/")),
      { recursive: true },
    );
    await mkdir(path.join(root, "governance"), { recursive: true });
    await mkdir(path.join(root, "docs"), { recursive: true });
    await writeFile(path.join(root, "governance", "manifest.json"), "{}\n");
    await writeFile(path.join(root, "governance", "coverage.json"), "{}\n");
    await writeFile(path.join(root, "docs", "decisions.md"), "**D-99 / SYNTHETIC-OPTOUT**\n");
    await writeFile(path.join(root, "docs", "owner-spec.md"), "Synthetic owner specification.\n");

    const auditResult = await runReachabilityAudit({
      root,
      cargoRoot: repoRoot,
      productRoot,
      sharedDataRoot: productRoot,
      userDataRoot: userRoot,
      schemaAssets: [schemaAsset, mirrorSchemaAsset],
    });
    assert.equal(auditResult.version, "m60-reachability-v1");
    assert.equal(auditResult.tuples.length, 5);
    assert.ok(auditResult.tuples.some(tuple => tuple.component === "script_translator@foo" && tuple.effective === false));
    assert.ok(
      auditResult.tuples.some(
        tuple => tuple.component === "table_translator@bar" && tuple.runtimeArm === "upstream-table-lazy-bounded",
      ),
    );
    assert.ok(auditResult.tuples.some(tuple => tuple.component === "r10n_translator@baz"));
    assert.ok(auditResult.tuples.some(tuple => tuple.sourceTrace.length > 1), "fixture must exercise multiple sources");
    const traceKinds = auditResult.tuples.flatMap(tuple => tuple.sourceTrace.map(trace => trace.kind));
    assert.ok(traceKinds.some(kind => kind.includes("include")), "fixture must exercise include resolution");
    assert.ok(traceKinds.some(kind => kind.includes("patch")), "fixture must exercise patch resolution");
    const customSource = await readFile(path.join(root, ...userRoot.split("/"), "audit.custom.yaml"), "utf8");
    assert.match(customSource, /["'](?=[^"']*[A-Z])false["']/i, "fixture must exercise quoted mixed-case booleans");

    const schemaIds = new Map();
    for (const tuple of auditResult.tuples) {
      const previous = schemaIds.get(tuple.schemaAsset);
      if (previous === undefined) schemaIds.set(tuple.schemaAsset, tuple.schemaId);
      else assert.equal(tuple.schemaId, previous);
    }
    assert.deepEqual([...schemaIds.keys()].sort(), [schemaAsset, mirrorSchemaAsset].sort());
    const falseTuples = auditResult.tuples.filter(tuple => !tuple.effective);
    assert.equal(falseTuples.length, 3, "fixture must contain namespaced and shared-carrier explicit false tuples");
    const sharedFalseTuples = falseTuples.filter(
      tuple => tuple.configPath === "shared/leading_syllable_reachability",
    );
    assert.equal(sharedFalseTuples.length, 2, "shared carrier must affect both product schema assets");
    assert.equal(new Set(sharedFalseTuples.map(tuple => tuple.settingAsset)).size, 1, "shared tuples must name one carrier");
    assert.equal(new Set(sharedFalseTuples.map(tuple => tuple.schemaAsset)).size, 2, "shared carrier must preserve schema keys");
    const reachabilityOptOuts = falseTuples.map((tuple, index) => ({
      optOutId: `synthetic-future-opt-out-${index + 1}`,
      schemaAsset: tuple.schemaAsset,
      settingAsset: tuple.settingAsset,
      schemaId: tuple.schemaId,
      configPath: tuple.configPath,
      source: {
        repository: "https://example.invalid/owner/schema",
        commit: "1111111111111111111111111111111111111111",
      },
      owner: "Synthetic fixture owner",
      reason: "Synthetic owner-approved compatibility exception used only to validate the future row shape.",
      affectedSurfaces: ["synthetic-real-deploy-path"],
      evidence: {
        kind: "owner-spec",
        path: "docs/owner-spec.md",
        sourceCommit: "2222222222222222222222222222222222222222",
      },
      acceptanceId: `synthetic-${tuple.schemaId}-real-path`,
      approval: {
        decisionId: "D-99",
        approver: "Synthetic fixture owner",
        approvedOn: "2029-01-01",
      },
      review: {
        triggers: ["owner contract changes"],
        reviewBy: "2099-01-01",
      },
    }));
    const coverage = {
      version: "m59-reach03-v1",
      reachabilityFormalismVersion: "m60-reachability-v1",
      manifest: "governance/manifest.json",
      schemaRoots: [
        {
          path: `${productRoot}/`,
          classification: "product",
          manifest: "governance/manifest.json",
          acceptanceRegistry: "governance/coverage.json",
        },
      ],
      reachabilityOptOuts,
      mechanismContract: {
        canonical: "docs/contracts/schema-general-reachability.md",
        schemaGeneralDefault: "translator/leading_syllable_reachability defaults on",
        profilePrecedence: "prefix_fallback_owned is decided per input",
      },
      schemaAssets: [...schemaIds].map(([fullAsset, schemaId]) => ({
          asset: fullAsset.slice(`${productRoot}/`.length),
          schemaId,
          status: "accepted",
          disposition: "top_level_selectable",
          acceptanceId: `synthetic-${schemaId}-real-path`,
          acceptance: { testFile: "docs/owner-spec.md" },
        })),
    };
    const trackedPaths = (await allFiles(root)).filter(trackedPath => trackedPath !== unresolvedSchemaAsset);
    const base = {
      coverage,
      auditResult,
      trackedPaths,
      trackedSchemaPaths: [schemaAsset, mirrorSchemaAsset],
      root,
      productManifestAssets: ["audit.schema.yaml", "audit_mirror.schema.yaml"],
      coverageRegistryPath: "governance/coverage.json",
      asOfDate: new Date("2030-01-01T00:00:00.000Z"),
    };
    const valid = await validateReachabilityFormalism(base);
    assert.deepEqual(valid, { trackedSchemas: 2, productSchemas: 2, auditTuples: 5, optOuts: 3 });

    await t.test("multiple prescriptions sharing one reconciliation key collapse to one opt-out", async () => {
      const candidate = cloneValidation(base);
      const duplicateKeyTuple = structuredClone(candidate.auditResult.tuples.find(tuple => !tuple.effective));
      duplicateKeyTuple.component = "r10n_translator@foo";
      duplicateKeyTuple.translatorArm = "r10n";
      candidate.auditResult.tuples.push(duplicateKeyTuple);
      const collapsed = await validateReachabilityFormalism(candidate);
      assert.equal(collapsed.auditTuples, 6);
      assert.equal(collapsed.optOuts, 3);
    });

    async function rejects(name, mutate, pattern) {
      await t.test(name, async () => {
        const candidate = cloneValidation(base);
        await mutate(candidate);
        await assert.rejects(validateReachabilityFormalism(candidate), pattern);
      });
    }

    await rejects("explicit false without an opt-out", candidate => {
      candidate.coverage.reachabilityOptOuts = [];
    }, /effective false tuple has no approved opt-out/);
    await rejects("shared false carrier requires one opt-out per affected schema", candidate => {
      candidate.coverage.reachabilityOptOuts.pop();
    }, /effective false tuple has no approved opt-out/);
    await rejects("opt-out without a false tuple", candidate => {
      candidate.auditResult.tuples.find(tuple => !tuple.effective).effective = true;
    }, /no matching effective false tuple/);
    await rejects("duplicate opt-out id", candidate => {
      candidate.coverage.reachabilityOptOuts.push(structuredClone(candidate.coverage.reachabilityOptOuts[0]));
    }, /duplicate opt-out id/);
    await rejects("duplicate reconciliation triplet", candidate => {
      const duplicate = structuredClone(candidate.coverage.reachabilityOptOuts[0]);
      duplicate.optOutId = "different-id-same-triplet";
      candidate.coverage.reachabilityOptOuts.push(duplicate);
    }, /duplicates reconciliation key/);
    await rejects("missing required field", candidate => {
      delete candidate.coverage.reachabilityOptOuts[0].owner;
    }, /owner must be a non-empty string/);
    await rejects("malformed source repository", candidate => {
      candidate.coverage.reachabilityOptOuts[0].source.repository = "http://example.invalid/not-https";
    }, /must be an https URL/);
    await rejects("unknown schema asset", candidate => {
      candidate.coverage.reachabilityOptOuts[0].schemaAsset = "docs/owner-spec.md";
    }, /unknown or unmanifested schema asset/);
    await rejects("open acceptance row", candidate => {
      candidate.coverage.schemaAssets[0].status = "open";
    }, /unresolved schema asset/);
    await rejects("stale registry row after asset removal", candidate => {
      candidate.coverage.schemaAssets.push({
        asset: "removed.schema.yaml",
        schemaId: "removed",
        status: "accepted",
        disposition: "top_level_selectable",
        acceptanceId: "removed-real-path",
        acceptance: {},
      });
    }, /stale or unmanifested schema acceptance row/);
    await rejects("non-40-hex source commit", candidate => {
      candidate.coverage.reachabilityOptOuts[0].source.commit = "abc123";
    }, /source.commit must be 40 hex/);
    await rejects("non-40-hex evidence source commit", candidate => {
      candidate.coverage.reachabilityOptOuts[0].evidence.sourceCommit = "abc123";
    }, /evidence.sourceCommit must be 40 hex/);
    await rejects("unsafe repository-relative evidence path", candidate => {
      candidate.coverage.reachabilityOptOuts[0].evidence.path = "../outside.md";
    }, /must not escape the repository/);
    await rejects("missing tracked evidence path", candidate => {
      candidate.coverage.reachabilityOptOuts[0].evidence.path = "docs/missing.md";
    }, /is not tracked/);
    await rejects("intermediate symbolic-link path component", async candidate => {
      await symlink(
        path.join(root, "docs"),
        path.join(root, "linked-docs"),
        process.platform === "win32" ? "junction" : "dir",
      );
      candidate.trackedPaths.push("linked-docs/owner-spec.md");
      candidate.coverage.reachabilityOptOuts[0].evidence.path = "linked-docs/owner-spec.md";
    }, /traverses symbolic link component linked-docs/);
    await rejects("empty affected surfaces", candidate => {
      candidate.coverage.reachabilityOptOuts[0].affectedSurfaces = [];
    }, /affectedSurfaces must be a non-empty array/);
    await rejects("empty review triggers", candidate => {
      candidate.coverage.reachabilityOptOuts[0].review.triggers = [];
    }, /review.triggers must be a non-empty array/);
    await rejects("invalid approval date", candidate => {
      candidate.coverage.reachabilityOptOuts[0].approval.approvedOn = "2029-02-30";
    }, /approval.approvedOn is invalid/);
    await rejects("invalid review date", candidate => {
      candidate.coverage.reachabilityOptOuts[0].review.reviewBy = "not-a-date";
    }, /review.reviewBy must use YYYY-MM-DD/);
    await rejects("expired review row", candidate => {
      candidate.coverage.reachabilityOptOuts[0].review.reviewBy = "2029-12-31";
    }, /expired on 2029-12-31/);
    await rejects("decision id does not prefix-match a longer heading", candidate => {
      candidate.coverage.reachabilityOptOuts[0].approval.decisionId = "D-9";
    }, /approval decision is missing/);
    await rejects("dependency-only opt-out", candidate => {
      candidate.coverage.schemaAssets[0].disposition = "dependency_only";
    }, /cannot use dependency-only schema/);
    await rejects("unregistered tracked schema root", async candidate => {
      await writeFile(path.join(root, "docs", "outside.schema.yaml"), "schema:\n  schema_id: outside\n");
      candidate.trackedPaths.push("docs/outside.schema.yaml");
      candidate.trackedSchemaPaths.push("docs/outside.schema.yaml");
    }, /must match exactly one classified schema root/);
    await rejects("overlapping schema roots", async candidate => {
      candidate.coverage.schemaRoots.push({
        path: `${productRoot}/nested/`,
        classification: "test-fixture",
        disposition: "synthetic",
      });
      await mkdir(path.join(root, ...productRoot.split("/"), "nested"), { recursive: true });
    }, /schema roots overlap/);
    await rejects("malformed audit tuple", candidate => {
      delete candidate.auditResult.tuples[0].component;
    }, /component is not a deployed dictionary translator prescription/);
    await rejects("asset hash mismatch", candidate => {
      candidate.auditResult.tuples[0].assetHashes[0].sha256 = "0".repeat(64);
    }, /asset hash mismatch/);

    await t.test("unresolved relevant merge fails in the real Rust extractor", async () => {
      await assert.rejects(
        runReachabilityAudit({
          root,
          cargoRoot: repoRoot,
          productRoot,
          sharedDataRoot: productRoot,
          userDataRoot: userRoot,
          schemaAssets: [unresolvedSchemaAsset],
        }),
        /unresolved|unsupported|failed/i,
      );
    });
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
