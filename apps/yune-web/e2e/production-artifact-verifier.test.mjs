import assert from "node:assert/strict";
import test from "node:test";

import { sha256 } from "./public-artifact-verifier.mjs";
import {
  productionManifestRows,
  verifyProductionManifestMember,
} from "./production-artifact-verifier.mjs";

test("production verification includes and byte-checks every manifest row", () => {
  const ordinary = Buffer.from("app\n");
  const formerlyUnverified = Buffer.from("rare split asset\n");
  const manifest = {
    files: [
      { path: "assets/app.js", bytes: ordinary.length, sha256: sha256(ordinary) },
      {
        path: "schema/rare.bin.part17",
        bytes: formerlyUnverified.length,
        sha256: sha256(formerlyUnverified),
      },
    ],
  };
  assert.deepEqual(
    productionManifestRows(manifest).map((row) => row.path),
    ["assets/app.js", "schema/rare.bin.part17"],
  );
  assert.doesNotThrow(() =>
    verifyProductionManifestMember(manifest.files[1], formerlyUnverified),
  );
  assert.throws(
    () => verifyProductionManifestMember(manifest.files[1], Buffer.from("tampered\n")),
    /differs from the certified manifest/,
  );
});
