import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { readFile, readdir, realpath, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "@playwright/test";

import type { ComparatorIdentityManifest, ComparatorStableObservation } from "./startup-benchmark/comparator-endpoint";
import type {
  Web06PeerAttestation,
  Web06PeerAttempt,
  Web06PeerCompactReceipt,
  Web06PeerRawPacket,
  Web06PeerRowId,
  Web06PeerPostRunIntegrity,
} from "./startup-benchmark/web06-peer-lane";
import { independentStableObservationFailures } from "./startup-benchmark/web06-peer-independent.ts";

const rows = [
  {
    id: "phase4-peer/luna-short-ni",
    input: "ni",
    expectedPrefixCount: 2,
    membership: {
      "yune-web": ["你", "擬", "尼", "泥", "呢", "妳"],
      "my-rime": ["你", "擬", "尼", "泥", "呢", "妳"],
    },
    shape: {
      "yune-web": { candidateCount: 6, nextDisabled: false },
      "my-rime": { candidateCount: 6, nextDisabled: false },
    },
  },
  {
    id: "phase4-peer/luna-sustained-59",
    input: "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong",
    expectedPrefixCount: 59,
    membership: {
      "yune-web": ["這個引擎其實應該支持超長句子輸入才能用", "這個", "這歌", "這格", "這"],
      "my-rime": ["這個引擎其實應該支持超長句子輸入才能用", "這個", "這歌", "這格", "這"],
    },
    shape: {
      "yune-web": { candidateCount: 6, nextDisabled: false },
      "my-rime": { candidateCount: 6, nextDisabled: false },
    },
  },
] as const;
const apps = ["yune-web", "my-rime"] as const;
const measurementFailureCodes = new Set([
  "CANDIDATE_ENDPOINT_TIMEOUT", "CANDIDATE_ENDPOINT_SUPERSEDED",
  "COMMIT_ENDPOINT_TIMEOUT", "COMMIT_ENDPOINT_SUPERSEDED",
  "ENDPOINT_OBSERVER_MISSING", "ENDPOINT_OBSERVER_WRONG_APP",
  "ENDPOINT_METRIC_CONTRACT_FAILURE", "EVENT_STREAM_METRIC_CONTRACT_FAILURE",
  "FRAME_OBSERVER_MISSING",
  "FRAME_WINDOW_NOT_STARTED", "FRAME_WINDOW_END_MISSING",
  "FRAME_WINDOW_DURATION_INVALID", "FRAME_IDLE_CONTROL_TIMEOUT",
  "FRAME_BOUNDARY_CONTRACT_FAILURE", "FRAME_RECEIPT_CONTRACT_FAILURE",
  "FRAME_EVALUATION_FAILURE", "PAGE_ENVIRONMENT_LOSS",
  "POST_DISPATCH_METRIC_CONTRACT_FAILURE",
]);
const behaviorMeasurementFailureCodes = new Set([
  "CANDIDATE_ENDPOINT_TIMEOUT", "CANDIDATE_ENDPOINT_SUPERSEDED",
  "COMMIT_ENDPOINT_TIMEOUT", "COMMIT_ENDPOINT_SUPERSEDED",
]);
const expectedInputRegistrySha256 = "c8ee4c5e33b12883271e8d92ade281836d7fa12fc05ddca79c3054b6ac316536";
const expectedSelectorManifestSha256 = "861f078303ca3619ef7bc3f0d2189555da9b87e07a03ba9201b4e98d7fcdb013";
const pinnedPeer = {
  archiveSha256: "a5eea5ebffa1f62e3f4d058117c1405137fddc02c1046e725ee7b4e7c47420ba",
  completeManifestSha256: "bfe733f1c190898a10c22afc53b237232c4d9b3c594c056d832dfc253dd6e1b6",
  treeSha256: "d0dde476677373f24c5cfd335780a0bac753932d0eb52049950f874f0e37e7b3",
  upstreamCommit: "c73ea172d28f07031ba87a1d71c4d2e1c8ba82a3",
} as const;

export interface Web06PeerIndependentVerification {
  version: "web06-phase4-peer-independent-verifier-v1";
  status: "PASS";
  rawPacketSha256: string;
  compactReceiptSha256: string;
  attestationSha256: string;
  negativeEssayControlSha256: string;
  pageSizePreflightSha256: string;
  postRunIntegritySha256: string;
  verifiedSource: {
    yuneCommit: string;
    yuneTree: string;
    yuneArchiveSha256: string;
    yuneCompleteManifestSha256: string;
    peerUpstreamCommit: string;
    peerArtifactSourceCommit: string;
    peerArtifactSourceTree: string;
    peerArchiveSha256: string;
    peerCompleteManifestSha256: string;
  };
  verifiedBindings: {
    toolchainIdentitySha256: string;
    finalRunEnvironmentSha256: string;
    configSha256: string;
    inputRegistrySha256: string;
    selectorManifestSha256: string;
    identityManifestSha256: string;
    attestationSha256: string;
    negativeEssayControlSha256: string;
    pageSizePreflightSha256: string;
    postRunIntegritySha256: string;
    rawPacketSha256: string;
    compactReceiptSha256: string;
  };
  attemptsRecomputed: number;
  groupsRecomputed: number;
  packageAlignment: "DATA_CONFOUNDED";
  ratioStatus: "OMITTED";
  numericRatioFields: 0;
  fullSuccessEligible: false;
  milestoneDisposition: "PENDING_EXPLICIT_PARTIAL_APPROVAL";
  existingIndependentVerificationSha256?: string;
}

export async function verifyWeb06PeerEvidence(
  root: string,
  options: {
    writeIndependentVerification?: boolean;
    requireExistingIndependentVerification?: boolean;
  } = {},
): Promise<Web06PeerIndependentVerification> {
  if (options.writeIndependentVerification && options.requireExistingIndependentVerification) {
    throw new Error("Independent verifier write and verify-only modes are mutually exclusive");
  }
  const outputRoot = path.resolve(root);
  const attestationFile = path.join(outputRoot, "raw", "attestation.json");
  const negativeEssayFile = path.join(outputRoot, "raw", "negative-essay-control.json");
  const pageSizeFile = path.join(outputRoot, "raw", "page-size-preflight.json");
  const postRunFile = path.join(outputRoot, "raw", "post-run-integrity.json");
  const rawFile = path.join(outputRoot, "raw", "peer-raw-packet.json");
  const compactFile = path.join(outputRoot, "compact", "peer-summary.json");
  const [attestationText, negativeEssayText, pageSizeText, postRunText, rawText, compactText] = await Promise.all([
    readFile(attestationFile, "utf8"),
    readFile(negativeEssayFile, "utf8"),
    readFile(pageSizeFile, "utf8"),
    readFile(postRunFile, "utf8"),
    readFile(rawFile, "utf8"),
    readFile(compactFile, "utf8"),
  ]);
  const attestation = parseCanonical<Web06PeerAttestation & {
    liveEnvironmentDrift?: unknown;
    sourceMismatch?: unknown;
  }>(attestationText, "attestation");
  const packet = parseCanonical<Web06PeerRawPacket>(rawText, "raw packet");
  const postRun = parseCanonical<Web06PeerPostRunIntegrity>(postRunText, "post-run integrity");
  const actualCompact = parseCanonical<Web06PeerCompactReceipt>(compactText, "compact receipt");

  verifyTopLevel(attestation, packet, attestationText);
  verifyAuxiliaryReceipts(attestation, attestationText, negativeEssayText, pageSizeText);
  await verifyPostRunIntegrity(attestation, attestationText, postRun);
  await verifyEvidenceFileSet(outputRoot, options.requireExistingIndependentVerification === true);
  const attemptDigests = await verifyAttemptFiles(outputRoot, packet.attempts);
  const expectedCompact = recomputeCompact(packet, attestation, digest(rawText), attemptDigests);
  const expectedText = canonical(expectedCompact);
  if (compactText !== expectedText) throw new Error("Independent compact recomputation is not byte-identical");
  verifyNoNumericRatio(actualCompact);
  verifyCompactIdentity(actualCompact);
  verifyPrivacy(actualCompact);
  const verification: Web06PeerIndependentVerification = {
    version: "web06-phase4-peer-independent-verifier-v1",
    status: "PASS",
    rawPacketSha256: digest(rawText),
    compactReceiptSha256: digest(compactText),
    attestationSha256: digest(attestationText),
    negativeEssayControlSha256: digest(negativeEssayText),
    pageSizePreflightSha256: digest(pageSizeText),
    postRunIntegritySha256: digest(postRunText),
    verifiedSource: {
      yuneCommit: attestation.yune.sourceCommit,
      yuneTree: attestation.yune.sourceTree,
      yuneArchiveSha256: attestation.yune.archiveSha256,
      yuneCompleteManifestSha256: attestation.yune.completeManifestSha256,
      peerUpstreamCommit: attestation.identityManifest.peer.upstreamPinnedCommit,
      peerArtifactSourceCommit: attestation.identityManifest.peer.artifactSourceCommit,
      peerArtifactSourceTree: attestation.identityManifest.peer.artifactSourceTree,
      peerArchiveSha256: attestation.peer.archiveSha256,
      peerCompleteManifestSha256: attestation.peer.completeManifestSha256,
    },
    verifiedBindings: {
      toolchainIdentitySha256: digest(canonical(attestation.toolchain)),
      finalRunEnvironmentSha256: attestation.finalRunEnvironmentSha256,
      configSha256: attestation.configSha256,
      inputRegistrySha256: attestation.inputRegistrySha256,
      selectorManifestSha256: attestation.selectorManifestSha256,
      identityManifestSha256: attestation.identityManifestSha256,
      attestationSha256: digest(attestationText),
      negativeEssayControlSha256: digest(negativeEssayText),
      pageSizePreflightSha256: digest(pageSizeText),
      postRunIntegritySha256: digest(postRunText),
      rawPacketSha256: digest(rawText),
      compactReceiptSha256: digest(compactText),
    },
    attemptsRecomputed: packet.attempts.length,
    groupsRecomputed: expectedCompact.rows.length,
    packageAlignment: "DATA_CONFOUNDED",
    ratioStatus: "OMITTED",
    numericRatioFields: 0,
    fullSuccessEligible: false,
    milestoneDisposition: "PENDING_EXPLICIT_PARTIAL_APPROVAL",
  };
  const verificationFile = path.join(outputRoot, "compact", "independent-verification.json");
  if (options.writeIndependentVerification) {
    await writeFile(verificationFile, canonical(verification), { flag: "wx", mode: 0o600 });
  }
  if (options.requireExistingIndependentVerification) {
    const existing = await readFile(verificationFile, "utf8");
    if (existing !== canonical(verification)) {
      throw new Error("Existing independent verification is not canonical or byte-identical to recomputation");
    }
    return {
      ...verification,
      existingIndependentVerificationSha256: digest(existing),
    };
  }
  return verification;
}

const invokedDirectly = process.argv[1] !== undefined
  && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  validateCliArguments(process.argv.slice(2));
  const verifyOnly = process.argv.includes("--verify-only");
  const verification = await verifyWeb06PeerEvidence(argument("--output-root"), {
    writeIndependentVerification: !verifyOnly,
    requireExistingIndependentVerification: verifyOnly,
  });
  process.stdout.write(`${JSON.stringify(verification)}\n`);
}

function parseCanonical<T>(text: string, label: string): T {
  const value = JSON.parse(text) as T;
  if (text !== canonical(value)) throw new Error(`${label} is not canonical JSON`);
  return value;
}

async function verifyEvidenceFileSet(root: string, requireIndependent: boolean): Promise<void> {
  const rawEntries = await readdir(path.join(root, "raw"), { withFileTypes: true });
  const rawNames = rawEntries.map(entry => `${entry.isDirectory() ? "d" : "f"}:${entry.name}`).sort();
  const expectedRaw = [
    "d:attempts",
    "f:attestation.json",
    "f:negative-essay-control.json",
    "f:page-size-preflight.json",
    "f:post-run-integrity.json",
    "f:peer-raw-packet.json",
  ].sort();
  if (canonical(rawNames) !== canonical(expectedRaw)) {
    throw new Error("Raw evidence file set is incomplete or contains extras");
  }
  const compactNames = (await readdir(path.join(root, "compact"))).sort();
  const expectedCompact = requireIndependent
    ? ["independent-verification.json", "peer-summary.json"]
    : ["peer-summary.json"];
  if (canonical(compactNames) !== canonical(expectedCompact.sort())) {
    throw new Error("Compact evidence file set is incomplete or contains extras");
  }
}

