import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { validateLocalBundle } from "./public-artifact-verifier.mjs";

const distRoot = path.resolve(process.env.YUNE_WEB_EXPECTED_DIST ?? "");
const expectedCommit = process.env.YUNE_WEB_EXPECTED_SOURCE_COMMIT
  ?.trim()
  .toLowerCase();
const receiptPath = path.resolve(
  process.env.YUNE_WEB_LOCAL_ARTIFACT_RECEIPT ??
    path.join(distRoot, "..", "local-artifact-verification.json"),
);
const startedAt = new Date().toISOString();
let passed = false;
let failure = null;
let fileCount = null;

try {
  if (!process.env.YUNE_WEB_EXPECTED_DIST) {
    throw new Error("YUNE_WEB_EXPECTED_DIST is required");
  }
  if (!/^[0-9a-f]{40}$/.test(expectedCommit ?? "")) {
    throw new Error("YUNE_WEB_EXPECTED_SOURCE_COMMIT must be a full lowercase SHA");
  }
  const local = await validateLocalBundle(distRoot, expectedCommit);
  fileCount = local.manifest.files.length;
  passed = true;
} catch (error) {
  failure = error instanceof Error ? error.message : String(error);
} finally {
  await mkdir(path.dirname(receiptPath), { recursive: true });
  await writeFile(
    receiptPath,
    `${JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        startedAt,
        expectedSourceCommit: expectedCommit ?? null,
        distRoot,
        passed,
        failure,
        fileCount,
      },
      null,
      2,
    )}\n`,
  );
}

if (!passed) throw new Error(failure);
console.log(`Verified ${fileCount} files in the local certified artifact`);
