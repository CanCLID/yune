import assert from "node:assert/strict";
import test from "node:test";

import { validateSetupOnlyRetry } from "./web06-setup-retry.mjs";

const sourceCommit = "1".repeat(40);
const workflowPath = ".github/workflows/deploy-yune-web.yml";
const current = {
  runId: "200",
  runAttempt: 1,
  retryOf: "100",
  sourceCommit,
  workflowPath,
};
const priorRun = {
  id: 100,
  run_attempt: 1,
  head_sha: sourceCommit,
  path: workflowPath,
  status: "completed",
  conclusion: "failure",
};
const priorReceipt = {
  version: "web06-workflow-attempt-v1",
  workflowPath,
  sourceCommit,
  runId: "100",
  runAttempt: 1,
  web06HandoffRequired: false,
  certifyOutcome: "failure",
  status: "failed",
  failureClass: "SETUP_ONLY",
  measurementStarted: false,
  measurementCompleted: false,
  validRedObserved: false,
  canaryStarted: false,
  productionMutationStarted: false,
};

test("a fresh same-workflow/source attempt-1 setup-only failure is retryable", () => {
  assert.equal(
    validateSetupOnlyRetry({ priorRun, priorReceipt, current }).status,
    "SETUP_RETRY_AUTHORIZED",
  );
});

test("a valid RED or started measurement can never be relabelled setup-only", () => {
  assert.throws(
    () => validateSetupOnlyRetry({
      priorRun,
      priorReceipt: { ...priorReceipt, validRedObserved: true },
      current,
    }),
    /valid RED may never be retried/,
  );
  assert.throws(
    () => validateSetupOnlyRetry({
      priorRun,
      priorReceipt: { ...priorReceipt, measurementStarted: true },
      current,
    }),
    /after measurement started/,
  );
  assert.throws(
    () => validateSetupOnlyRetry({
      priorRun,
      priorReceipt: { ...priorReceipt, measurementCompleted: true },
      current,
    }),
    /claims completed measurement/,
  );
});

test("retry authorization rejects a different source, workflow, or rerun attempt", () => {
  assert.throws(
    () => validateSetupOnlyRetry({
      priorRun: { ...priorRun, head_sha: "2".repeat(40) },
      priorReceipt,
      current,
    }),
    /source differs/,
  );
  assert.throws(
    () => validateSetupOnlyRetry({
      priorRun,
      priorReceipt,
      current: { ...current, runAttempt: 2 },
    }),
    /fresh workflow runs at attempt 1/,
  );
});