function verifyAuxiliaryReceipts(
  attestation: Web06PeerAttestation,
  attestationText: string,
  negativeEssayText: string,
  pageSizeText: string,
): void {
  if (digest(negativeEssayText) !== attestation.negativeEssayControlSha256) {
    throw new Error("Negative essay control hash is not attestation-bound");
  }
  const negative = parseCanonical<ComparatorIdentityManifest>(negativeEssayText, "negative essay control");
  if (negative.version !== "web06-peer-data-v1"
      || !independentlyConfounded(negative).includes("logical-input-different:essay")) {
    throw new Error("Negative essay control does not independently refuse alignment");
  }
  const pageSize = parseCanonical<Record<string, unknown>>(pageSizeText, "page-size preflight");
  const expectedKeys = [
    "attestationSha256", "benchmarkAttempt", "environmentId", "peerArchiveSha256",
    "pinnedPeerSelector", "probes", "sourceKey", "status", "version", "yuneArchiveSha256",
  ].sort();
  if (canonical(Object.keys(pageSize).sort()) !== canonical(expectedKeys)
      || pageSize.version !== "web06-phase4-peer-page-size-preflight-v1"
      || pageSize.benchmarkAttempt !== false || pageSize.sourceKey !== "pageSize"
      || pageSize.pinnedPeerSelector !== "my-rime-c73ea17-public-dom-v1"
      || pageSize.status !== "PASS"
      || pageSize.attestationSha256 !== digest(attestationText)
      || pageSize.yuneArchiveSha256 !== attestation.yune.archiveSha256
      || pageSize.peerArchiveSha256 !== attestation.peer.archiveSha256
      || pageSize.environmentId !== attestation.finalRunEnvironment.environmentId) {
    throw new Error("Page-size preflight identity/config contract failed");
  }
  const probes = pageSize.probes;
  if (!Array.isArray(probes) || probes.length !== 2) {
    throw new Error("Page-size preflight must contain exactly two app probes");
  }
  for (const [index, app] of apps.entries()) {
    const probe = probes[index] as Record<string, unknown> | undefined;
    const expectedArchive = app === "yune-web" ? attestation.yune.archiveSha256 : attestation.peer.archiveSha256;
    const expectedProbeKeys = [
      "app", "benchmarkAttempt", "blockedSetupNetworkRequests", "commitClearedComposition",
      "committedValuePresent", "finalCandidateRows", "finalCandidateSnapshots", "networkRouteInstalled",
      "pageSizeSetup", "pageVisibleAndFocused", "prefixesObserved", "selectorManifestId",
      "sourceArchiveSha256", "unexpectedNetworkRequestCount",
    ].sort();
    if (!probe || canonical(Object.keys(probe).sort()) !== canonical(expectedProbeKeys)
        || probe.app !== app || probe.benchmarkAttempt !== false
        || probe.sourceArchiveSha256 !== expectedArchive || probe.finalCandidateRows !== 6
        || probe.prefixesObserved !== 2 || probe.commitClearedComposition !== true
        || probe.committedValuePresent !== true || probe.pageVisibleAndFocused !== true
        || probe.networkRouteInstalled !== true
        || probe.unexpectedNetworkRequestCount !== 0) {
      throw new Error(`Page-size preflight probe failed for ${app}`);
    }
    const expectedSelector = app === "yune-web"
      ? "yune-web-public-dom-v1"
      : "my-rime-c73ea17-public-dom-v1";
    if (probe.selectorManifestId !== expectedSelector
        || !Array.isArray(probe.blockedSetupNetworkRequests)
        || !isRecord(probe.pageSizeSetup)
        || probe.pageSizeSetup.sourceKey !== "pageSize") {
      throw new Error(`Page-size preflight source/selector contract failed for ${app}`);
    }
    verifyPreflightCandidateSnapshots(probe.finalCandidateSnapshots, app, expectedSelector);
    if (app === "yune-web") {
      const measurement = probe.pageSizeSetup.yuneMeasurementPage;
      if (!isRecord(measurement) || !isRecord(measurement.final)
          || measurement.final.uiValue !== "6" || measurement.final.localStorageValue !== "6"
          || measurement.final.persistedConfigValue !== "6" || measurement.final.deployStatus !== "success"
          || measurement.final.loadingComplete !== true || !Array.isArray(measurement.actions)
          || measurement.actions.length !== 2) {
        throw new Error("Yune page-size preflight did not prove final UI/storage/config/runtime posture");
      }
      const actions = measurement.actions as Array<Record<string, unknown>>;
      if (actions[0]?.targetUiValue !== "7" || actions[1]?.targetUiValue !== "6") {
        throw new Error("Yune page-size preflight omitted the 6->7->6 transition proof");
      }
    } else if (probe.pageSizeSetup.myRimeLocalStorageValue !== "6"
        || probe.pageSizeSetup.proof !== "actual measured candidate endpoints require exactly six visible rows") {
      throw new Error("My RIME page-size preflight did not prove local storage plus actual six-row endpoints");
    }
  }
}

export function verifyPreflightCandidateSnapshots(
  value: unknown,
  app: (typeof apps)[number],
  expectedSelector: string,
): void {
  if (!Array.isArray(value) || value.length !== rows.length) {
    throw new Error(`Page-size preflight must contain both frozen row snapshots for ${app}`);
  }
  for (const [index, row] of rows.entries()) {
    const snapshot = value[index];
    const expectedKeys = [
      "candidateCount", "candidateSurfaceCount", "candidates", "composition",
      "expectedPrefixCount", "highlightedIndex", "observedInputEventCount", "page",
      "rowId", "schemaId", "selectorManifestId",
    ].sort();
    if (!isRecord(snapshot)
        || canonical(Object.keys(snapshot).sort()) !== canonical(expectedKeys)
        || snapshot.rowId !== row.id
        || snapshot.expectedPrefixCount !== row.expectedPrefixCount
        || snapshot.observedInputEventCount !== row.expectedPrefixCount
        || snapshot.selectorManifestId !== expectedSelector
        || snapshot.composition !== row.input
        || snapshot.schemaId !== "luna_pinyin"
        || snapshot.candidateSurfaceCount !== 1
        || !Number.isInteger(snapshot.highlightedIndex)) {
      throw new Error(`Source-pinned preflight row identity/schema failed for ${row.id}/${app}`);
    }
    const candidates = snapshot.candidates;
    const expectedShape = row.shape[app];
    if (!Array.isArray(candidates)
        || snapshot.candidateCount !== expectedShape.candidateCount
        || candidates.length !== expectedShape.candidateCount
        || (snapshot.highlightedIndex as number) < 0
        || (snapshot.highlightedIndex as number) >= candidates.length) {
      throw new Error(`Source-pinned preflight candidate shape failed for ${row.id}/${app}`);
    }
    const texts: string[] = [];
    for (const [candidateIndex, candidate] of candidates.entries()) {
      if (!isRecord(candidate)
          || canonical(Object.keys(candidate).sort()) !== canonical(["comment", "label", "text"])
          || typeof candidate.label !== "string"
          || typeof candidate.text !== "string"
          || typeof candidate.comment !== "string"
          || candidate.text === ""
          || candidate.label.replace(/[.\s]/g, "") !== String(candidateIndex + 1)) {
        throw new Error(`Source-pinned preflight candidate tuple failed for ${row.id}/${app}`);
      }
      texts.push(candidate.text);
    }
    if (!row.membership[app].every(text => texts.includes(text))) {
      throw new Error(`Source-pinned preflight membership failed for ${row.id}/${app}`);
    }
    const page = snapshot.page;
    if (!isRecord(page)
        || canonical(Object.keys(page).sort()) !== canonical([
          "buttonCount", "index", "nextDisabled", "previousDisabled",
        ])
        || page.index !== 0 || page.buttonCount !== 2
        || page.previousDisabled !== true
        || page.nextDisabled !== expectedShape.nextDisabled) {
      throw new Error(`Source-pinned preflight page evidence failed for ${row.id}/${app}`);
    }
  }
}

function verifyTopLevel(
  receipt: Web06PeerAttestation & { liveEnvironmentDrift?: unknown; sourceMismatch?: unknown },
  raw: Web06PeerRawPacket,
  receiptText: string,
): void {
  if (receipt.version !== "web06-phase4-peer-attestation-v1"
      || receipt.phase !== "phase4-peer"
      || receipt.mode !== "binding"
      || !receipt.benchmarkAttempt) {
    throw new Error("Independent verifier refuses a nonbinding attestation");
  }
  if (!Array.isArray(receipt.liveEnvironmentDrift) || receipt.liveEnvironmentDrift.length !== 0
      || !Array.isArray(receipt.sourceMismatch) || receipt.sourceMismatch.length !== 0
      || receipt.toolchain.runnerSourceTreeState !== "clean") {
    throw new Error("Binding attestation contains environment/source drift or a provisional runner");
  }
  const sha40 = /^[0-9a-f]{40}$/;
  const sha64 = /^[0-9a-f]{64}$/;
  const artifactIdentities = [receipt.yune, receipt.peer];
  if (artifactIdentities.some(identity => !sha64.test(identity.archiveSha256)
    || !sha64.test(identity.completeManifestSha256) || !sha64.test(identity.treeSha256)
    || !sha40.test(identity.sourceCommit) || !sha40.test(identity.sourceTree)
    || identity.sourceTreeState !== "clean" || !sha64.test(identity.wasmSha256)
    || !Number.isInteger(identity.fileCount) || identity.fileCount < 1)
      || !sha64.test(receipt.yune.buildInfoSha256 ?? "")
      || !sha64.test(receipt.yune.schemaManifestSha256 ?? "")
      || !sha40.test(receipt.toolchain.runnerSourceCommit)
      || !sha40.test(receipt.toolchain.runnerSourceTree)) {
    throw new Error("Binding attestation contains malformed source/artifact identities");
  }
  const toolchainHashes = [
    receipt.toolchain.playwrightPackageLockSha256,
    receipt.toolchain.chromiumExecutableSha256,
    receipt.toolchain.endpointSourceSha256,
    receipt.toolchain.browserEndpointSourceSha256,
    receipt.toolchain.laneSourceSha256,
    receipt.toolchain.artifactAttestorSourceSha256,
    receipt.toolchain.launcherSourceSha256,
    receipt.toolchain.independentLogicSourceSha256,
    receipt.toolchain.specSourceSha256,
    receipt.toolchain.configSourceSha256,
    receipt.toolchain.verifierSourceSha256,
  ];
  if (toolchainHashes.some(value => !sha64.test(value))
      || receipt.toolchain.runnerSourceCommit !== receipt.yune.sourceCommit
      || receipt.toolchain.runnerSourceTree !== receipt.yune.sourceTree) {
    throw new Error("Phase-4 runner source is not exactly equal to the FINAL Yune artifact source");
  }
  if (receipt.peer.archiveSha256 !== pinnedPeer.archiveSha256
      || receipt.peer.completeManifestSha256 !== pinnedPeer.completeManifestSha256
      || receipt.peer.treeSha256 !== pinnedPeer.treeSha256
      || receipt.identityManifest.peer.upstreamPinnedCommit !== pinnedPeer.upstreamCommit) {
    throw new Error("Binding attestation does not use the frozen pinned My RIME artifact/source");
  }
  if (receipt.identityManifestSha256 !== digest(canonical(receipt.identityManifest))
      || receipt.inputRegistrySha256 !== expectedInputRegistrySha256
      || receipt.selectorManifestSha256 !== expectedSelectorManifestSha256
      || receipt.identityManifest.yune.artifactSourceCommit !== receipt.yune.sourceCommit
      || receipt.identityManifest.yune.artifactSourceTree !== receipt.yune.sourceTree
      || receipt.identityManifest.yune.artifactSha256 !== receipt.yune.archiveSha256
      || receipt.identityManifest.yune.completeArtifactManifestSha256 !== receipt.yune.completeManifestSha256
      || receipt.identityManifest.yune.generatedManifestSha256 !== receipt.yune.schemaManifestSha256
      || receipt.identityManifest.yune.compiledHashes.runtime !== receipt.yune.wasmSha256
      || receipt.identityManifest.peer.artifactSourceCommit !== receipt.peer.sourceCommit
      || receipt.identityManifest.peer.artifactSourceTree !== receipt.peer.sourceTree
      || receipt.identityManifest.peer.artifactSha256 !== receipt.peer.archiveSha256
      || receipt.identityManifest.peer.completeArtifactManifestSha256 !== receipt.peer.completeManifestSha256
      || receipt.identityManifest.peer.compiledHashes.runtime !== receipt.peer.wasmSha256) {
    throw new Error("Binding source/artifact/logical-input identities do not reconcile");
  }
  if (receipt.extraction.yune !== "SAFE_EXTRACTED_AND_FULLY_RECONCILED"
      || receipt.extraction.peer !== "SAFE_EXTRACTED_AND_FULLY_RECONCILED"
      || !path.isAbsolute(receipt.runnerRoot) || !path.isAbsolute(receipt.outputRoot)
      || !path.isAbsolute(receipt.yuneArchivePath) || !path.isAbsolute(receipt.peerArchivePath)
      || !path.isAbsolute(receipt.yuneRoot) || !path.isAbsolute(receipt.peerRoot)
      || !path.isAbsolute(receipt.peerPacketRoot)) {
    throw new Error("Binding extraction/path attestation is incomplete");
  }
  if (raw.version !== "web06-phase4-peer-raw-v1"
      || raw.phase !== "phase4-peer"
      || !raw.benchmarkAttempt
      || raw.packageAlignment !== "DATA_CONFOUNDED") {
    throw new Error("Independent verifier refuses the raw packet contract");
  }
  if (raw.attestationSha256 !== digest(receiptText)
      || raw.configSha256 !== receipt.configSha256
      || raw.identityManifestSha256 !== receipt.identityManifestSha256) {
    throw new Error("Raw packet is not bound to its exact attestation/config/identity");
  }
  verifyGlobalRunOrder(raw.attempts);
  const environmentText = canonical(receipt.finalRunEnvironment);
  const { environmentId, ...environmentWithoutId } = receipt.finalRunEnvironment;
  if (environmentId !== digest(canonical(environmentWithoutId))) {
    throw new Error("Independent FINAL environmentId recomputation failed");
  }
  if (receipt.finalRunEnvironment.browserMode !== "headed-foreground") {
    throw new Error("Independent verifier rejects a non-foreground peer browser mode");
  }
  if (receipt.host.powerSource !== "AC Power" || receipt.host.lowPowerMode !== false
      || receipt.host.display.refreshRateHz !== 60
      || canonical(receipt.finalRunEnvironment.viewport) !== canonical({ width: 1365, height: 900 })
      || receipt.finalRunEnvironment.locale !== "zh-HK"
      || receipt.finalRunEnvironment.cacheRegime !== "fresh-persistent-profile-no-store-v1") {
    throw new Error("Independent verifier rejects non-AC/60Hz/frozen viewport-locale-cache evidence");
  }
  if (receipt.finalRunEnvironmentSha256 !== digest(environmentText)) {
    throw new Error("Attested FINAL environment file hash is not canonical-byte identity");
  }
  if (receipt.finalRunEnvironment.nodeVersion !== receipt.toolchain.nodeVersion
      || receipt.finalRunEnvironment.playwrightVersion !== receipt.toolchain.playwrightVersion
      || receipt.finalRunEnvironment.chromiumVersion !== receipt.toolchain.chromiumVersion
      || receipt.finalRunEnvironment.chromiumExecutableSha256 !== receipt.toolchain.chromiumExecutableSha256
      || canonical(receipt.finalRunEnvironment.host) !== canonical(receipt.host)) {
    throw new Error("Attested FINAL environment/toolchain/host identity is inconsistent");
  }
}

