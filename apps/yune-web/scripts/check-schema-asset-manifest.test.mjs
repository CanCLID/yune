import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  assertManifestMatchesTree,
  schemaTreeEntryKind,
} from "./check-schema-asset-manifest.mjs";

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
