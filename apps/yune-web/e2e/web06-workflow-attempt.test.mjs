import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { finalizeWorkflowAttempt } from "./web06-workflow-attempt.mjs";

const sourceCommit = "1".repeat(40);

async function fixture(root, {
  certifyOutcome = "failure",
  marker = null,
  latency = null,
  normal = null,
  certification = null,
} = {}) {
  await mkdir(root, { recursive: true });
  const paths = {
    currentPath: path.join(root, "current.json"),
    measurementMarkerPath: path.join(root, "measurement-started.json"),
    latencyReceiptPath: path.join(root, "latency.json"),
    normalReceiptPath: path.join(root, "normal.json"),
    certificationReceiptPath: path.join(root, "certification.json"),
    outputPath: path.join(root, "workflow-attempt.json"),
  };
  await writeFile(paths.currentPath, `${JSON.stringify({
    version: "web06-workflow-attempt-input-v1",
    workflowPath: ".github/workflows/deploy-yune-web.yml",
    sourceCommit,
    runId: "1234",
    runAttempt: 1,
    web06HandoffRequired: false,
    certifyOutcome,
  })}\n`);
  for (const [file, value] of [
    [paths.measurementMarkerPath, marker],
    [paths.latencyReceiptPath, latency],
    [paths.normalReceiptPath, normal],
    [paths.certificationReceiptPath, certification],
  ]) {
    if (value !== null) await writeFile(file, `${JSON.stringify(value)}\n`);
  }
  return paths;
}

const marker = {
  version: "web06-workflow-measurement-start-v1",
  sourceCommit,
  scenarioId: "web03-normal-typing",
  startedAt: "2026-07-22T03:30:00.123Z",
};
const passingLatency = {
  measurementCompleted: true,
  passed: true,
  thresholdVerdict: "pass",
  releaseGradeVerdict: "pass",
  buildInfo: { sourceCommit },
};
const passingNormal = {
  measurementCompleted: true,
  passed: true,
  buildInfo: { sourceCommit },
};
const passingCertification = {
  version: "yune-web-legacy-local-certification-v1",
  sourceCommit,
  status: "passed",
};

test("a pre-measurement certification failure remains setup-only retryable", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-workflow-setup-"));
  try {
    const paths = await fixture(root);
    const receipt = await finalizeWorkflowAttempt(paths);
    assert.equal(receipt.status, "failed");
    assert.equal(receipt.failureClass, "SETUP_ONLY");
    assert.equal(receipt.measurementStarted, false);
    assert.equal(receipt.validRedObserved, false);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("a completed valid red is retained and never classified setup-only", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-workflow-red-"));
  try {
    const paths = await fixture(root, {
      marker,
      latency: { ...passingLatency, passed: false, thresholdVerdict: "fail" },
      normal: passingNormal,
    });
    const receipt = await finalizeWorkflowAttempt(paths);
    assert.equal(receipt.status, "failed");
    assert.equal(receipt.failureClass, "MEASUREMENT_OR_BEHAVIOR");
    assert.equal(receipt.measurementStarted, true);
    assert.equal(receipt.measurementCompleted, true);
    assert.equal(receipt.validRedObserved, true);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("a post-start incomplete failure is non-retryable without inventing a red", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-workflow-incomplete-"));
  try {
    const paths = await fixture(root, {
      marker,
      latency: {
        measurementCompleted: false,
        passed: false,
        buildInfo: { sourceCommit },
      },
    });
    const receipt = await finalizeWorkflowAttempt(paths);
    assert.equal(receipt.failureClass, "MEASUREMENT_OR_BEHAVIOR");
    assert.equal(receipt.measurementStarted, true);
    assert.equal(receipt.measurementCompleted, false);
    assert.equal(receipt.validRedObserved, false);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("step success cannot pass without source-bound passing evidence", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-workflow-success-"));
  try {
    const validPaths = await fixture(path.join(root, "valid"), {
      certifyOutcome: "success",
      marker,
      latency: passingLatency,
      normal: passingNormal,
      certification: passingCertification,
    });
    const valid = await finalizeWorkflowAttempt(validPaths);
    assert.equal(valid.status, "passed");
    assert.equal(valid.failureClass, "NONE");

    const invalidPaths = await fixture(path.join(root, "invalid"), {
      certifyOutcome: "success",
      marker,
      latency: passingLatency,
      normal: { ...passingNormal, buildInfo: { sourceCommit: "2".repeat(40) } },
      certification: passingCertification,
    });
    const invalid = await finalizeWorkflowAttempt(invalidPaths);
    assert.equal(invalid.status, "failed");
    assert.equal(invalid.failureClass, "MEASUREMENT_OR_BEHAVIOR");
    assert.equal(invalid.validRedObserved, false);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("workflow attempt output is immutable create-new evidence", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-workflow-immutable-"));
  try {
    const paths = await fixture(root);
    await finalizeWorkflowAttempt(paths);
    const original = await readFile(paths.outputPath);
    await assert.rejects(finalizeWorkflowAttempt(paths), { code: "EEXIST" });
    assert.deepEqual(await readFile(paths.outputPath), original);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