async function verifyPostRunIntegrity(
  attestation: Web06PeerAttestation,
  attestationText: string,
  postRun: Web06PeerPostRunIntegrity,
): Promise<void> {
  const verifierFile = fileURLToPath(import.meta.url);
  const e2eRoot = path.dirname(verifierFile);
  const repoRoot = path.resolve(e2eRoot, "../../..");
  if (await realpath(attestation.runnerRoot) !== await realpath(repoRoot)) {
    throw new Error("Independent verifier is not executing from the attested runner root");
  }
  const git = (...args: string[]) => execFileSync(
    "git", args, { cwd: repoRoot, encoding: "utf8" },
  ).trim();
  const runnerSourceCommit = git("rev-parse", "HEAD");
  const runnerSourceTree = git("rev-parse", "HEAD^{tree}");
  if (git("status", "--porcelain=v1", "--untracked-files=all") !== ""
      || runnerSourceCommit !== attestation.yune.sourceCommit
      || runnerSourceTree !== attestation.yune.sourceTree) {
    throw new Error("Independent post-run runner HEAD/tree/clean check failed");
  }
  const sourceFiles = {
    endpointSourceSha256: path.join(e2eRoot, "startup-benchmark", "comparator-endpoint.ts"),
    browserEndpointSourceSha256: path.join(e2eRoot, "startup-benchmark", "comparator-browser-endpoint.ts"),
    laneSourceSha256: path.join(e2eRoot, "startup-benchmark", "web06-peer-lane.ts"),
    artifactAttestorSourceSha256: path.join(e2eRoot, "startup-benchmark", "web06-peer-artifacts.ts"),
    launcherSourceSha256: path.join(e2eRoot, "run-web06-peer-lane.ts"),
    independentLogicSourceSha256: path.join(e2eRoot, "startup-benchmark", "web06-peer-independent.ts"),
    specSourceSha256: path.join(e2eRoot, "web06-peer-phase4.spec.ts"),
    configSourceSha256: path.join(e2eRoot, "playwright.web06-peer.config.ts"),
    verifierSourceSha256: verifierFile,
  } as const;
  for (const [field, file] of Object.entries(sourceFiles)) {
    if (await fileDigest(file) !== attestation.toolchain[field as keyof typeof sourceFiles]) {
      throw new Error(`Independent runner source hash changed after measurement: ${field}`);
    }
  }
  const executable = chromium.executablePath();
  const playwrightPackage = JSON.parse(await readFile(
    path.join(e2eRoot, "node_modules", "@playwright", "test", "package.json"),
    "utf8",
  )) as { version: string };
  const toolchain = {
    runnerSourceCommit,
    runnerSourceTree,
    runnerSourceTreeState: "clean" as const,
    nodeVersion: process.version,
    npmVersion: execFileSync("npm", ["--version"], { encoding: "utf8" }).trim(),
    playwrightVersion: playwrightPackage.version,
    playwrightPackageLockSha256: await fileDigest(path.join(e2eRoot, "package-lock.json")),
    chromiumVersion: execFileSync(executable, ["--version"], { encoding: "utf8" }).trim(),
    chromiumExecutableSha256: await fileDigest(executable),
    ...Object.fromEntries(await Promise.all(Object.entries(sourceFiles).map(async ([field, file]) =>
      [field, await fileDigest(file)]
    ))),
  };
  if (canonical(toolchain) !== canonical(attestation.toolchain)) {
    throw new Error("Independent post-run toolchain/source snapshot differs from the pre-run attestation");
  }
  if (await fileDigest(attestation.yuneArchivePath) !== attestation.yune.archiveSha256
      || await fileDigest(attestation.peerArchivePath) !== attestation.peer.archiveSha256) {
    throw new Error("Independent post-run archive hash check failed");
  }
  const yuneIdentity = await independentlyVerifyYuneRoot(attestation);
  const peerIdentity = await independentlyVerifyPeerRoot(attestation);
  const expected: Web06PeerPostRunIntegrity = {
    version: "web06-phase4-peer-post-run-integrity-v1",
    benchmarkAttempt: true,
    attestationSha256: digest(attestationText),
    runnerSourceCommit,
    runnerSourceTree,
    runnerSourceTreeState: "clean",
    toolchainIdentitySha256: digest(canonical(toolchain)),
    yuneArtifactIdentitySha256: digest(canonical(yuneIdentity)),
    peerArtifactIdentitySha256: digest(canonical(peerIdentity)),
    finalRunEnvironmentId: attestation.finalRunEnvironment.environmentId,
  };
  if (canonical(postRun) !== canonical(expected)) {
    throw new Error("Independent post-run source/artifact integrity receipt recomputation failed");
  }
}

async function independentlyVerifyYuneRoot(attestation: Web06PeerAttestation): Promise<unknown> {
  const root = attestation.yuneRoot;
  const buildInfoFile = path.join(root, "build-info.json");
  const manifestFile = path.join(root, "public-artifact-manifest.json");
  if (await fileDigest(buildInfoFile) !== attestation.yune.buildInfoSha256
      || await fileDigest(manifestFile) !== attestation.yune.completeManifestSha256) {
    throw new Error("Independent post-run Yune build-info/manifest identity failed");
  }
  const buildInfo = JSON.parse(await readFile(buildInfoFile, "utf8")) as {
    sourceCommit: string;
    sourceTreeState: string;
    schemaManifestSha256: string;
    wasmSha256: string;
    publicArtifactManifestSha256: string;
  };
  const manifest = JSON.parse(await readFile(manifestFile, "utf8")) as {
    generatedFor: string;
    files: Array<{ path: string; bytes: number; sha256: string }>;
  };
  if (buildInfo.sourceCommit !== attestation.yune.sourceCommit || buildInfo.sourceTreeState !== "clean"
      || buildInfo.publicArtifactManifestSha256 !== attestation.yune.completeManifestSha256
      || manifest.generatedFor !== "yune-web" || !Array.isArray(manifest.files)) {
    throw new Error("Independent post-run Yune manifest source contract failed");
  }
  const expected = new Map(manifest.files.map(item => [safeRelative(item.path), item]));
  if (expected.size !== manifest.files.length) throw new Error("Independent Yune manifest contains duplicates");
  const files = await independentRegularFiles(root);
  const allowedExtra = new Set(["build-info.json", "public-artifact-manifest.json"]);
  if (files.some(file => !expected.has(file) && !allowedExtra.has(file))
      || [...expected.keys()].some(file => !files.includes(file))) {
    throw new Error("Independent post-run Yune full file reconciliation failed");
  }
  for (const [relative, item] of expected) {
    const info = await stat(path.join(root, relative));
    if (info.size !== item.bytes || await fileDigest(path.join(root, relative)) !== item.sha256) {
      throw new Error(`Independent post-run Yune member changed: ${relative}`);
    }
  }
  if (await fileDigest(path.join(root, "schema-asset-manifest.json")) !== buildInfo.schemaManifestSha256
      || await fileDigest(path.join(root, "yune-web.wasm")) !== buildInfo.wasmSha256) {
    throw new Error("Independent post-run Yune schema/WASM identity failed");
  }
  const identity = {
    archiveSha256: attestation.yune.archiveSha256,
    completeManifestSha256: attestation.yune.completeManifestSha256,
    treeSha256: await independentTreeDigest(root, files),
    fileCount: files.length,
    sourceCommit: buildInfo.sourceCommit,
    sourceTree: attestation.yune.sourceTree,
    sourceTreeState: "clean",
    buildInfoSha256: attestation.yune.buildInfoSha256,
    schemaManifestSha256: buildInfo.schemaManifestSha256,
    wasmSha256: buildInfo.wasmSha256,
  };
  if (canonical(identity) !== canonical(attestation.yune)) {
    throw new Error("Independent post-run Yune artifact identity does not reconcile");
  }
  return identity;
}

async function independentlyVerifyPeerRoot(attestation: Web06PeerAttestation): Promise<unknown> {
  const manifestFile = path.join(attestation.peerPacketRoot, "artifact-manifest.json");
  if (await fileDigest(manifestFile) !== attestation.peer.completeManifestSha256) {
    throw new Error("Independent post-run peer artifact manifest changed");
  }
  const manifest = JSON.parse(await readFile(manifestFile, "utf8")) as {
    manifestVersion: number;
    treeSha256: string;
    fileCount: number;
    files: Array<{ path: string; size: number; mode: string; sha256: string }>;
  };
  const expected = new Map(manifest.files.map(item => [safeRelative(item.path), item]));
  const files = await independentRegularFiles(attestation.peerRoot);
  if (manifest.manifestVersion !== 1 || manifest.treeSha256 !== attestation.peer.treeSha256
      || expected.size !== manifest.fileCount || files.length !== manifest.fileCount
      || files.some(file => !expected.has(file))
      || [...expected.keys()].some(file => !files.includes(file))) {
    throw new Error("Independent post-run peer full file-list reconciliation failed");
  }
  const rows: string[] = [];
  for (const [relative, item] of expected) {
    const file = path.join(attestation.peerRoot, relative);
    const info = await stat(file);
    const mode = (info.mode & 0o777).toString(8);
    const sha256 = await fileDigest(file);
    if (info.size !== item.size || mode !== item.mode || sha256 !== item.sha256) {
      throw new Error(`Independent post-run peer member changed: ${relative}`);
    }
    rows.push(`${relative}\0${item.size}\0${item.mode}\0${item.sha256}\n`);
  }
  if (digest(rows.sort().join("")) !== manifest.treeSha256) {
    throw new Error("Independent post-run peer tree digest failed");
  }
  const identity = {
    archiveSha256: attestation.peer.archiveSha256,
    completeManifestSha256: attestation.peer.completeManifestSha256,
    treeSha256: manifest.treeSha256,
    fileCount: manifest.fileCount,
    sourceCommit: attestation.identityManifest.peer.artifactSourceCommit,
    sourceTree: attestation.identityManifest.peer.artifactSourceTree,
    sourceTreeState: "clean",
    wasmSha256: attestation.identityManifest.peer.compiledHashes.runtime,
  };
  if (canonical(identity) !== canonical(attestation.peer)) {
    throw new Error("Independent post-run peer artifact identity does not reconcile");
  }
  return identity;
}

async function independentRegularFiles(root: string): Promise<string[]> {
  const files: string[] = [];
  const visit = async (directory: string): Promise<void> => {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const file = path.join(directory, entry.name);
      if (entry.isDirectory()) await visit(file);
      else if (entry.isFile()) files.push(path.relative(root, file).split(path.sep).join("/"));
      else throw new Error(`Independent artifact walk rejected a nonregular entry: ${file}`);
    }
  };
  await visit(root);
  return files.sort();
}

async function independentTreeDigest(root: string, files: string[]): Promise<string> {
  const rows: string[] = [];
  for (const relative of files) {
    const file = path.join(root, relative);
    const info = await stat(file);
    rows.push(`${relative}\0${info.size}\0${(info.mode & 0o777).toString(8)}\0${await fileDigest(file)}\n`);
  }
  return digest(rows.sort().join(""));
}

function safeRelative(value: string): string {
  if (!value || value.includes("\\") || path.posix.isAbsolute(value)
      || path.posix.normalize(value) !== value || value === ".." || value.startsWith("../")) {
    throw new Error(`Independent artifact manifest contains an unsafe path: ${value}`);
  }
  return value;
}

async function fileDigest(file: string): Promise<string> {
  return digest(await readFile(file));
}

