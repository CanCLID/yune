import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const SHA40 = /^[0-9a-f]{40}$/;
const RUN_ID = /^[1-9][0-9]*$/;

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

export function validateSetupOnlyRetry({ priorRun, priorReceipt, current }) {
  assert(RUN_ID.test(String(current?.runId ?? "")), "Current WEB06 run ID is invalid");
  assert(current?.runAttempt === 1, "WEB06 retries must be fresh workflow runs at attempt 1");
  assert(SHA40.test(current?.sourceCommit ?? ""), "Current WEB06 source commit is invalid");
  assert(typeof current?.workflowPath === "string" && current.workflowPath !== "", "Current workflow path is invalid");

  assert(priorRun?.id === Number(current.retryOf), "Setup retry does not reference the declared prior run");
  assert(priorRun?.run_attempt === 1, "Only an attempt-1 workflow may be retried");
  assert(priorRun?.head_sha === current.sourceCommit, "Setup retry source differs from the prior run");
  assert(priorRun?.path === current.workflowPath, "Setup retry references another workflow");
  assert(priorRun?.status === "completed" && priorRun?.conclusion === "failure", "Setup retry requires a completed failed prior run");

  assert(priorReceipt?.version === "web06-workflow-attempt-v1", "Prior WEB06 attempt receipt version is invalid");
  assert(priorReceipt.workflowPath === current.workflowPath, "Prior receipt workflow differs");
  assert(priorReceipt.sourceCommit === current.sourceCommit, "Prior receipt source differs");
  assert(priorReceipt.runId === String(priorRun.id), "Prior receipt run ID differs");
  assert(priorReceipt.runAttempt === 1, "Prior receipt was not attempt 1");
  assert(priorReceipt.status === "failed", "Prior receipt is not failed");
  assert(priorReceipt.certifyOutcome === "failure", "Prior receipt does not bind a failed certification");
  assert(typeof priorReceipt.web06HandoffRequired === "boolean", "Prior receipt lacks its handoff classification");
  assert(priorReceipt.failureClass === "SETUP_ONLY", "Prior failure was not explicitly setup-only");
  assert(priorReceipt.measurementStarted === false, "Prior failure occurred after measurement started");
  assert(priorReceipt.measurementCompleted === false, "Prior setup failure claims completed measurement");
  assert(priorReceipt.validRedObserved === false, "A prior valid RED may never be retried");
  assert(priorReceipt.canaryStarted === false, "A started preview canary may never be retried as setup-only");
  assert(priorReceipt.productionMutationStarted === false, "A production mutation may never be retried as setup-only");
  return Object.freeze({
    retryOf: String(priorRun.id),
    sourceCommit: current.sourceCommit,
    workflowPath: current.workflowPath,
    status: "SETUP_RETRY_AUTHORIZED",
  });
}

const invoked = process.argv[1] ? pathToFileURL(process.argv[1]).href : null;
if (invoked === import.meta.url) {
  try {
    if (process.argv.length !== 5) {
      throw new Error("usage: node web06-setup-retry.mjs PRIOR_RUN PRIOR_RECEIPT CURRENT");
    }
    const [priorRun, priorReceipt, current] = await Promise.all(
      process.argv.slice(2).map(async (file) => JSON.parse(await readFile(file, "utf8"))),
    );
    process.stdout.write(`${JSON.stringify(validateSetupOnlyRetry({ priorRun, priorReceipt, current }))}\n`);
  } catch (error) {
    console.error(error);
    process.exitCode = 1;
  }
}
