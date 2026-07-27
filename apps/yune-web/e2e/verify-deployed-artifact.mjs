import { mkdir, readFile, writeFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  manifestName,
  sha256,
  validateLocalBundle,
} from "./public-artifact-verifier.mjs";
import {
  productionManifestRows,
  verifyProductionManifestMember,
} from "./production-artifact-verifier.mjs";

const e2eRoot = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(e2eRoot, "..");
const distRoot = path.resolve(
  process.env.YUNE_WEB_EXPECTED_DIST ?? path.join(appRoot, "public-demo", "dist"),
);
const expectedCommit = process.env.YUNE_WEB_EXPECTED_SOURCE_COMMIT
  ?.trim()
  .toLowerCase();
const appUrlValue = process.env.YUNE_WEB_APP_URL;
const receiptPath = path.resolve(
  process.env.YUNE_WEB_DEPLOYMENT_RECEIPT ??
    path.join(e2eRoot, "test-results", "deployed-artifact-verification.json"),
);
const waitMs = Number(process.env.YUNE_WEB_DEPLOYMENT_WAIT_MS ?? "120000");
const requestTimeoutMs = Number(
  process.env.YUNE_WEB_DEPLOYMENT_REQUEST_TIMEOUT_MS ?? "20000",
);

async function fetchBytes(relative) {
  const url = new URL(relative, appUrlValue);
  url.searchParams.set("yuneSource", expectedCommit);
  url.searchParams.set("yuneVerify", `${Date.now()}-${randomUUID()}`);
  const response = await fetch(url, {
    cache: "no-store",
    signal: AbortSignal.timeout(requestTimeoutMs),
  });
  if (!response.ok) throw new Error(`${relative} returned HTTP ${response.status}`);
  return Buffer.from(await response.arrayBuffer());
}

async function waitForExactBytes(relative, expectedBytes, deadline) {
  let lastObservation = null;
  do {
    try {
      const bytes = await fetchBytes(relative);
      lastObservation = `${bytes.byteLength} bytes / sha256 ${sha256(bytes)}`;
      if (bytes.equals(expectedBytes)) return bytes;
    } catch (error) {
      lastObservation = error instanceof Error ? error.message : String(error);
    }
    if (Date.now() >= deadline) break;
    await new Promise((resolve) => setTimeout(resolve, 5_000));
  } while (true);
  throw new Error(
    `Deployment did not serve exact certified ${relative} for ${expectedCommit}; ` +
      `last observation: ${lastObservation}`,
  );
}

const startedAt = new Date().toISOString();
const verified = [];
let passed = false;
let failure = null;

try {
  if (!appUrlValue) throw new Error("YUNE_WEB_APP_URL is required");
  if (!/^[0-9a-f]{40}$/.test(expectedCommit ?? "")) {
    throw new Error("YUNE_WEB_EXPECTED_SOURCE_COMMIT must be a full lowercase SHA");
  }
  if (!Number.isSafeInteger(waitMs) || waitMs < 0 || waitMs > 300_000) {
    throw new Error(`Invalid YUNE_WEB_DEPLOYMENT_WAIT_MS: ${waitMs}`);
  }
  if (
    !Number.isSafeInteger(requestTimeoutMs) ||
    requestTimeoutMs < 1_000 ||
    requestTimeoutMs > 60_000
  ) {
    throw new Error(
      `Invalid YUNE_WEB_DEPLOYMENT_REQUEST_TIMEOUT_MS: ${requestTimeoutMs}`,
    );
  }

  const local = await validateLocalBundle(distRoot, expectedCommit);
  const propagationDeadline = Date.now() + waitMs;
  const remoteBuildInfoBytes = await waitForExactBytes(
    "build-info.json",
    local.buildInfoBytes,
    propagationDeadline,
  );
  verified.push({ path: "build-info.json", sha256: sha256(remoteBuildInfoBytes) });

  const remoteManifestBytes = await waitForExactBytes(
    manifestName,
    local.manifestBytes,
    propagationDeadline,
  );
  verified.push({ path: manifestName, sha256: sha256(remoteManifestBytes) });

  for (const expected of productionManifestRows(local.manifest)) {
    const relative = expected.path;
    const expectedBytes = await readFile(path.join(distRoot, ...relative.split("/")));
    const bytes = await waitForExactBytes(
      relative,
      expectedBytes,
      propagationDeadline,
    );
    verifyProductionManifestMember(expected, bytes);
    verified.push({ path: relative, bytes: bytes.byteLength, sha256: expected.sha256 });
  }
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
        appUrl: appUrlValue ?? null,
        expectedSourceCommit: expectedCommit ?? null,
        passed,
        failure,
        verified,
      },
      null,
      2,
    )}\n`,
  );
}

if (!passed) throw new Error(failure);
console.log(`Verified deployed yune-web artifact at ${appUrlValue}`);