async function verifyAttemptFiles(
  root: string,
  attempts: Web06PeerAttempt[],
): Promise<Map<string, string>> {
  const directory = path.join(root, "raw", "attempts");
  const names = (await readdir(directory)).sort();
  const expectedNames = attempts.map(attempt => `${key(attempt)}.json`).sort();
  if (canonical(names) !== canonical(expectedNames)) throw new Error("Raw attempt file set is incomplete or contains extras");
  const digests = new Map<string, string>();
  for (const attempt of attempts) {
    const text = await readFile(path.join(directory, `${key(attempt)}.json`), "utf8");
    if (text !== canonical(attempt)) throw new Error(`${key(attempt)} is not canonical or differs from aggregate raw`);
    validateAttemptIndependently(attempt);
    digests.set(key(attempt), digest(text));
  }
  return digests;
}

export function validateAttemptIndependently(attempt: Web06PeerAttempt): void {
  const row = rows.find(item => item.id === attempt.rowId);
  if (!row) throw new Error(`${key(attempt)} has an unknown row`);
  if (attempt.version !== "web06-phase4-peer-raw-v1"
      || !attempt.benchmarkAttempt
      || attempt.attempt < 1 || attempt.attempt > 7
      || !attempt.freshProfile || !attempt.firstKeyIncluded || attempt.warmupKeyCount !== 0
      || attempt.viewport.width !== 1365 || attempt.viewport.height !== 900
      || attempt.locale !== "zh-HK"
      || attempt.cacheRegime !== "fresh-persistent-profile-no-store-v1"
      || attempt.cadenceMs !== 60 || attempt.pageSize !== 6
      || typeof attempt.networkRouteInstalled !== "boolean") {
    throw new Error(`${key(attempt)} violates immutable run configuration`);
  }
  if ((attempt.measurementStarted && (attempt.pretypingIdleMedianMs === null
      || !Number.isFinite(attempt.pretypingIdleMedianMs) || attempt.pretypingIdleMedianMs <= 0))
      || (attempt.pretypingIdleMedianMs !== null
        && (!Number.isFinite(attempt.pretypingIdleMedianMs) || attempt.pretypingIdleMedianMs <= 0))) {
    throw new Error(`${key(attempt)} has an invalid pretyping idle median`);
  }
  validateMeasurementFailuresIndependently(attempt, row.expectedPrefixCount);
  const eventStreamFailure = attempt.measurementFailures.some(failure =>
    failure.stage === "post-dispatch" && failure.code === "EVENT_STREAM_METRIC_CONTRACT_FAILURE"
  );
  if (attempt.observedEvents === null) {
    if (attempt.measurementStarted && !eventStreamFailure
        && !attempt.measurementFailures.some(failure => failure.code === "PAGE_ENVIRONMENT_LOSS")) {
      throw new Error(`${key(attempt)} omits the observed event stream without a typed failure`);
    }
  } else {
    if (!Array.isArray(attempt.observedEvents) || eventStreamFailure) {
      throw new Error(`${key(attempt)} event stream contradicts its typed failure`);
    }
    const expectedEventReasons = new Set(independentEventBehaviorReasons(
      attempt.observedEvents,
      row.input,
    ));
    for (const reason of ["event-count-mismatch", "first-production-event-not-counted"] as const) {
      if (attempt.behaviorRedReasons.includes(reason) !== expectedEventReasons.has(reason)) {
        throw new Error(`${key(attempt)} event behavior RED ${reason} is not raw-derived`);
      }
    }
  }
  const behaviorEndpointMiss = attempt.measurementFailures.some(failure =>
    failure.stage === "candidate" && failure.disposition === "BEHAVIOR_RED"
  );
  if (attempt.prefixSamples.length !== row.expectedPrefixCount
      && (attempt.status !== "SETUP_INVALID" || behaviorEndpointMiss)
      && !attempt.behaviorRedReasons.includes("exact-prefix-count")) {
    throw new Error(`${key(attempt)} launders an incomplete prefix set`);
  }
  if (attempt.status !== "SETUP_INVALID" && (!attempt.measurementStarted || attempt.initialEventOrdinal !== 0)) {
    throw new Error(`${key(attempt)} did not arm before the first production event`);
  }
  if (attempt.status !== "SETUP_INVALID" && attempt.driverDispatches.length !== row.expectedPrefixCount
      && !attempt.behaviorRedReasons.includes("exact-driver-dispatch-count")) {
    throw new Error(`${key(attempt)} launders a missing driver dispatch`);
  }
  for (const [index, dispatch] of attempt.driverDispatches.entries()) {
    if (dispatch.prefixOrdinal !== index + 1 || dispatch.key !== row.input[index]
        || !Number.isFinite(dispatch.phaseDeadlineAt)
        || !Number.isFinite(dispatch.requestedDispatchAt) || !Number.isFinite(dispatch.actualDriverDispatchAt)
        || dispatch.requestedDispatchAt < dispatch.phaseDeadlineAt
        || dispatch.actualDriverDispatchAt < dispatch.requestedDispatchAt
        || dispatch.cadenceRebased !== (dispatch.requestedDispatchAt > dispatch.phaseDeadlineAt
          || dispatch.actualDriverDispatchAt - dispatch.phaseDeadlineAt > 1)) {
      throw new Error(`${key(attempt)} driver dispatch identity/order is invalid`);
    }
    if (index > 0) {
      const previous = attempt.driverDispatches[index - 1]!;
      const actualGap = dispatch.actualDriverDispatchAt - previous.actualDriverDispatchAt;
      const expectedPhase = Math.max(previous.phaseDeadlineAt, previous.actualDriverDispatchAt) + 60;
      if (Math.abs(dispatch.phaseDeadlineAt - expectedPhase) > 0.000_001 && actualGap >= 48) {
        throw new Error(`${key(attempt)} driver phase rebase is invalid`);
      }
    }
    if (dispatch.normalizedEventAt !== null && dispatch.eventDeliveredAt !== null
        && dispatch.eventDeliveredAt < dispatch.normalizedEventAt
        && !hasIndependentMetricFailure(attempt, "candidate", index + 1)) {
      throw new Error(`${key(attempt)} event delivery precedes normalized event time`);
    }
  }
  if (attempt.status !== "SETUP_INVALID" || attempt.measurementStarted) {
    if (!attempt.frame) {
      if (attempt.status !== "SETUP_INVALID"
          && !attempt.behaviorRedReasons.includes("frame-window-missing")) {
        throw new Error(`${key(attempt)} omits the frame window without a retained failure`);
      }
    } else {
      validateFrameReceipt(attempt);
    }
  }
  const seenPrefixOrdinals = new Set<number>();
  for (const sample of attempt.prefixSamples) {
    const prefix = row.input.slice(0, sample.prefixOrdinal);
    if (sample.prefixOrdinal < 1 || sample.prefixOrdinal > row.expectedPrefixCount
        || seenPrefixOrdinals.has(sample.prefixOrdinal) || sample.expectedPrefix !== prefix) {
      throw new Error(`${key(attempt)} prefix sequence is missing/reordered`);
    }
    seenPrefixOrdinals.add(sample.prefixOrdinal);
    const dispatch = attempt.driverDispatches[sample.prefixOrdinal - 1];
    if (sample.requestedDispatchAt !== dispatch?.requestedDispatchAt
        || sample.actualDriverDispatchAt !== dispatch.actualDriverDispatchAt) {
      throw new Error(`${key(attempt)} substituted the wrong driver dispatch for prefix ${sample.prefixOrdinal}`);
    }
    if ((sample.observation.event.ordinal !== sample.prefixOrdinal
        || dispatch.normalizedEventAt !== sample.observation.event.timeStamp
        || dispatch.eventDeliveredAt !== (sample.observation.event.deliveredAt ?? null))
        && !hasIndependentMetricFailure(attempt, "candidate", sample.prefixOrdinal)) {
      throw new Error(`${key(attempt)} substituted the wrong event for prefix ${sample.prefixOrdinal}`);
    }
    const candidateReds = validateCandidate(
      sample.observation,
      prefix,
      attempt.app,
      sample.prefixOrdinal === row.expectedPrefixCount ? row.shape[attempt.app] : undefined,
    );
    const expectedSelector = attempt.app === "yune-web"
      ? "yune-web-public-dom-v1" : "my-rime-c73ea17-public-dom-v1";
    const metricFailures = independentMetricContractFailures(sample.observation, expectedSelector);
    if (metricFailures.length > 0) {
      const retainedMetricFailure = attempt.measurementFailures.some(failure =>
        failure.stage === "candidate" && failure.prefixOrdinal === sample.prefixOrdinal
        && failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE"
        && failure.disposition === "SETUP_INVALID"
      );
      if (!retainedMetricFailure) {
        throw new Error(`${key(attempt)} launders candidate observer/metric-contract failure`);
      }
    }
    for (const reason of candidateReds) {
      if (!attempt.behaviorRedReasons.includes(`prefix-${sample.prefixOrdinal}:${reason}`)) {
        throw new Error(`${key(attempt)} omits retained candidate behavior RED ${sample.prefixOrdinal}:${reason}`);
      }
    }
    if (sample.prefixOrdinal === row.expectedPrefixCount) {
      const observed = new Set(sample.observation.secondRaf.candidates.map(candidate => candidate.text));
      if (!row.membership[attempt.app].every(text => observed.has(text))
          && !attempt.behaviorRedReasons.includes(
            `prefix-${sample.prefixOrdinal}:frozen-final-candidate-membership-mismatch`,
          )) {
        throw new Error(`${key(attempt)} omits the frozen final candidate membership RED`);
      }
    }
    const duration = sample.observation.secondRaf.observedAt - sample.observation.event.timeStamp;
    if (!Number.isFinite(duration) || duration < 0 || Math.abs(duration - sample.eventToStableCandidateMs) > 0.000_001) {
      if (!hasIndependentMetricFailure(attempt, "candidate", sample.prefixOrdinal)) {
        throw new Error(`${key(attempt)} prefix duration is negative/nonfinite/not raw-derived`);
      }
    }
  }
  if (attempt.commit) {
    const finalPrefix = attempt.prefixSamples.find(sample =>
      sample.prefixOrdinal === row.expectedPrefixCount
    );
    if (!finalPrefix) {
      throw new Error(`${key(attempt)} committed without a coherent final-prefix endpoint`);
    }
    const commitReds = validateCommit(attempt.commit, attempt.committedValue, attempt.app);
    const expectedSelector = attempt.app === "yune-web"
      ? "yune-web-public-dom-v1" : "my-rime-c73ea17-public-dom-v1";
    const metricFailures = independentMetricContractFailures(attempt.commit, expectedSelector);
    if (metricFailures.length > 0) {
      const retainedMetricFailure = attempt.measurementFailures.some(failure =>
        failure.stage === "commit" && failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE"
        && failure.disposition === "SETUP_INVALID"
      );
      if (!retainedMetricFailure) {
        throw new Error(`${key(attempt)} launders commit observer/metric-contract failure`);
      }
    }
    for (const reason of commitReds) {
      if (!attempt.behaviorRedReasons.includes(`commit:${reason}`)) {
        throw new Error(`${key(attempt)} omits retained commit behavior RED ${reason}`);
      }
    }
    for (const reason of independentCandidateCommitBehaviorReasons(
      finalPrefix,
      attempt.commit,
      attempt.committedValue,
    )) {
      if (!attempt.behaviorRedReasons.includes(`commit-sequence:${reason}`)) {
        throw new Error(`${key(attempt)} omits retained commit linkage RED ${reason}`);
      }
    }
    const dispatch = attempt.commitDriverDispatch;
    if (!dispatch
        || dispatch.prefixOrdinal !== row.input.length + 1 || dispatch.key !== " "
        || !Number.isFinite(dispatch.phaseDeadlineAt)
        || !Number.isFinite(dispatch.requestedDispatchAt)
        || !Number.isFinite(dispatch.actualDriverDispatchAt)
        || dispatch.phaseDeadlineAt !== dispatch.requestedDispatchAt
        || dispatch.requestedDispatchAt !== dispatch.actualDriverDispatchAt
        || dispatch.cadenceRebased) {
      throw new Error(`${key(attempt)} has an invalid commit driver dispatch`);
    }
    if ((attempt.commit.event.ordinal !== row.input.length + 1
        || attempt.commitDriverDispatch?.normalizedEventAt !== attempt.commit.event.timeStamp
        || attempt.commitDriverDispatch.eventDeliveredAt !== (attempt.commit.event.deliveredAt ?? null))
        && !hasIndependentMetricFailure(attempt, "commit", null)) {
      throw new Error(`${key(attempt)} substituted the wrong commit event`);
    }
  }
  else if (attempt.status !== "SETUP_INVALID"
      && !attempt.measurementFailures.some(failure =>
        failure.stage === "commit"
        || (failure.stage === "candidate"
          && failure.prefixOrdinal === row.expectedPrefixCount
          && failure.disposition === "BEHAVIOR_RED")
      )) {
    throw new Error(`${key(attempt)} omits commit without a retained behavior RED`);
  }
  const driverGaps = attempt.driverDispatches.slice(1).map((dispatch, index) =>
    dispatch.actualDriverDispatchAt
      - (attempt.driverDispatches[index]?.actualDriverDispatchAt ?? dispatch.actualDriverDispatchAt)
  );
  if (canonical(driverGaps) !== canonical(attempt.actualDispatchGapsMs)
      || attempt.actualDispatchGapsMs.length !== Math.max(0, attempt.driverDispatches.length - 1)
      || attempt.actualDispatchGapsMs.some(value => !Number.isFinite(value) || value < 0)) {
    throw new Error(`${key(attempt)} cadence samples are invalid`);
  }
  const expectedCadenceReasons: string[] = [];
  if (driverGaps.some(value => value < 48)) expectedCadenceReasons.push("invalid-cadence-too-short");
  if (driverGaps.some(value => value > 75)) expectedCadenceReasons.push("invalid-cadence-too-long");
  const declaredCadenceReasons = attempt.setupInvalidReasons.filter(reason =>
    reason === "invalid-cadence-too-short" || reason === "invalid-cadence-too-long"
  );
  if (canonical(expectedCadenceReasons) !== canonical(declaredCadenceReasons)) {
    throw new Error(`${key(attempt)} cadence setup reasons are not raw-derived`);
  }
  const expectedSetupReasons = independentlyDerivedSetupReasons(attempt, driverGaps);
  if (!Array.isArray(attempt.timedAssetRequests)
      || new Set(attempt.setupInvalidReasons).size !== attempt.setupInvalidReasons.length
      || canonical([...expectedSetupReasons].sort()) !== canonical([...attempt.setupInvalidReasons].sort())
      || (attempt.consoleErrors.length > 0)
        !== attempt.behaviorRedReasons.includes("console-or-page-error")) {
    throw new Error(`${key(attempt)} network/console failure receipts are not losslessly classified`);
  }
  if (attempt.measurementStarted && attempt.app === "yune-web"
      && attempt.rowId === "phase4-peer/luna-short-ni") {
    const expectedLatencyReds: string[] = [];
    for (const sample of attempt.prefixSamples) {
      if (sample.eventToStableCandidateMs > 67) {
        expectedLatencyReds.push(`short-prefix-${sample.prefixOrdinal}-over-67ms`);
      }
    }
    const commitMs = attempt.commit
      ? attempt.commit.secondRaf.observedAt - attempt.commit.event.timeStamp
      : undefined;
    if (commitMs !== undefined && commitMs > 67) expectedLatencyReds.push("short-commit-over-67ms");
    if (attempt.frame) {
      if (attempt.frame.interactionFrameIntervalsMs.some(value => value >= 50)) {
        expectedLatencyReds.push("short-frame-interval-at-least-50ms");
      }
      if (nearestRank(attempt.frame.interactionFrameIntervalsMs, 0.99) > 35.4) {
        expectedLatencyReds.push("short-frame-p99-over-35.4ms");
      }
      if (attempt.frame.interactionLongTasks.some(entry => entry.duration >= 50)) {
        expectedLatencyReds.push("short-long-task-at-least-50ms");
      }
    }
    if (canonical(expectedLatencyReds.sort()) !== canonical([...attempt.latencyRedReasons].sort())) {
      throw new Error(`${key(attempt)} short-row latency RED set is not raw-derived`);
    }
  }
  const retainedHardRed = attempt.behaviorRedReasons.length > 0
    || attempt.latencyRedReasons.length > 0;
  const cadencePrecedenceHardRed = retainedHardRed
    && independentlyHasRawCadencePrecedenceHardRed(attempt, row);
  const blockingSetupReasons = cadencePrecedenceHardRed
    ? attempt.setupInvalidReasons.filter(reason => reason !== "invalid-cadence-too-long")
    : attempt.setupInvalidReasons;
  const validForLatencyFrame = blockingSetupReasons.length === 0;
  const derivedStatus = validForLatencyFrame
    ? retainedHardRed ? "VALID_RED" : "VALID_GREEN"
    : "SETUP_INVALID";
  if (attempt.status !== derivedStatus || attempt.retainedHardRed !== retainedHardRed
      || attempt.validForLatencyFrame !== validForLatencyFrame) {
    throw new Error(`${key(attempt)} validity/RED dimensions were not independently derived`);
  }
}

