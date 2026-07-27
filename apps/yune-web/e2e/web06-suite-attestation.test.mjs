import assert from "node:assert/strict";
import {
  copyFile,
  mkdir,
  readFile,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import {
  artifactReference,
  createBaselineSetupNoGoBundle,
  createObserverPartialBundle,
  digestJson,
  refreshSourceRoleBindings,
  sha256,
  writeCanonicalJson,
} from "./web06-artifact-set-test-fixture.mjs";
import {
  WEB06_BINDING_SCENARIO_ORDER,
  WEB06_RUNNER_TOOLING_PATHS as METRIC_RUNNER_TOOLING_PATHS,
} from "./web06-collector.mjs";
import {
  WEB06_RUNNER_TOOLING_PATHS,
} from "./web06-suite-attestation.mjs";

const HASH_D = "d".repeat(64);

const readJson = async (file) =>
  JSON.parse(await readFile(file, "utf8"));

async function snapshotArtifacts(bundle) {
  return new Map(
    await Promise.all(
      Object.values(bundle.paths).map(async (file) => [
        file,
        await readFile(file),
      ]),
    ),
  );
}

async function restoreArtifacts(snapshot) {
  for (const [file, bytes] of snapshot) {
    await rm(file, { recursive: true, force: true });
    await writeFile(file, bytes);
  }
}

async function updateAttestationReferences(bundle, attestation, {
  collector = false,
  independent = false,
} = {}) {
  if (collector) {
    attestation.collectorOutput = await artifactReference(
      bundle.paths.collector,
      attestation.collectorOutput.relativePath,
    );
  }
  if (independent) {
    attestation.independentRecompute = await artifactReference(
      bundle.paths.independent,
      attestation.independentRecompute.relativePath,
    );
  }
  await writeCanonicalJson(bundle.paths.attestation, attestation);
}

async function rewriteIndependentCollectorLink(bundle, independent) {
  independent.collectorOutputSha256 =
    (await artifactReference(bundle.paths.collector)).sha256;
  await writeCanonicalJson(bundle.paths.independent, independent);
}

async function rewriteRawPackets(bundle, collector, independent, mutate) {
  const independentByIdentity = new Map(
    independent.scenarioResults.flatMap((scenario) =>
      scenario.attemptResults.map((attempt) => [
        `${scenario.scenarioRunId}:${attempt.attemptId}`,
        attempt,
      ]),
    ),
  );
  for (const scenario of collector.scenarioResults) {
    for (const attempt of scenario.attempts) {
      const file = path.join(
        bundle.evidenceRoot,
        ...attempt.rawPacket.relativePath.split("/"),
      );
      const envelope = await readJson(file);
      mutate(envelope, scenario, attempt);
      const bytes = Buffer.from(`${JSON.stringify(envelope)}\n`, "utf8");
      await writeFile(file, bytes);
      attempt.rawPacket.bytes = bytes.length;
      attempt.rawPacket.sha256 = sha256(bytes);
      independentByIdentity.get(
        `${scenario.scenarioRunId}:${attempt.attemptId}`,
      ).rawPacketSha256 = attempt.rawPacket.sha256;
    }
  }
}

test("release verifier imports the metric-owned exact runner tooling set", () => {
  assert.strictEqual(WEB06_RUNNER_TOOLING_PATHS, METRIC_RUNNER_TOOLING_PATHS);
  assert.equal(WEB06_RUNNER_TOOLING_PATHS.length, 12);
  assert.ok(
    WEB06_RUNNER_TOOLING_PATHS.includes(
      "apps/yune-web/e2e/web06-suite-attestation.mjs",
    ),
  );
});

test("a genuine source-bound collector/independent/attestation bundle passes", async () => {
  const bundle = await createBaselineSetupNoGoBundle();
  try {
    const verified = await bundle.verify();
    assert.equal(verified.payload.expectation, "BASELINE");
    assert.equal(verified.payload.verdict, "SETUP_NO_GO");
    assert.equal(verified.recomputed.verificationStatus, "PASS");
    assert.deepEqual(
      verified.independent.payload,
      verified.recomputed,
    );
    assert.equal(
      verified.payload.scenarioResults.length,
      WEB06_BINDING_SCENARIO_ORDER.length,
    );
    assert.ok(
      verified.payload.scenarioResults.every(
        (scenario) =>
          scenario.attempts.length === 7 &&
          scenario.attempts.every(
            (attempt) =>
              attempt.classification === "SETUP_INVALID" &&
              attempt.measurementStarted === false,
          ),
      ),
    );
  } finally {
    await bundle.cleanup();
  }
});

test("missing, escaping, and symlinked artifact references fail closed", async () => {
  const bundle = await createBaselineSetupNoGoBundle();
  const snapshot = await snapshotArtifacts(bundle);
  try {
    for (const [label, mutate, expected] of [
      [
        "missing collector",
        async () => rm(bundle.paths.collector),
        /ENOENT/,
      ],
      [
        "missing independent",
        async () => rm(bundle.paths.independent),
        /ENOENT/,
      ],
      [
        "internal symlink",
        async () => {
          const nested = path.join(
            bundle.evidenceRoot,
            "nested",
            "collector-output.json",
          );
          await mkdir(path.dirname(nested));
          await copyFile(bundle.paths.collector, nested);
          await rm(bundle.paths.collector);
          await symlink(nested, bundle.paths.collector);
        },
        /contains a symlink/,
      ],
      [
        "external symlink",
        async () => {
          const outside = path.join(bundle.outerRoot, "outside-collector.json");
          await copyFile(bundle.paths.collector, outside);
          await rm(bundle.paths.collector);
          await symlink(outside, bundle.paths.collector);
        },
        /contains a symlink/,
      ],
      [
        "escaping reference",
        async () => {
          const attestation = await readJson(bundle.paths.attestation);
          attestation.collectorOutput.relativePath = "../collector-output.json";
          await writeCanonicalJson(bundle.paths.attestation, attestation);
        },
        /schema failed|path is unsafe/,
      ],
    ]) {
      await restoreArtifacts(snapshot);
      await mutate();
      await assert.rejects(bundle.verify(), expected, label);
    }
  } finally {
    await bundle.cleanup();
  }
});

test("collector and independent byte-count and hash links are exact", async () => {
  const bundle = await createBaselineSetupNoGoBundle();
  const snapshot = await snapshotArtifacts(bundle);
  try {
    for (const [label, mutate, expected] of [
      [
        "collector byte count",
        (attestation) => {
          attestation.collectorOutput.bytes += 1;
        },
        /collector output byte count changed/,
      ],
      [
        "collector hash",
        (attestation) => {
          attestation.collectorOutput.sha256 = HASH_D;
        },
        /collector output bytes changed/,
      ],
      [
        "independent byte count",
        (attestation) => {
          attestation.independentRecompute.bytes += 1;
        },
        /independent recompute byte count changed/,
      ],
      [
        "independent hash",
        (attestation) => {
          attestation.independentRecompute.sha256 = HASH_D;
        },
        /independent recompute bytes changed/,
      ],
    ]) {
      await restoreArtifacts(snapshot);
      const attestation = await readJson(bundle.paths.attestation);
      mutate(attestation);
      await writeCanonicalJson(bundle.paths.attestation, attestation);
      await assert.rejects(bundle.verify(), expected, label);
    }
  } finally {
    await bundle.cleanup();
  }
});

test("stale bytes, malformed schemas, and stale collector links are rejected", async () => {
  const bundle = await createBaselineSetupNoGoBundle();
  const snapshot = await snapshotArtifacts(bundle);
  try {
    await writeFile(
      bundle.paths.collector,
      Buffer.concat([await readFile(bundle.paths.collector), Buffer.from("\n")]),
    );
    await assert.rejects(
      bundle.verify(),
      /collector output byte count changed|collector output bytes changed/,
    );

    await restoreArtifacts(snapshot);
    const collector = await readJson(bundle.paths.collector);
    const reformatted = Buffer.from(`${JSON.stringify(collector)}\n`, "utf8");
    await writeFile(bundle.paths.collector, reformatted);
    const attestation = await readJson(bundle.paths.attestation);
    await updateAttestationReferences(bundle, attestation, { collector: true });
    await assert.rejects(
      bundle.verify(),
      /independent recompute is not bound to the collector bytes/,
    );

    await restoreArtifacts(snapshot);
    const malformed = await readJson(bundle.paths.collector);
    malformed.unexpected = true;
    await writeCanonicalJson(bundle.paths.collector, malformed);
    const malformedAttestation = await readJson(bundle.paths.attestation);
    await updateAttestationReferences(bundle, malformedAttestation, {
      collector: true,
    });
    await assert.rejects(bundle.verify(), /collector output schema failed/);
  } finally {
    await bundle.cleanup();
  }
});

test("schema-valid semantic mutations in each compact artifact are rejected", async () => {
  const bundle = await createBaselineSetupNoGoBundle();
  const snapshot = await snapshotArtifacts(bundle);
  try {
    const attestationOnly = await readJson(bundle.paths.attestation);
    attestationOnly.scenarioResults[0].attempts[0].rawPacket.bytes += 1;
    await writeCanonicalJson(bundle.paths.attestation, attestationOnly);
    await assert.rejects(
      bundle.verify(),
      /attestation scenario results differ from collector output/,
    );

    await restoreArtifacts(snapshot);
    const collectorOnly = await readJson(bundle.paths.collector);
    collectorOnly.scenarioResults[0].attempts[0].rawPacket.bytes += 1;
    await writeCanonicalJson(bundle.paths.collector, collectorOnly);
    const collectorOnlyIndependent =
      await readJson(bundle.paths.independent);
    await rewriteIndependentCollectorLink(
      bundle,
      collectorOnlyIndependent,
    );
    const collectorOnlyAttestation =
      await readJson(bundle.paths.attestation);
    await updateAttestationReferences(bundle, collectorOnlyAttestation, {
      collector: true,
      independent: true,
    });
    await assert.rejects(
      bundle.verify(),
      /attestation scenario results differ from collector output/,
    );

    await restoreArtifacts(snapshot);
    const independentOnly = await readJson(bundle.paths.independent);
    independentOnly.scenarioResults[0].attemptResults[0].failureCode =
      "WEB06_SETUP_FOREGROUND_POSTURE";
    await writeCanonicalJson(bundle.paths.independent, independentOnly);
    const independentOnlyAttestation =
      await readJson(bundle.paths.attestation);
    await updateAttestationReferences(bundle, independentOnlyAttestation, {
      independent: true,
    });
    await assert.rejects(
      bundle.verify(),
      /referenced independent recompute differs from verifier output/,
    );
  } finally {
    await bundle.cleanup();
  }
});

test("independent verifier bytes must be its canonical exact output", async () => {
  const bundle = await createBaselineSetupNoGoBundle();
  try {
    const independent = await readJson(bundle.paths.independent);
    await writeFile(
      bundle.paths.independent,
      Buffer.from(`${JSON.stringify(independent)}\n`, "utf8"),
    );
    const attestation = await readJson(bundle.paths.attestation);
    await updateAttestationReferences(bundle, attestation, {
      independent: true,
    });
    await assert.rejects(
      bundle.verify(),
      /independent recompute bytes are not canonical verifier output/,
    );
  } finally {
    await bundle.cleanup();
  }
});

test("a stale but self-consistently relinked independent recompute is rejected", async () => {
  const bundle = await createBaselineSetupNoGoBundle();
  try {
    const collector = await readJson(bundle.paths.collector);
    const independent = await readJson(bundle.paths.independent);
    const first = collector.scenarioResults[0].attempts[0];
    const rawPath = path.join(
      bundle.evidenceRoot,
      ...first.rawPacket.relativePath.split("/"),
    );
    const envelope = await readJson(rawPath);
    envelope.setupFailure.message =
      "WEB06_SETUP_FOREGROUND_POSTURE:fixture";
    envelope.browserFailure.messageCode =
      "WEB06_SETUP_FOREGROUND_POSTURE";
    envelope.partialAttempt.failure.code =
      "WEB06_SETUP_FOREGROUND_POSTURE";
    const rawBytes = Buffer.from(`${JSON.stringify(envelope)}\n`, "utf8");
    await writeFile(rawPath, rawBytes);
    first.rawPacket.bytes = rawBytes.length;
    first.rawPacket.sha256 = sha256(rawBytes);
    await writeCanonicalJson(bundle.paths.collector, collector);
    await rewriteIndependentCollectorLink(bundle, independent);

    const attestation = await readJson(bundle.paths.attestation);
    attestation.scenarioResults = structuredClone(collector.scenarioResults);
    await updateAttestationReferences(bundle, attestation, {
      collector: true,
      independent: true,
    });
    await assert.rejects(
      bundle.verify(),
      /referenced independent recompute differs from verifier output/,
    );
  } finally {
    await bundle.cleanup();
  }
});

test("mutually forged collector contract values cannot replace the frozen anchor", async () => {
  const bundle = await createBaselineSetupNoGoBundle();
  try {
    const collector = await readJson(bundle.paths.collector);
    const independent = await readJson(bundle.paths.independent);
    await rewriteRawPackets(
      bundle,
      collector,
      independent,
      (envelope) => {
        envelope.target.collectorContractSha256 = HASH_D;
      },
    );
    collector.collectorContractSha256 = HASH_D;
    independent.collectorContractSha256 = HASH_D;
    await writeCanonicalJson(bundle.paths.collector, collector);
    await rewriteIndependentCollectorLink(bundle, independent);

    const attestation = await readJson(bundle.paths.attestation);
    attestation.collectorContractSha256 = HASH_D;
    attestation.scenarioResults = structuredClone(collector.scenarioResults);
    await updateAttestationReferences(bundle, attestation, {
      collector: true,
      independent: true,
    });
    await assert.rejects(
      bundle.verify(),
      /schema failed|collector contract/,
    );
  } finally {
    await bundle.cleanup();
  }
});

test("coherently forged environment identity fails its external source pin", async () => {
  const bundle = await createBaselineSetupNoGoBundle();
  try {
    const collector = await readJson(bundle.paths.collector);
    const independent = await readJson(bundle.paths.independent);
    await rewriteRawPackets(
      bundle,
      collector,
      independent,
      (envelope) => {
        envelope.environmentManifestSha256 = HASH_D;
        envelope.environmentId = HASH_D;
      },
    );
    collector.environmentManifestSha256 = HASH_D;
    collector.environmentId = HASH_D;
    independent.environmentManifestSha256 = HASH_D;
    independent.environmentId = HASH_D;
    await writeCanonicalJson(bundle.paths.collector, collector);
    await rewriteIndependentCollectorLink(bundle, independent);

    const attestation = await readJson(bundle.paths.attestation);
    attestation.environmentManifestSha256 = HASH_D;
    attestation.environmentId = HASH_D;
    attestation.sourceArtifactRoles.runnerSource.environmentManifestSha256 =
      HASH_D;
    attestation.sourceArtifactRoles.runnerSource.environmentId = HASH_D;
    refreshSourceRoleBindings(attestation);
    attestation.scenarioResults = structuredClone(collector.scenarioResults);
    await updateAttestationReferences(bundle, attestation, {
      collector: true,
      independent: true,
    });
    await assert.rejects(
      bundle.verify(),
      /environmentManifestSha256 does not match its source-bound expectation/,
    );
  } finally {
    await bundle.cleanup();
  }
});

test("coherently forged runner tooling identity fails actual clean-source bytes", async () => {
  const bundle = await createBaselineSetupNoGoBundle();
  try {
    const collector = await readJson(bundle.paths.collector);
    const independent = await readJson(bundle.paths.independent);
    collector.runnerSourceManifest.tooling.files[0].sha256 = HASH_D;
    collector.runnerSourceManifest.toolingManifestSha256 =
      digestJson(collector.runnerSourceManifest.tooling);
    collector.runnerSourceManifestSha256 =
      digestJson(collector.runnerSourceManifest);
    const sourceSnapshot = {
      version: "web06-runner-source-observation-v1",
      sourceCommit: collector.runnerSourceManifest.sourceCommit,
      sourceTree: collector.runnerSourceManifest.sourceTree,
      sourceTreeState: collector.runnerSourceManifest.sourceTreeState,
      toolingManifestSha256:
        collector.runnerSourceManifest.toolingManifestSha256,
      files: collector.runnerSourceManifest.tooling.files,
    };
    const sourceObservation = {
      ...sourceSnapshot,
      observationSha256: digestJson(sourceSnapshot),
    };
    collector.runnerSourceObservationSha256 =
      sourceObservation.observationSha256;
    collector.runnerSourcePostObservationSha256 =
      sourceObservation.observationSha256;
    await rewriteRawPackets(
      bundle,
      collector,
      independent,
      (envelope) => {
        envelope.runnerSourceManifestSha256 =
          collector.runnerSourceManifestSha256;
        envelope.runnerSourceBefore = structuredClone(sourceObservation);
        envelope.attemptSourceBefore = structuredClone(sourceObservation);
      },
    );
    await writeCanonicalJson(bundle.paths.collector, collector);
    await rewriteIndependentCollectorLink(bundle, independent);

    const attestation = await readJson(bundle.paths.attestation);
    attestation.runnerSourceManifestSha256 =
      collector.runnerSourceManifestSha256;
    attestation.runnerSourceObservationSha256 =
      sourceObservation.observationSha256;
    attestation.runnerSourcePostObservationSha256 =
      sourceObservation.observationSha256;
    Object.assign(attestation.sourceArtifactRoles.runnerSource, {
      sourceManifestSha256: collector.runnerSourceManifestSha256,
      toolingManifestSha256:
        collector.runnerSourceManifest.toolingManifestSha256,
      beforeObservationSha256: sourceObservation.observationSha256,
      afterObservationSha256: sourceObservation.observationSha256,
    });
    refreshSourceRoleBindings(attestation);
    attestation.scenarioResults = structuredClone(collector.scenarioResults);
    await updateAttestationReferences(bundle, attestation, {
      collector: true,
      independent: true,
    });
    await assert.rejects(
      bundle.verify(),
      /WEB06_INDEPENDENT_RUNNER_FILE_HASH/,
    );
  } finally {
    await bundle.cleanup();
  }
});

test("observer partial/unpaired RED survives and one-artifact triplet mutations fail", async () => {
  const bundle = await createObserverPartialBundle();
  const snapshot = await snapshotArtifacts(bundle);
  try {
    const verified = await bundle.verify();
    assert.equal(
      verified.payload.verdict,
      "SETUP_NO_GO_INSUFFICIENT_VALID_TRIPLETS",
    );
    assert.deepEqual(
      verified.recomputed.observerEvaluation.preservedUnpairedReds,
      ["triplet-attempt-1:minimal"],
    );
    assert.equal(
      verified.payload.observerTriplets[0].minimal.hardRedObserved,
      true,
    );
    assert.equal(
      verified.payload.observerTriplets[0].product.measurementStarted,
      false,
    );

    const attestationOnly = await readJson(bundle.paths.attestation);
    attestationOnly.observerTriplets[0].minimal.rawPacket.bytes += 1;
    await writeCanonicalJson(bundle.paths.attestation, attestationOnly);
    await assert.rejects(
      bundle.verify(),
      /attestation observer triplets differ from collector output/,
    );

    await restoreArtifacts(snapshot);
    const collectorOnly = await readJson(bundle.paths.collector);
    collectorOnly.observerTriplets[0].modeContextIds[0] += "-changed";
    collectorOnly.observerTriplets[0].freshContextId =
      collectorOnly.observerTriplets[0].modeContextIds.join("+");
    await writeCanonicalJson(bundle.paths.collector, collectorOnly);
    const collectorOnlyIndependent =
      await readJson(bundle.paths.independent);
    await rewriteIndependentCollectorLink(
      bundle,
      collectorOnlyIndependent,
    );
    const collectorOnlyAttestation =
      await readJson(bundle.paths.attestation);
    await updateAttestationReferences(bundle, collectorOnlyAttestation, {
      collector: true,
      independent: true,
    });
    await assert.rejects(
      bundle.verify(),
      /attestation observer triplets differ from collector output/,
    );

    await restoreArtifacts(snapshot);
    const independentOnly = await readJson(bundle.paths.independent);
    independentOnly.observerTriplets[0].modeContextIds[0] += "-changed";
    independentOnly.observerTriplets[0].freshContextId =
      independentOnly.observerTriplets[0].modeContextIds.join("+");
    await writeCanonicalJson(bundle.paths.independent, independentOnly);
    const independentOnlyAttestation =
      await readJson(bundle.paths.attestation);
    await updateAttestationReferences(bundle, independentOnlyAttestation, {
      independent: true,
    });
    await assert.rejects(
      bundle.verify(),
      /referenced independent recompute differs from verifier output/,
    );
  } finally {
    await bundle.cleanup();
  }
});
