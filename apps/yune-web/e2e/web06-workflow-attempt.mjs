import { lstat, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

const SHA40 = /^[0-9a-f]{40}$/;
const ISO_SECOND = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/;

async function readPlainJson(file, label, { optional = false } = {}) {
  let metadata;
  try {
    metadata = await lstat(file);
  } catch (error) {
    if (optional && error?.code === "ENOENT") return null;
    throw error;
  }
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new Error(`${label} must be a plain file`);
  }
  let payload;
  try {
    payload = JSON.parse(await readFile(file, "utf8"));
  } catch {
    throw new Error(`${label} must be valid JSON`);
  }
  return payload;
}

function validateCurrent(current) {
  if (
    current?.version !== "web06-workflow-attempt-input-v1" ||
    current.workflowPath !== ".github/workflows/deploy-yune-web.yml" ||
    !SHA40.test(current.sourceCommit ?? "") ||
    typeof current.runId !== "string" ||
    !/^[1-9][0-9]*$/.test(current.runId) ||
    current.runAttempt !== 1 ||
    typeof current.web06HandoffRequired !== "boolean" ||
    !["success", "failure"].includes(current.certifyOutcome)
  ) {
    throw new Error("Workflow attempt input is malformed");
  }
  return current;
}

function validateMeasurementMarker(marker, sourceCommit) {
  if (
    marker?.version !== "web06-workflow-measurement-start-v1" ||
    marker.sourceCommit !== sourceCommit ||
    !["web03-normal-typing", "web03-release-hard-stop"].includes(
      marker.scenarioId,
    ) ||
    typeof marker.startedAt !== "string" ||
    !ISO_SECOND.test(marker.startedAt.replace(/\.\d{3}Z$/, "Z"))
  ) {
    throw new Error("Workflow measurement-start marker is malformed or unbound");
  }
  return marker;
}

function passingLatency(receipt, sourceCommit) {
  return (
    receipt?.measurementCompleted === true &&
    receipt.passed === true &&
    receipt.thresholdVerdict === "pass" &&
    receipt.releaseGradeVerdict === "pass" &&
    receipt.buildInfo?.sourceCommit === sourceCommit
  );
}

function passingNormal(receipt, sourceCommit) {
  return (
    receipt?.measurementCompleted === true &&
    receipt.passed === true &&
    receipt.buildInfo?.sourceCommit === sourceCommit
  );
}

function completedRed(receipt, sourceCommit) {
  return (
    receipt?.measurementCompleted === true &&
    receipt.passed === false &&
    receipt.buildInfo?.sourceCommit === sourceCommit
  );
}

function passingCertification(receipt, sourceCommit) {
  return (
    [
      "yune-web-legacy-local-certification-v1",
      "web06-local-release-certification-v1",
    ].includes(receipt?.version) &&
    receipt.sourceCommit === sourceCommit &&
    receipt.status === "passed"
  );
}

export async function finalizeWorkflowAttempt({
  currentPath,
  measurementMarkerPath,
  latencyReceiptPath,
  normalReceiptPath,
  certificationReceiptPath,
  outputPath,
}) {
  const current = validateCurrent(
    await readPlainJson(currentPath, "Workflow attempt input"),
  );
  const marker = await readPlainJson(
    measurementMarkerPath,
    "Workflow measurement-start marker",
    { optional: true },
  );
  if (marker !== null) validateMeasurementMarker(marker, current.sourceCommit);
  const [latency, normal, certification] = await Promise.all([
    readPlainJson(latencyReceiptPath, "WEB03 latency receipt", {
      optional: true,
    }),
    readPlainJson(normalReceiptPath, "WEB03 normal-typing receipt", {
      optional: true,
    }),
    readPlainJson(certificationReceiptPath, "Local certification receipt", {
      optional: true,
    }),
  ]);
  const measurementStarted = marker !== null;
  const measurementCompleted =
    latency?.measurementCompleted === true &&
    normal?.measurementCompleted === true;
  const validRedObserved =
    completedRed(latency, current.sourceCommit) ||
    completedRed(normal, current.sourceCommit);
  const evidencePassed =
    measurementStarted &&
    passingLatency(latency, current.sourceCommit) &&
    passingNormal(normal, current.sourceCommit) &&
    passingCertification(certification, current.sourceCommit);

  let status;
  let failureClass;
  if (current.certifyOutcome === "success" && evidencePassed) {
    status = "passed";
    failureClass = "NONE";
  } else if (current.certifyOutcome === "failure" && !measurementStarted) {
    status = "failed";
    failureClass = "SETUP_ONLY";
  } else if (measurementStarted) {
    status = "failed";
    failureClass = "MEASUREMENT_OR_BEHAVIOR";
  } else {
    status = "failed";
    failureClass = "EVIDENCE_INVALID";
  }
  const receipt = {
    version: "web06-workflow-attempt-v1",
    workflowPath: current.workflowPath,
    sourceCommit: current.sourceCommit,
    runId: current.runId,
    runAttempt: current.runAttempt,
    web06HandoffRequired: current.web06HandoffRequired,
    certifyOutcome: current.certifyOutcome,
    status,
    failureClass,
    measurementStarted,
    measurementCompleted,
    validRedObserved,
    canaryStarted: false,
    productionMutationStarted: false,
  };
  await writeFile(outputPath, `${JSON.stringify(receipt, null, 2)}\n`, {
    flag: "wx",
    mode: 0o600,
  });
  return receipt;
}

async function cli() {
  if (process.argv.length !== 8) {
    throw new Error(
      "usage: web06-workflow-attempt.mjs CURRENT MARKER LATENCY NORMAL CERTIFICATION OUTPUT",
    );
  }
  await finalizeWorkflowAttempt({
    currentPath: path.resolve(process.argv[2]),
    measurementMarkerPath: path.resolve(process.argv[3]),
    latencyReceiptPath: path.resolve(process.argv[4]),
    normalReceiptPath: path.resolve(process.argv[5]),
    certificationReceiptPath: path.resolve(process.argv[6]),
    outputPath: path.resolve(process.argv[7]),
  });
}

const invokedPath = process.argv[1] ? pathToFileURL(process.argv[1]).href : null;
if (invokedPath === import.meta.url) {
  try {
    await cli();
  } catch (error) {
    console.error(error);
    process.exitCode = 1;
  }
}