function independentFailureDisposition(code: string): "SETUP_INVALID" | "BEHAVIOR_RED" {
  return behaviorMeasurementFailureCodes.has(code) ? "BEHAVIOR_RED" : "SETUP_INVALID";
}

function independentFailureReason(
  failure: Web06PeerAttempt["measurementFailures"][number],
): string {
  return ["measurement-failure", failure.stage, failure.prefixOrdinal ?? "none", failure.code].join(":");
}

function validateMeasurementFailuresIndependently(
  attempt: Web06PeerAttempt,
  expectedPrefixCount: number,
): void {
  if (!Array.isArray(attempt.measurementFailures)
      || !Array.isArray(attempt.setupInvalidReasons)
      || !Array.isArray(attempt.behaviorRedReasons)) {
    throw new Error(`${key(attempt)} omits typed failure dimensions`);
  }
  const seen = new Set<string>();
  const retainedReasons = new Set<string>();
  for (const failure of attempt.measurementFailures) {
    const identity = canonical(failure);
    if (seen.has(identity) || !measurementFailureCodes.has(failure.code)) {
      throw new Error(`${key(attempt)} has a duplicate/unknown measurement failure`);
    }
    seen.add(identity);
    if (failure.disposition !== independentFailureDisposition(failure.code)) {
      throw new Error(`${key(attempt)} changes a measurement failure disposition`);
    }
    const candidateCode = failure.code.startsWith("CANDIDATE_")
      || failure.code === "ENDPOINT_OBSERVER_MISSING"
      || failure.code === "ENDPOINT_OBSERVER_WRONG_APP"
      || failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE"
      || failure.code === "PAGE_ENVIRONMENT_LOSS";
    const commitCode = failure.code.startsWith("COMMIT_")
      || failure.code === "ENDPOINT_OBSERVER_MISSING"
      || failure.code === "ENDPOINT_OBSERVER_WRONG_APP"
      || failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE"
      || failure.code === "PAGE_ENVIRONMENT_LOSS";
    if (failure.stage === "candidate") {
      if (!candidateCode || !Number.isInteger(failure.prefixOrdinal)
          || (failure.prefixOrdinal ?? 0) < 1 || (failure.prefixOrdinal ?? 0) > expectedPrefixCount) {
        throw new Error(`${key(attempt)} has an invalid candidate failure identity`);
      }
      const sample = attempt.prefixSamples.find(item => item.prefixOrdinal === failure.prefixOrdinal);
      if ((failure.code === "CANDIDATE_ENDPOINT_TIMEOUT"
          || failure.code === "CANDIDATE_ENDPOINT_SUPERSEDED"
          || failure.code === "ENDPOINT_OBSERVER_MISSING"
          || failure.code === "ENDPOINT_OBSERVER_WRONG_APP"
          || failure.code === "PAGE_ENVIRONMENT_LOSS") && sample) {
        throw new Error(`${key(attempt)} candidate failure contradicts a retained endpoint`);
      }
      if (failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE" && sample) {
        const selector = attempt.app === "yune-web"
          ? "yune-web-public-dom-v1" : "my-rime-c73ea17-public-dom-v1";
        const detectable = independentMetricContractFailures(sample.observation, selector).length > 0;
        if (!detectable) {
          throw new Error(`${key(attempt)} declares an unproved candidate metric failure`);
        }
      }
    } else if (failure.stage === "commit") {
      if (!commitCode || failure.prefixOrdinal !== null) {
        throw new Error(`${key(attempt)} has an invalid commit failure identity`);
      }
      if ((failure.code === "COMMIT_ENDPOINT_TIMEOUT"
          || failure.code === "COMMIT_ENDPOINT_SUPERSEDED"
          || failure.code === "ENDPOINT_OBSERVER_MISSING"
          || failure.code === "ENDPOINT_OBSERVER_WRONG_APP"
          || failure.code === "PAGE_ENVIRONMENT_LOSS") && attempt.commit) {
        throw new Error(`${key(attempt)} commit failure contradicts a retained endpoint`);
      }
      if (failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE" && attempt.commit) {
        const selector = attempt.app === "yune-web"
          ? "yune-web-public-dom-v1" : "my-rime-c73ea17-public-dom-v1";
        const detectable = independentMetricContractFailures(attempt.commit, selector).length > 0;
        if (!detectable) {
          throw new Error(`${key(attempt)} declares an unproved commit metric failure`);
        }
      }
    } else if (failure.stage === "frame") {
      if (!failure.code.startsWith("FRAME_") || failure.prefixOrdinal !== null || attempt.frame !== null) {
        throw new Error(`${key(attempt)} has an invalid frame failure identity`);
      }
    } else if (failure.stage === "post-dispatch") {
      if ((failure.code !== "POST_DISPATCH_METRIC_CONTRACT_FAILURE"
          && failure.code !== "EVENT_STREAM_METRIC_CONTRACT_FAILURE"
          && failure.code !== "PAGE_ENVIRONMENT_LOSS") || failure.prefixOrdinal !== null) {
        throw new Error(`${key(attempt)} has an invalid post-dispatch failure identity`);
      }
      if (failure.code === "POST_DISPATCH_METRIC_CONTRACT_FAILURE"
          && attempt.frame && attempt.commit
          && attempt.prefixSamples.length === expectedPrefixCount) {
        throw new Error(`${key(attempt)} declares an unproved post-dispatch metric failure`);
      }
    } else {
      throw new Error(`${key(attempt)} has an unknown measurement failure stage`);
    }
    if (failure.code === "PAGE_ENVIRONMENT_LOSS" && attempt.foregroundAndFocused) {
      throw new Error(`${key(attempt)} environment-loss code contradicts final foreground state`);
    }
    const reason = independentFailureReason(failure);
    retainedReasons.add(reason);
    const owner = failure.disposition === "SETUP_INVALID"
      ? attempt.setupInvalidReasons : attempt.behaviorRedReasons;
    const other = failure.disposition === "SETUP_INVALID"
      ? attempt.behaviorRedReasons : attempt.setupInvalidReasons;
    if (!owner.includes(reason) || other.includes(reason)) {
      throw new Error(`${key(attempt)} stores a typed failure in the wrong dimension`);
    }
  }
  for (const reason of [...attempt.setupInvalidReasons, ...attempt.behaviorRedReasons]) {
    if (reason.startsWith("measurement-failure:") && !retainedReasons.has(reason)) {
      throw new Error(`${key(attempt)} has an orphaned measurement-failure reason`);
    }
    if (reason.startsWith("post-dispatch-contract-exception") || reason === "frame-window-missing") {
      throw new Error(`${key(attempt)} uses a legacy untyped post-dispatch failure`);
    }
  }
  if (attempt.measurementStarted && !attempt.frame) {
    const retained = attempt.measurementFailures.some(failure =>
      failure.stage === "frame" || failure.stage === "post-dispatch"
      || failure.code === "PAGE_ENVIRONMENT_LOSS"
    );
    if (!retained) throw new Error(`${key(attempt)} omits its frame without a setup-invalid cause`);
  }
}

function independentlyDerivedSetupReasons(
  attempt: Web06PeerAttempt,
  driverGaps: number[],
): string[] {
  const reasons: string[] = [];
  if (!attempt.measurementStarted) reasons.push("setup-before-measurement");
  if (!attempt.networkRouteInstalled) reasons.push("network-route-not-installed-before-navigation");
  if (attempt.pretypingIdleMedianMs !== null
      && (!Number.isFinite(attempt.pretypingIdleMedianMs)
        || attempt.pretypingIdleMedianMs < 15 || attempt.pretypingIdleMedianMs > 18)) {
    reasons.push("pretyping-idle-raf-median-outside-15-to-18ms");
  }
  if (attempt.frame) {
    if (!Number.isFinite(attempt.frame.idleMedianMs)
        || attempt.frame.idleMedianMs < 15 || attempt.frame.idleMedianMs > 18) {
      reasons.push("final-idle-raf-median-outside-15-to-18ms");
    }
    if (attempt.frame.visibilityOrFocusLost) reasons.push("visibility-or-focus-lost-during-window");
  }
  if (attempt.blockedSetupNetworkRequests.length > 0) reasons.push("blocked-setup-network-request");
  if (attempt.timedAssetRequests.length > 0) reasons.push("asset-request-during-timed-window");
  if (attempt.unexpectedNetworkRequests.length > 0) reasons.push("unexpected-network-request");
  if (driverGaps.some(value => value < 48)) reasons.push("invalid-cadence-too-short");
  if (driverGaps.some(value => value > 75)) reasons.push("invalid-cadence-too-long");
  if (!attempt.foregroundAndFocused) reasons.push("foreground-or-focus-lost");
  reasons.push(...attempt.measurementFailures
    .filter(failure => failure.disposition === "SETUP_INVALID")
    .map(independentFailureReason));
  return [...new Set(reasons)];
}

function hasIndependentMetricFailure(
  attempt: Web06PeerAttempt,
  stage: "candidate" | "commit",
  prefixOrdinal: number | null,
): boolean {
  return attempt.measurementFailures.some(failure =>
    failure.stage === stage && failure.prefixOrdinal === prefixOrdinal
    && failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE"
    && failure.disposition === "SETUP_INVALID"
  );
}

