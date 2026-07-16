import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  main as updateSchemaAssetManifest,
  reconcileSchemaCoverage,
} from "./update-schema-asset-manifest.mjs";

const formalismBlock = {
  version: "m59-reach03-v1",
  reachabilityFormalismVersion: "m60-reachability-v1",
  schemaRoots: [
    {
      path: "apps/yune-web/public/schema/",
      classification: "product",
      manifest: "apps/yune-web/public/schema-asset-manifest.json",
      acceptanceRegistry: "apps/yune-web/schema-acceptance-coverage.json",
    },
  ],
  reachabilityOptOuts: [
    {
      optOutId: "preserved-sentinel",
      owner: "fixture-owner",
    },
  ],
};

test("reconciliation preserves governance blocks and opens every new schema", () => {
  const coverage = {
    ...structuredClone(formalismBlock),
    schemaAssets: [
      {
        asset: "alpha.schema.yaml",
        schemaId: "alpha",
        status: "accepted",
        disposition: "top_level_selectable",
        acceptanceId: "alpha-real-path",
      },
    ],
  };
  const before = JSON.stringify(formalismBlock);
  const reconciled = reconcileSchemaCoverage({
    coverage,
    manifestedSchemaAssets: ["alpha.schema.yaml", "nested/beta.schema.yaml"],
    schemaIdsByAsset: new Map([
      ["alpha.schema.yaml", "alpha"],
      ["nested/beta.schema.yaml", "beta"],
    ]),
  });
  assert.equal(
    JSON.stringify({
      version: reconciled.version,
      reachabilityFormalismVersion: reconciled.reachabilityFormalismVersion,
      schemaRoots: reconciled.schemaRoots,
      reachabilityOptOuts: reconciled.reachabilityOptOuts,
    }),
    before,
  );
  assert.deepEqual(reconciled.schemaAssets[0], coverage.schemaAssets[0]);
  assert.deepEqual(reconciled.schemaAssets[1], {
    asset: "nested/beta.schema.yaml",
    schemaId: "beta",
    status: "open",
    disposition: "unclassified",
    acceptanceId: "open-beta",
    reason: "Automatically opened by update-schema-asset-manifest.mjs; classify ownership and add real-path acceptance before the manifest gate can pass.",
  });
  assert.equal(Object.hasOwn(reconciled.schemaAssets[1], "reachabilityOptOut"), false);
  assert.equal(reconciled.schemaAssets[1].status, "open");
});

test("production updater entry point creates a blocking open row in a temporary app", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "yune-schema-updater-"));
  const appRoot = path.join(root, "apps", "yune-web");
  const schemaRoot = path.join(appRoot, "public", "schema");
  try {
    await mkdir(path.join(schemaRoot, "nested"), { recursive: true });
    await mkdir(path.join(appRoot, "public-demo"), { recursive: true });
    await writeFile(path.join(schemaRoot, "alpha.schema.yaml"), "schema:\n  schema_id: alpha\n");
    await writeFile(path.join(schemaRoot, "nested", "beta.schema.yaml"), "schema:\n  schema_id: beta\n");
    const alphaBytes = Buffer.byteLength("schema:\n  schema_id: alpha\n");
    const template = {
      version: "web03-three-schema-launch-v1",
      generatedFor: "yune-web",
      sourceRoot: "apps/yune-web/public/schema",
      assets: [
        {
          path: "alpha.schema.yaml",
          sha256: "ignored-template-hash",
          bytes: alphaBytes,
          tier: "shared",
          required: true,
        },
      ],
    };
    await writeFile(
      path.join(appRoot, "public-demo", "schema-asset-manifest.json"),
      `${JSON.stringify(template, null, 2)}\n`,
    );
    const coverage = {
      ...structuredClone(formalismBlock),
      manifest: "apps/yune-web/public/schema-asset-manifest.json",
      schemaAssets: [
        {
          asset: "alpha.schema.yaml",
          schemaId: "alpha",
          status: "accepted",
          disposition: "top_level_selectable",
          acceptanceId: "alpha-real-path",
        },
      ],
    };
    await writeFile(
      path.join(appRoot, "schema-acceptance-coverage.json"),
      `${JSON.stringify(coverage, null, 2)}\n`,
    );

    const messages = [];
    await updateSchemaAssetManifest({
      appRoot,
      repoRoot: root,
      extras: [],
      log: message => messages.push(message),
    });

    const updatedCoverageText = await readFile(path.join(appRoot, "schema-acceptance-coverage.json"), "utf8");
    const updatedCoverage = JSON.parse(updatedCoverageText);
    assert.equal(updatedCoverage.schemaAssets[1].asset, "nested/beta.schema.yaml");
    assert.equal(updatedCoverage.schemaAssets[1].status, "open");
    assert.equal(updatedCoverage.schemaAssets[1].disposition, "unclassified");
    assert.deepEqual(updatedCoverage.schemaRoots, coverage.schemaRoots);
    assert.deepEqual(updatedCoverage.reachabilityOptOuts, coverage.reachabilityOptOuts);
    assert.equal(updatedCoverage.reachabilityFormalismVersion, coverage.reachabilityFormalismVersion);
    assert.match(updatedCoverageText, /"status": "open"/);
    assert.equal(messages.length, 3);

    const publicManifest = await readFile(path.join(appRoot, "public", "schema-asset-manifest.json"), "utf8");
    const demoManifest = await readFile(path.join(appRoot, "public-demo", "schema-asset-manifest.json"), "utf8");
    assert.equal(publicManifest, demoManifest);
    assert.deepEqual(
      JSON.parse(publicManifest).assets.map(asset => asset.path),
      ["alpha.schema.yaml", "nested/beta.schema.yaml"],
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