function independentMetricContractFailures(
  observation: ComparatorStableObservation,
  expectedSelector: string,
): string[] {
  const failures = [...independentStableObservationFailures(observation)];
  const event = observation.event;
  if (!Number.isInteger(event.ordinal) || event.ordinal < 1 || event.type !== "keydown"
      || typeof event.key !== "string" || typeof event.code !== "string"
      || !Number.isFinite(event.timeStamp) || event.timeStamp < 0
      || !Number.isFinite(event.revisionBeforeEvent) || event.revisionBeforeEvent < 0) {
    failures.push("event-boundary-invalid");
  }
  for (const tuple of [observation.initial, observation.firstRaf, observation.secondRaf]) {
    if (tuple.contractVersion !== "web06-comparator-endpoint-v1"
        || tuple.selectorManifestId !== expectedSelector
        || !Number.isInteger(tuple.revision) || tuple.revision < 0
        || !Number.isFinite(tuple.observedAt) || tuple.observedAt < 0) {
      failures.push("dom-observation-contract-invalid");
    }
  }
  if (observation.initial.revision <= event.revisionBeforeEvent
      || observation.initial.observedAt < event.timeStamp) {
    failures.push("accepted-observation-precedes-event");
  }
  return [...new Set(failures)];
}

function validateFrameReceipt(attempt: Web06PeerAttempt): void {
  const frame = attempt.frame!;
  const idleIntervals = intervals(frame.idleRafTimestamps);
  const interactionIntervals = intervals(frame.interactionRafTimestamps);
  const interactionDuration = frame.interactionWindowEndAt - frame.interactionWindowStartAt;
  const idleControlDuration = frame.idleControlWindowEndAt - frame.idleControlWindowStartAt;
  const finiteSeries = [
    ...frame.idleRafTimestamps,
    ...frame.idleIntervalsMs,
    ...frame.idleControlRafTimestamps,
    ...frame.interactionRafTimestamps,
    ...frame.interactionFrameIntervalsMs,
  ].every(value => Number.isFinite(value) && value >= 0);
  if (frame.longTaskObserverSupported !== true
      || !finiteSeries
      || frame.idleRafTimestamps.length !== 121 || frame.idleIntervalsMs.length !== 120
      || frame.interactionRafTimestamps.length < 2
      || frame.interactionFrameIntervalsMs.length !== frame.interactionRafTimestamps.length - 1
      || frame.idleControlRafTimestamps.length < 2
      || interactionDuration <= 0 || idleControlDuration <= 0
      || Math.abs(interactionDuration - idleControlDuration) > 0.000_001
      || frame.idleRafTimestamps.at(-1) !== frame.interactionWindowStartAt
      || frame.interactionRafTimestamps[0] !== frame.interactionWindowStartAt
      || frame.interactionRafTimestamps.at(-1) !== frame.interactionWindowEndAt
      || frame.idleControlWindowStartAt !== frame.interactionWindowEndAt
      || frame.idleControlRafTimestamps[0] !== frame.idleControlWindowStartAt
      || (frame.idleControlRafTimestamps.at(-1) ?? -1) < frame.idleControlWindowEndAt
      || (frame.idleControlRafTimestamps.at(-2) ?? Number.POSITIVE_INFINITY) >= frame.idleControlWindowEndAt
      || !sameNumbers(idleIntervals, frame.idleIntervalsMs)
      || !sameNumbers(interactionIntervals, frame.interactionFrameIntervalsMs)
      || Math.abs(median(frame.idleIntervalsMs) - frame.idleMedianMs) > 0.000_001) {
    throw new Error(`${key(attempt)} frame/Long-Task receipt is not independently reproducible`);
  }
  if (frame.observedLongTasks.some(entry => !Number.isFinite(entry.startTime) || entry.startTime < 0
    || !Number.isFinite(entry.duration) || entry.duration < 0)) {
    throw new Error(`${key(attempt)} contains an invalid observed Long Task`);
  }
  const overlaps = (
    entry: { startTime: number; duration: number },
    start: number,
    end: number,
  ) => entry.startTime < end && entry.startTime + entry.duration > start;
  const expectedInteraction = frame.observedLongTasks.filter(entry => overlaps(
    entry, frame.interactionWindowStartAt, frame.interactionWindowEndAt,
  ));
  const expectedIdle = frame.observedLongTasks.filter(entry => overlaps(
    entry, frame.idleControlWindowStartAt, frame.idleControlWindowEndAt,
  ));
  const expectedOutside = frame.observedLongTasks.filter(entry =>
    !overlaps(entry, frame.interactionWindowStartAt, frame.interactionWindowEndAt)
    && !overlaps(entry, frame.idleControlWindowStartAt, frame.idleControlWindowEndAt)
  );
  if (!sameTaskMultiset(expectedInteraction, frame.interactionLongTasks)
      || !sameTaskMultiset(expectedIdle, frame.idleLongTasks)
      || !sameTaskMultiset(expectedOutside, frame.outsideWindowLongTasks)) {
    throw new Error(`${key(attempt)} Long Task window classification is not raw-derived`);
  }
}

function independentEventBehaviorReasons(
  events: NonNullable<Web06PeerAttempt["observedEvents"]>,
  expectedInput: string,
): string[] {
  const reasons: string[] = [];
  if (events.length !== expectedInput.length + 1) reasons.push("event-count-mismatch");
  if (events[0]?.ordinal !== 1 || events[0]?.key !== expectedInput[0]) {
    reasons.push("first-production-event-not-counted");
  }
  return reasons;
}

function independentlyHasRawCadencePrecedenceHardRed(
  attempt: Web06PeerAttempt,
  row: (typeof rows)[number],
): boolean {
  if (attempt.measurementFailures.some(failure => failure.disposition === "BEHAVIOR_RED")
      || attempt.consoleErrors.length > 0
      || (attempt.observedEvents !== null
        && independentEventBehaviorReasons(attempt.observedEvents, row.input).length > 0)) {
    return true;
  }
  const behaviorEndpointMiss = attempt.measurementFailures.some(failure =>
    failure.stage === "candidate" && failure.disposition === "BEHAVIOR_RED"
  );
  if (attempt.prefixSamples.length !== row.expectedPrefixCount && behaviorEndpointMiss) return true;
  for (const sample of attempt.prefixSamples) {
    if (validateCandidate(
      sample.observation,
      sample.expectedPrefix,
      attempt.app,
      sample.prefixOrdinal === row.expectedPrefixCount ? row.shape[attempt.app] : undefined,
    ).length > 0) {
      return true;
    }
    if (sample.prefixOrdinal === row.expectedPrefixCount) {
      const observed = new Set(sample.observation.secondRaf.candidates.map(candidate => candidate.text));
      if (!row.membership[attempt.app].every(text => observed.has(text))) return true;
    }
  }
  const finalPrefix = attempt.prefixSamples.find(sample =>
    sample.prefixOrdinal === row.expectedPrefixCount
  );
  if (finalPrefix) {
    const selected = finalPrefix.observation.secondRaf.candidates[
      finalPrefix.observation.secondRaf.highlightedIndex
    ]?.text;
    if (!selected) return true;
  }
  if (attempt.commit) {
    if (validateCommit(attempt.commit, attempt.committedValue, attempt.app).length > 0
        || (finalPrefix && independentCandidateCommitBehaviorReasons(
          finalPrefix,
          attempt.commit,
          attempt.committedValue,
        ).length > 0)) {
      return true;
    }
  }
  if (attempt.app === "yune-web" && attempt.rowId === "phase4-peer/luna-short-ni") {
    if (attempt.prefixSamples.some(sample => sample.eventToStableCandidateMs > 67)) return true;
    if (attempt.commit
        && attempt.commit.secondRaf.observedAt - attempt.commit.event.timeStamp > 67) return true;
    if (attempt.frame?.interactionFrameIntervalsMs.some(value => value >= 50)
        || (attempt.frame && nearestRank(attempt.frame.interactionFrameIntervalsMs, 0.99) > 35.4)
        || attempt.frame?.interactionLongTasks.some(entry => entry.duration >= 50)) {
      return true;
    }
  }
  return false;
}

function independentCandidateCommitBehaviorReasons(
  finalPrefix: Web06PeerAttempt["prefixSamples"][number],
  commit: ComparatorStableObservation,
  committedValue: string,
): string[] {
  const reasons: string[] = [];
  const finalTuple = finalPrefix.observation.secondRaf;
  const selected = finalTuple.candidates[finalTuple.highlightedIndex]?.text;
  if (commit.event.ordinal !== finalPrefix.observation.event.ordinal + 1) {
    reasons.push("commit-event-does-not-immediately-follow-final-prefix");
  }
  if (commit.event.timeStamp < finalTuple.observedAt) {
    reasons.push("commit-event-precedes-coherent-final-prefix");
  }
  if (commit.event.revisionBeforeEvent < finalTuple.revision) {
    reasons.push("commit-event-revision-precedes-final-prefix");
  }
  if (!selected || committedValue !== selected || commit.secondRaf.caret.value !== selected) {
    reasons.push("commit-value-does-not-match-highlighted-final-candidate");
  }
  return reasons;
}

function validateCandidate(
  observation: ComparatorStableObservation,
  prefix: string,
  app: "yune-web" | "my-rime",
  expectedPageShape: { candidateCount: number; nextDisabled: boolean } = {
    candidateCount: 6,
    nextDisabled: false,
  },
): string[] {
  const tuple = observation.secondRaf;
  const reasons: string[] = [];
  const expectedKey = prefix.at(-1) ?? "";
  if (observation.event.key !== expectedKey) reasons.push("candidate-final-event-key-mismatch");
  if (observation.event.code !== `Key${expectedKey.toUpperCase()}`) {
    reasons.push("candidate-final-event-code-mismatch");
  }
  if (tuple.composition !== prefix) reasons.push("candidate-composition-is-not-complete-input");
  if (tuple.candidateSurfaceCount !== 1 || tuple.candidates.length === 0) {
    reasons.push("candidate-surface-is-not-one-complete-visible-collection");
  }
  if (tuple.candidates.some(candidate => candidate.text === "")) {
    reasons.push("candidate-collection-contains-empty-visible-text");
  }
  if (tuple.candidates.some((candidate, index) =>
    candidate.label.replace(/[.\s]/g, "") !== String(index + 1)
  )) reasons.push("candidate-collection-label-order-is-not-the-frozen-default");
  if (tuple.candidates.length !== expectedPageShape.candidateCount) {
    reasons.push("candidate-page-size-does-not-match-frozen-shape");
  }
  if (tuple.page.index !== 0 || tuple.page.buttonCount !== 2
      || tuple.page.previousDisabled !== true
      || tuple.page.nextDisabled !== expectedPageShape.nextDisabled) {
    reasons.push("candidate-page-evidence-incomplete");
  }
  if (tuple.highlightedIndex < 0 || tuple.highlightedIndex >= tuple.candidates.length) {
    reasons.push("candidate-highlight-evidence-incomplete");
  }
  if (tuple.caret.selectorCount !== 1 || !tuple.caret.active || !tuple.caret.visible
      || tuple.caret.disabled || tuple.caret.selectionStart === null || tuple.caret.selectionEnd === null) {
    reasons.push("candidate-caret-evidence-incomplete");
  }
  if (!tuple.status.schemaId || !tuple.status.composing || !tuple.status.surfaceVisible) {
    reasons.push("candidate-status-evidence-incomplete");
  }
  if ([observation.initial, observation.firstRaf, observation.secondRaf]
    .some(value => value.status.schemaId !== "luna_pinyin")) {
    reasons.push("active-schema-is-not-luna-pinyin");
  }
  return reasons;
}

function validateCommit(
  observation: ComparatorStableObservation,
  committedValue: string,
  app: "yune-web" | "my-rime",
): string[] {
  const tuple = observation.secondRaf;
  const reasons: string[] = [];
  if (observation.event.key !== " " || observation.event.code !== "Space") {
    reasons.push("commit-event-is-not-space-key-code");
  }
  if (tuple.caret.value !== committedValue) reasons.push("committed-visible-value-mismatch");
  if (tuple.caret.selectionStart !== committedValue.length
      || tuple.caret.selectionEnd !== committedValue.length) reasons.push("committed-caret-mismatch");
  if (tuple.composition !== "" || tuple.candidateSurfaceCount !== 0 || tuple.candidates.length !== 0) {
    reasons.push("commit-left-visible-composition-or-candidates");
  }
  if (tuple.caret.selectorCount !== 1 || !tuple.caret.active || !tuple.caret.visible
      || tuple.caret.disabled || tuple.status.composing || tuple.status.surfaceVisible) {
    reasons.push("commit-visible-surface-status-incomplete");
  }
  if (!tuple.status.schemaId) reasons.push("commit-schema-status-missing");
  if ([observation.initial, observation.firstRaf, observation.secondRaf]
    .some(value => value.status.schemaId !== "luna_pinyin")) {
    reasons.push("active-schema-is-not-luna-pinyin");
  }
  return reasons;
}

function recomputeCompact(
  packet: Web06PeerRawPacket,
  attestation: Web06PeerAttestation,
  rawSha256: string,
  attemptDigests: Map<string, string>,
): Web06PeerCompactReceipt {
  const alignmentReasons = independentlyConfounded(attestation.identityManifest);
  const pairedValidByRow = new Map(rows.map(row => [row.id, pairedOrdinals(packet, row.id)]));
  const compactRows = rows.flatMap(row => apps.map(app => {
    const attempts = packet.attempts.filter(item => item.rowId === row.id && item.app === app);
    const pairedValidAttemptOrdinals = pairedValidByRow.get(row.id) ?? [];
    const pairedSet = new Set(pairedValidAttemptOrdinals);
    const valid = attempts.filter(attempt => pairedSet.has(attempt.attempt));
    const reds = valid.filter(attempt => attempt.retainedHardRed);
    const retainedHardReds = attempts.filter(attempt => attempt.retainedHardRed);
    const verdictBearingReds = attempts.filter(independentVerdictBearingRed);
    const finalPrefix = valid.flatMap(attempt => {
      const sample = attempt.prefixSamples.find(item => item.prefixOrdinal === row.expectedPrefixCount);
      return sample ? [sample.eventToStableCandidateMs] : [];
    });
    const allPrefixes = valid.flatMap(attempt => attempt.prefixSamples.map(sample => sample.eventToStableCandidateMs));
    const commits = valid.flatMap(attempt => attempt.commit
      ? [attempt.commit.secondRaf.observedAt - attempt.commit.event.timeStamp]
      : []);
    const driverGaps = attempts.flatMap(attempt => attempt.actualDispatchGapsMs);
    const validFrames = valid.flatMap(attempt => attempt.frame ? [attempt.frame] : []);
    const rawAttemptPacketSha256s = attempts.map(attempt => {
      const value = attemptDigests.get(key(attempt));
      if (!value) throw new Error(`${key(attempt)} digest missing`);
      return value;
    });
    return {
      rowId: row.id as Web06PeerRowId,
      app,
      packageAlignment: "DATA_CONFOUNDED" as const,
      attemptsRetained: attempts.length,
      validRounds: valid.length,
      validGreenRounds: valid.length - reds.length,
      validRedRounds: reds.length,
      retainedHardRedAttempts: retainedHardReds.length,
      verdictBearingRedAttempts: verdictBearingReds.length,
      invalidHardRedAttempts: attempts.filter(attempt =>
        attempt.retainedHardRed && !attempt.validForLatencyFrame
      ).length,
      setupInvalidAttempts: attempts.filter(attempt => !attempt.validForLatencyFrame).length,
      unpairedValidAttemptsRetained: attempts.filter(attempt =>
        attempt.validForLatencyFrame && !pairedSet.has(attempt.attempt)
      ).length,
      cadence: independentCadenceCounts(driverGaps),
      pairedValidAttemptOrdinals,
      verdict: valid.length < 5
        ? "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS" as const
        : verdictBearingReds.length ? "RED" as const : "PASS" as const,
      finalPrefix: metrics(finalPrefix),
      allPrefixes: metrics(allPrefixes),
      commit: metrics(commits),
      frameIntervals: frameMetrics(validFrames.flatMap(frame =>
        frame.interactionFrameIntervalsMs
      )),
      longTasks: independentLongTaskSet(validFrames),
      perRound: attempts.map((attempt, index) => {
        const finalSample = attempt.prefixSamples.find(sample =>
          sample.prefixOrdinal === row.expectedPrefixCount
        );
        return {
          attempt: attempt.attempt,
          status: attempt.status,
          retainedHardRed: attempt.retainedHardRed,
          verdictBearingRed: independentVerdictBearingRed(attempt),
          validForLatencyFrame: attempt.validForLatencyFrame,
          finalPrefix: metrics(finalSample ? [finalSample.eventToStableCandidateMs] : []),
          allPrefixes: metrics(attempt.prefixSamples.map(sample => sample.eventToStableCandidateMs)),
          commit: metrics(attempt.commit
            ? [attempt.commit.secondRaf.observedAt - attempt.commit.event.timeStamp]
            : []),
          cadence: independentCadenceCounts(attempt.actualDispatchGapsMs),
          frameIntervals: frameMetrics(attempt.frame?.interactionFrameIntervalsMs ?? []),
          longTasks: independentLongTaskSet(attempt.frame ? [attempt.frame] : []),
          rawAttemptPacketSha256: rawAttemptPacketSha256s[index] ?? "",
        };
      }),
      rawAttemptPacketSha256s,
    };
  }));
  const overallVerdict = compactRows.some(row => row.verdict === "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS")
    ? "SETUP_NO_GO" as const
    : compactRows.some(row => row.verdict === "RED") ? "RED" as const : "PASS_INFORMATIONAL_NO_RATIO" as const;
  return {
    version: "web06-phase4-peer-compact-v1",
    laneVersion: "web06-phase4-peer-lane-v1",
    phase: "phase4-peer",
    source: {
      yuneCommit: attestation.yune.sourceCommit,
      yuneTree: attestation.yune.sourceTree,
      yuneArchiveSha256: attestation.yune.archiveSha256,
      peerUpstreamCommit: attestation.identityManifest.peer.upstreamPinnedCommit,
      peerArtifactSourceCommit: attestation.identityManifest.peer.artifactSourceCommit,
      peerArtifactSourceTree: attestation.identityManifest.peer.artifactSourceTree,
      peerArchiveSha256: attestation.peer.archiveSha256,
    },
    contracts: {
      endpoint: "web06-comparator-endpoint-v1",
      inputRegistrySha256: attestation.inputRegistrySha256,
      selectorManifestSha256: attestation.selectorManifestSha256,
      configSha256: attestation.configSha256,
      attestationSha256: packet.attestationSha256,
      toolchainIdentitySha256: digest(canonical(attestation.toolchain)),
      negativeEssayControlSha256: attestation.negativeEssayControlSha256,
      identityManifestSha256: attestation.identityManifestSha256,
      finalRunEnvironmentSha256: attestation.finalRunEnvironmentSha256,
      rawPacketSha256: rawSha256,
    },
    environment: {
      viewport: { width: 1365, height: 900 },
      locale: "zh-HK",
      cacheRegime: "fresh-persistent-profile-no-store-v1",
      cadenceMs: 60,
      nodeVersion: attestation.toolchain.nodeVersion,
      npmVersion: attestation.toolchain.npmVersion,
      playwrightVersion: attestation.toolchain.playwrightVersion,
      chromiumVersion: attestation.toolchain.chromiumVersion,
      chromiumExecutableSha256: attestation.toolchain.chromiumExecutableSha256,
      platform: attestation.host.platform,
      architecture: attestation.host.architecture,
      environmentId: attestation.finalRunEnvironment.environmentId,
    },
    packageAlignment: "DATA_CONFOUNDED",
    ratio: {
      status: "OMITTED",
      reasons: [...new Set([
        ...alignmentReasons,
        "pinned-peer-packet-declares-data-confounded",
        "numeric-ratio-fields-structurally-omitted",
      ])].sort(),
    },
    rows: compactRows,
    overallVerdict,
  };
}

function pairedOrdinals(packet: Web06PeerRawPacket, rowId: Web06PeerRowId): number[] {
  const yune = packet.attempts.filter(item => item.rowId === rowId && item.app === "yune-web");
  const peer = packet.attempts.filter(item => item.rowId === rowId && item.app === "my-rime");
  for (const [app, attempts] of [["yune-web", yune], ["my-rime", peer]] as const) {
    if (attempts.length > 7 || attempts.some((attempt, index) => attempt.attempt !== index + 1)) {
      throw new Error(`${rowId}/${app} attempt identity is not contiguous within seven`);
    }
  }
  if (yune.length !== peer.length) throw new Error(`${rowId} has an incomplete retained peer pair`);
  const paired: number[] = [];
  for (let index = 0; index < yune.length; index += 1) {
    const yuneAttempt = yune[index]!;
    const peerAttempt = peer[index]!;
    if (yuneAttempt.attempt !== peerAttempt.attempt) throw new Error(`${rowId} pair identity is misaligned`);
    if (paired.length >= 5) throw new Error(`${rowId} continued after five paired valid rounds`);
    const yuneFirst = yuneAttempt.runOrderOrdinal < peerAttempt.runOrderOrdinal;
    if (yuneFirst !== (yuneAttempt.attempt % 2 === 1)) {
      throw new Error(`${rowId} violates frozen pair counterbalancing`);
    }
    if (yuneAttempt.validForLatencyFrame && peerAttempt.validForLatencyFrame) {
      paired.push(yuneAttempt.attempt);
    }
  }
  if (paired.length < 5 && yune.length !== 7) {
    throw new Error(`${rowId} ended before five paired valid rounds or seven retained pairs`);
  }
  return paired;
}

function verifyGlobalRunOrder(attempts: Web06PeerAttempt[]): void {
  const keys = new Set<string>();
  attempts.forEach((attempt, index) => {
    const attemptKey = key(attempt);
    if (keys.has(attemptKey)) throw new Error(`raw-attempt-identity-duplicate:${attemptKey}`);
    keys.add(attemptKey);
    if (!Number.isInteger(attempt.runOrderOrdinal) || attempt.runOrderOrdinal !== index + 1) {
      throw new Error(`raw-run-order-not-contiguous:index-${index + 1}:ordinal-${attempt.runOrderOrdinal}`);
    }
  });
  let cursor = 0;
  for (const row of rows) {
    let expectedAttempt = 1;
    while (cursor < attempts.length && attempts[cursor]?.rowId === row.id) {
      const first = attempts[cursor];
      const second = attempts[cursor + 1];
      const expectedApps = expectedAttempt % 2 === 1
        ? ["yune-web", "my-rime"] as const
        : ["my-rime", "yune-web"] as const;
      if (!first || !second || first.rowId !== row.id || second.rowId !== row.id
          || first.attempt !== expectedAttempt || second.attempt !== expectedAttempt
          || first.app !== expectedApps[0] || second.app !== expectedApps[1]) {
        throw new Error(`raw-frozen-row-pair-order:${row.id}:attempt-${expectedAttempt}`);
      }
      cursor += 2;
      expectedAttempt += 1;
    }
    if (expectedAttempt === 1) throw new Error(`raw-frozen-row-missing:${row.id}`);
  }
  if (cursor !== attempts.length) throw new Error(`raw-frozen-row-order-extra:index-${cursor + 1}`);
}

function independentlyConfounded(identity: ComparatorIdentityManifest): string[] {
  const reasons: string[] = [];
  const required = [
    "resolved-schema-includes-patches", "dictionary-and-imports", "essay", "grammar-model",
    "speller-algebra", "filters-and-options", "page-size-and-comments", "fresh-empty-userdb",
  ];
  for (const id of required) {
    const item = identity.logicalInputs.find(candidate => candidate.id === id);
    if (!item) reasons.push(`logical-input-missing:${id}`);
    else if (item.explicitNone
      ? item.yuneSha256 !== "none" || item.peerSha256 !== "none"
      : item.yuneSha256 !== item.peerSha256) reasons.push(`logical-input-different:${id}`);
  }
  if (identity.effectiveConfiguration.yuneSha256 !== identity.effectiveConfiguration.peerSha256) {
    reasons.push("effective-configuration-different");
  }
  if (!identity.freshEmptyUserdb) reasons.push("fresh-empty-userdb-not-proved");
  if (!identity.sameEndpointObserver) reasons.push("same-endpoint-observer-not-proved");
  if (reasons.length === 0) throw new Error("Sealed peer unexpectedly became PROVED");
  return reasons;
}

function metrics(values: number[]): {
  samples: number;
  medianMs: number;
  p95Ms: number;
  p99Ms: number;
  maxMs: number;
} | null {
  if (!values.length) return null;
  if (values.some(value => !Number.isFinite(value) || value < 0)) throw new Error("Negative/nonfinite duration in compact input");
  const sorted = values.slice().sort((left, right) => left - right);
  return {
    samples: sorted.length,
    medianMs: sorted[Math.ceil((sorted.length - 1) * 0.5)] ?? 0,
    p95Ms: sorted[Math.ceil((sorted.length - 1) * 0.95)] ?? 0,
    p99Ms: sorted[Math.ceil((sorted.length - 1) * 0.99)] ?? 0,
    maxMs: sorted.at(-1) ?? 0,
  };
}

function frameMetrics(values: number[]): {
  samples: number;
  medianMs: number;
  p95Ms: number;
  p99Ms: number;
  maxMs: number;
  atLeast50MsCount: number;
} | null {
  const metric = metrics(values);
  return metric ? {
    ...metric,
    atLeast50MsCount: values.filter(value => value >= 50).length,
  } : null;
}

function independentCadenceCounts(values: number[]): {
  driverGapSamples: number;
  inRangeGapSamples: number;
  tooShortGapSamples: number;
  delayedHostGapSamples: number;
} {
  return {
    driverGapSamples: values.length,
    inRangeGapSamples: values.filter(value => value >= 48 && value <= 75).length,
    tooShortGapSamples: values.filter(value => value < 48).length,
    delayedHostGapSamples: values.filter(value => value > 75).length,
  };
}

function independentLongTaskSet(
  frames: Array<NonNullable<Web06PeerAttempt["frame"]>>,
): {
  frameWindows: number;
  interactionWindowDurationTotalMs: number;
  observedCount: number;
  observedDurationTotalMs: number;
  interactionOverlapCount: number;
  interactionTaskDurationTotalMs: number;
  interactionTaskDurationMaxMs: number;
  idleOverlapCount: number;
  outsideWindowCount: number;
} | null {
  if (frames.length === 0) return null;
  const observed = frames.flatMap(frame => frame.observedLongTasks);
  const interaction = frames.flatMap(frame => frame.interactionLongTasks);
  return {
    frameWindows: frames.length,
    interactionWindowDurationTotalMs: frames.reduce(
      (sum, frame) => sum + frame.interactionWindowEndAt - frame.interactionWindowStartAt,
      0,
    ),
    observedCount: observed.length,
    observedDurationTotalMs: observed.reduce((sum, entry) => sum + entry.duration, 0),
    interactionOverlapCount: interaction.length,
    interactionTaskDurationTotalMs: interaction.reduce((sum, entry) => sum + entry.duration, 0),
    interactionTaskDurationMaxMs: Math.max(0, ...interaction.map(entry => entry.duration)),
    idleOverlapCount: frames.reduce((sum, frame) => sum + frame.idleLongTasks.length, 0),
    outsideWindowCount: frames.reduce((sum, frame) => sum + frame.outsideWindowLongTasks.length, 0),
  };
}

function intervals(timestamps: number[]): number[] {
  return timestamps.slice(1).map((value, index) => value - (timestamps[index] ?? value));
}

function sameNumbers(left: number[], right: number[]): boolean {
  return left.length === right.length && left.every((value, index) =>
    Number.isFinite(value) && Number.isFinite(right[index]) && Math.abs(value - (right[index] ?? value)) <= 0.000_001
  );
}

function sameTaskMultiset(
  left: Array<{ startTime: number; duration: number }>,
  right: Array<{ startTime: number; duration: number }>,
): boolean {
  const normalized = (values: Array<{ startTime: number; duration: number }>) => values
    .map(value => `${value.startTime}\0${value.duration}`)
    .sort();
  return canonical(normalized(left)) === canonical(normalized(right));
}

function median(values: number[]): number {
  const sorted = values.slice().sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2
    ? sorted[middle] ?? 0
    : ((sorted[middle - 1] ?? 0) + (sorted[middle] ?? 0)) / 2;
}

function nearestRank(values: number[], percentile: number): number {
  if (!values.length) return 0;
  const sorted = values.slice().sort((left, right) => left - right);
  return sorted[Math.ceil((sorted.length - 1) * percentile)] ?? 0;
}

function verifyNoNumericRatio(receipt: Web06PeerCompactReceipt): void {
  if (receipt.packageAlignment !== "DATA_CONFOUNDED" || receipt.ratio.status !== "OMITTED") {
    throw new Error("Independent ratio policy rejection");
  }
  const walk = (value: unknown): boolean => typeof value === "number"
    || (Array.isArray(value) ? value.some(walk)
      : value !== null && typeof value === "object" && Object.values(value).some(walk));
  if (walk(receipt.ratio)) throw new Error("Independent verifier found a numeric ratio value");
  const keys = Object.keys(receipt.ratio);
  if (keys.some(keyName => /p\d+|value|numeric/i.test(keyName))) throw new Error("Numeric ratio field is present");
}

function verifyCompactIdentity(receipt: Web06PeerCompactReceipt): void {
  const sha40 = /^[0-9a-f]{40}$/;
  const sha64 = /^[0-9a-f]{64}$/;
  const source40 = [
    receipt.source.yuneCommit,
    receipt.source.yuneTree,
    receipt.source.peerUpstreamCommit,
    receipt.source.peerArtifactSourceCommit,
    receipt.source.peerArtifactSourceTree,
  ];
  const source64 = [receipt.source.yuneArchiveSha256, receipt.source.peerArchiveSha256];
  const contracts64 = Object.entries(receipt.contracts)
    .filter(([name]) => name !== "endpoint")
    .map(([, value]) => value);
  if (receipt.version !== "web06-phase4-peer-compact-v1"
      || receipt.laneVersion !== "web06-phase4-peer-lane-v1"
      || receipt.phase !== "phase4-peer"
      || receipt.contracts.endpoint !== "web06-comparator-endpoint-v1"
      || source40.some(value => !sha40.test(value))
      || [...source64, ...contracts64, receipt.environment.chromiumExecutableSha256,
        receipt.environment.environmentId].some(value => !sha64.test(value))) {
    throw new Error("Independent compact source/artifact/toolchain/config identity rejection");
  }
  const safeVersion = /^[A-Za-z0-9][A-Za-z0-9 .()+_-]{0,119}$/;
  if (![receipt.environment.nodeVersion, receipt.environment.npmVersion,
    receipt.environment.playwrightVersion, receipt.environment.chromiumVersion]
    .every(value => safeVersion.test(value))
      || !/^(darwin|linux|win32)$/.test(receipt.environment.platform)
      || !/^(arm64|x64)$/.test(receipt.environment.architecture)
      || canonical(receipt.environment.viewport) !== canonical({ width: 1365, height: 900 })
      || receipt.environment.locale !== "zh-HK"
      || receipt.environment.cacheRegime !== "fresh-persistent-profile-no-store-v1"
      || receipt.environment.cadenceMs !== 60) {
    throw new Error("Independent compact environment/toolchain identity rejection");
  }
  const expectedRows = rows.flatMap(row => apps.map(app => `${row.id}\0${app}`));
  const actualRows = receipt.rows.map(row => `${row.rowId}\0${row.app}`);
  if (canonical(actualRows) !== canonical(expectedRows)) {
    throw new Error("Independent compact row identity/order rejection");
  }
  for (const row of receipt.rows) {
    if (row.packageAlignment !== "DATA_CONFOUNDED"
        || row.rawAttemptPacketSha256s.length !== row.attemptsRetained
        || row.rawAttemptPacketSha256s.some(value => !sha64.test(value))
        || row.pairedValidAttemptOrdinals.some(value => !Number.isInteger(value) || value < 1 || value > 7)
        || [row.attemptsRetained, row.validRounds, row.validGreenRounds, row.validRedRounds,
          row.retainedHardRedAttempts, row.invalidHardRedAttempts,
          row.verdictBearingRedAttempts,
          row.setupInvalidAttempts, row.unpairedValidAttemptsRetained,
          row.cadence.driverGapSamples, row.cadence.inRangeGapSamples,
          row.cadence.tooShortGapSamples, row.cadence.delayedHostGapSamples]
          .some(value => !Number.isInteger(value) || value < 0)
        || row.verdictBearingRedAttempts > row.retainedHardRedAttempts
        || row.validRedRounds > row.verdictBearingRedAttempts
        || row.verdict !== (row.validRounds < 5
          ? "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS"
          : row.verdictBearingRedAttempts > 0 ? "RED" : "PASS")
        || !independentCompactCadenceValid(row.cadence)
        || [row.finalPrefix, row.allPrefixes, row.commit]
          .some(metric => !independentCompactMetricValid(metric))
        || !independentCompactFrameMetricValid(row.frameIntervals)
        || !independentCompactLongTaskValid(row.longTasks)
        || row.perRound.length !== row.attemptsRetained
        || row.perRound.some((round, index) =>
          round.attempt !== index + 1
          || !["VALID_GREEN", "VALID_RED", "SETUP_INVALID"].includes(round.status)
          || typeof round.retainedHardRed !== "boolean"
          || typeof round.verdictBearingRed !== "boolean"
          || typeof round.validForLatencyFrame !== "boolean"
          || !independentCompactMetricValid(round.finalPrefix)
          || !independentCompactMetricValid(round.allPrefixes)
          || !independentCompactMetricValid(round.commit)
          || !independentCompactCadenceValid(round.cadence)
          || !independentCompactFrameMetricValid(round.frameIntervals)
          || !independentCompactLongTaskValid(round.longTasks)
          || (round.frameIntervals === null) !== (round.longTasks === null)
          || !sha64.test(round.rawAttemptPacketSha256)
          || round.rawAttemptPacketSha256 !== row.rawAttemptPacketSha256s[index])) {
      throw new Error("Independent compact row identity/hash/metric rejection");
    }
  }
  if (receipt.ratio.reasons.length === 0
      || receipt.ratio.reasons.some(reason => !/^[a-z0-9:-]+$/.test(reason))) {
    throw new Error("Independent compact alignment reason rejection");
  }
}

function independentVerdictBearingRed(attempt: Web06PeerAttempt): boolean {
  return attempt.measurementStarted && (
    attempt.behaviorRedReasons.length > 0
    || (attempt.validForLatencyFrame && attempt.latencyRedReasons.length > 0)
  );
}

function independentCompactMetricValid(
  metric: Web06PeerCompactReceipt["rows"][number]["finalPrefix"],
): boolean {
  return metric === null || (
    Number.isInteger(metric.samples) && metric.samples >= 1
    && [metric.medianMs, metric.p95Ms, metric.p99Ms, metric.maxMs]
      .every(value => Number.isFinite(value) && value >= 0)
    && metric.medianMs <= metric.p95Ms
    && metric.p95Ms <= metric.p99Ms
    && metric.p99Ms <= metric.maxMs
  );
}

function independentCompactFrameMetricValid(
  metric: Web06PeerCompactReceipt["rows"][number]["frameIntervals"],
): boolean {
  return metric === null || (
    independentCompactMetricValid(metric)
    && Number.isInteger(metric.atLeast50MsCount)
    && metric.atLeast50MsCount >= 0
    && metric.atLeast50MsCount <= metric.samples
  );
}

function independentCompactCadenceValid(
  cadence: Web06PeerCompactReceipt["rows"][number]["cadence"],
): boolean {
  return [cadence.driverGapSamples, cadence.inRangeGapSamples,
    cadence.tooShortGapSamples, cadence.delayedHostGapSamples]
    .every(value => Number.isInteger(value) && value >= 0)
    && cadence.driverGapSamples === cadence.inRangeGapSamples
      + cadence.tooShortGapSamples + cadence.delayedHostGapSamples;
}

function independentCompactLongTaskValid(
  tasks: Web06PeerCompactReceipt["rows"][number]["longTasks"],
): boolean {
  if (tasks === null) return true;
  const counts = [tasks.frameWindows, tasks.observedCount, tasks.interactionOverlapCount,
    tasks.idleOverlapCount, tasks.outsideWindowCount];
  const durations = [tasks.interactionWindowDurationTotalMs, tasks.observedDurationTotalMs,
    tasks.interactionTaskDurationTotalMs, tasks.interactionTaskDurationMaxMs];
  return counts.every(value => Number.isInteger(value) && value >= 0)
    && tasks.frameWindows >= 1
    && durations.every(value => Number.isFinite(value) && value >= 0)
    && tasks.interactionTaskDurationMaxMs <= tasks.interactionTaskDurationTotalMs
    && (tasks.interactionOverlapCount !== 0
      || (tasks.interactionTaskDurationTotalMs === 0
        && tasks.interactionTaskDurationMaxMs === 0));
}

function verifyPrivacy(value: unknown): void {
  for (const item of stringValues(value)) {
    if (item.startsWith("/")
        || /^[A-Za-z]:[\\/]/.test(item)
        || /^\\\\/.test(item)
        || /^\/\/[^/]/.test(item)
        || /^file:\/\//i.test(item)
        || /YUNE_WEB06_[A-Z0-9_]+/.test(item)
        || /(?:token|password|secret|authorization)["'=:\s]+\S+/i.test(item)) {
      throw new Error("Independent public privacy rejection");
    }
  }
}

function stringValues(value: unknown): string[] {
  if (typeof value === "string") return [value];
  if (Array.isArray(value)) return value.flatMap(stringValues);
  return value !== null && typeof value === "object"
    ? Object.values(value as Record<string, unknown>).flatMap(stringValues)
    : [];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function key(attempt: Pick<Web06PeerAttempt, "rowId" | "app" | "attempt">): string {
  return `${attempt.rowId.replaceAll("/", "--")}--${attempt.app}--attempt-${String(attempt.attempt).padStart(2, "0")}`;
}

function argument(name: string): string {
  const index = process.argv.indexOf(name);
  const value = index >= 0 ? process.argv[index + 1] : undefined;
  if (!value || value.startsWith("--")) throw new Error(`Missing ${name}`);
  return path.resolve(value);
}

function validateCliArguments(args: string[]): void {
  const outputIndexes = args.flatMap((value, index) => value === "--output-root" ? [index] : []);
  const verifyOnlyCount = args.filter(value => value === "--verify-only").length;
  if (outputIndexes.length !== 1 || verifyOnlyCount > 1) {
    throw new Error("Verifier requires one --output-root and at most one --verify-only");
  }
  const outputIndex = outputIndexes[0] ?? -1;
  const expectedLength = verifyOnlyCount === 1 ? 3 : 2;
  if (args.length !== expectedLength || outputIndex + 1 >= args.length
      || args[outputIndex + 1]?.startsWith("--")) {
    throw new Error("Unknown, duplicate, or malformed verifier arguments");
  }
}

function canonical(value: unknown): string {
  const sort = (item: unknown): unknown => Array.isArray(item) ? item.map(sort)
    : item !== null && typeof item === "object"
      ? Object.fromEntries(Object.entries(item as Record<string, unknown>)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([name, child]) => [name, sort(child)]))
      : item;
  return `${JSON.stringify(sort(value), null, 2)}\n`;
}

function digest(value: string | Uint8Array): string {
  return createHash("sha256").update(value).digest("hex");
}
