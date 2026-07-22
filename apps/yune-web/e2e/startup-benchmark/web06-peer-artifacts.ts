import { chromium } from "@playwright/test";

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { lstat, mkdir, readFile, readdir, realpath, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import {
  evaluatePackageAlignment,
  parseComparatorIdentityManifest,
  type ComparatorIdentityManifest,
} from "./comparator-endpoint.ts";
import {
  canonicalJson,
  sha256,
  web06PeerAttestationVersion,
  web06PeerCacheRegime,
  web06PeerInputRegistrySha256,
  web06PeerLocale,
  web06PeerPinnedArchiveSha256,
  web06PeerPinnedDataManifestSha256,
  web06PeerPinnedManifestSha256,
  web06PeerPinnedPacketChecksumsSha256,
  web06PeerPinnedProjectionSha256,
  web06PeerPinnedTreeSha256,
  web06PeerPinnedUpstreamCommit,
  web06PeerSelectorManifestSha256,
  web06PeerViewport,
  type Web06PeerArtifactIdentity,
  type Web06PeerAttestation,
  type Web06PeerFinalRunEnvironment,
  type Web06PeerHostIdentity,
  type Web06PeerPostRunIntegrity,
  type Web06PeerToolchainIdentity,
} from "./web06-peer-lane.ts";

interface YuneBuildInfo {
  generatedFor: "yune-web";
  builtAt: string;
  sourceCommit: string;
  sourceTreeState: "clean";
  toolchain: {
    emsdkVersion: string;
    emscriptenReleaseCommit: string;
    emsdkRepositoryCommit: string;
    emccVersion: string;
    rustcVersion: string;
    nodeVersion: string;
  };
  schemaManifestSha256: string;
  wasmSha256: string;
  publicArtifactManifestSha256: string;
}

interface YunePublicManifest {
  generatedFor: "yune-web";
  version: string;
  files: Array<{ path: string; bytes: number; sha256: string }>;
}

interface PeerArtifactManifest {
  manifestVersion: 1;
  treeSha256: string;
  fileCount: number;
  files: Array<{ path: string; size: number; mode: string; sha256: string }>;
}

export interface Web06AttestedServingMember {
  path: string;
  size: number;
  sha256: string;
}

export class Web06ArtifactServingGuard {
  readonly #rootReal: string;
  readonly #members: ReadonlyMap<string, Readonly<Web06AttestedServingMember>>;
  #integrityFailure: Error | null = null;

  private constructor(
    rootReal: string,
    members: ReadonlyMap<string, Readonly<Web06AttestedServingMember>>,
  ) {
    this.#rootReal = rootReal;
    this.#members = members;
  }

  static async create(
    root: string,
    members: Iterable<Web06AttestedServingMember>,
  ): Promise<Web06ArtifactServingGuard> {
    const rootReal = await realpath(root);
    const copy = new Map<string, Readonly<Web06AttestedServingMember>>();
    for (const member of members) {
      const relative = safeArchivePath(member.path);
      if (!relative || copy.has(relative) || !Number.isInteger(member.size) || member.size < 0
          || !/^[0-9a-f]{64}$/.test(member.sha256)) {
        throw new Error(`Invalid/duplicate immutable serving member: ${member.path}`);
      }
      copy.set(relative, Object.freeze({ ...member, path: relative }));
    }
    if (copy.size === 0) throw new Error("Immutable serving manifest is empty");
    return new Web06ArtifactServingGuard(rootReal, copy);
  }

  async read(relativePath: string): Promise<Uint8Array | null> {
    const relative = safeArchivePath(relativePath);
    const expected = this.#members.get(relative);
    if (!expected) return null;
    try {
      const file = path.resolve(this.#rootReal, relative);
      const relation = path.relative(this.#rootReal, file);
      if (relation.startsWith("..") || path.isAbsolute(relation)) {
        throw new Error(`Serving member escapes root: ${relative}`);
      }
      let component = this.#rootReal;
      const segments = relative.split("/");
      for (const [index, segment] of segments.entries()) {
        component = path.join(component, segment);
        const componentInfo = await lstat(component);
        if (componentInfo.isSymbolicLink()
            || (index < segments.length - 1 && !componentInfo.isDirectory())) {
          throw new Error(`Serving member contains a link/non-directory component: ${relative}`);
        }
      }
      const before = await lstat(file);
      if (!before.isFile() || before.isSymbolicLink()) {
        throw new Error(`Serving member is not a regular non-link file: ${relative}`);
      }
      const fileReal = await realpath(file);
      const realRelation = path.relative(this.#rootReal, fileReal);
      if (realRelation.startsWith("..") || path.isAbsolute(realRelation)) {
        throw new Error(`Serving member resolves outside root: ${relative}`);
      }
      const bytes = await readFile(fileReal);
      const after = await lstat(file);
      const unchangedIdentity = before.dev === after.dev && before.ino === after.ino
        && before.size === after.size && before.mtimeMs === after.mtimeMs
        && before.ctimeMs === after.ctimeMs;
      if (!after.isFile() || after.isSymbolicLink() || !unchangedIdentity
          || bytes.byteLength !== expected.size || sha256(bytes) !== expected.sha256) {
        throw new Error(`Serving member changed from its immutable attestation: ${relative}`);
      }
      return bytes;
    } catch (error) {
      const failure = error instanceof Error ? error : new Error(String(error));
      this.#integrityFailure ??= failure;
      throw failure;
    }
  }

  assertIntegrity(): void {
    if (this.#integrityFailure) {
      throw new Error(`Sticky artifact serving integrity failure: ${this.#integrityFailure.message}`);
    }
  }
}

export async function loadAttestedServingMembers(
  attestation: Web06PeerAttestation,
  app: "yune-web" | "my-rime",
): Promise<Web06AttestedServingMember[]> {
  if (app === "yune-web") {
    const manifestFile = path.join(attestation.yuneRoot, "public-artifact-manifest.json");
    const buildInfoFile = path.join(attestation.yuneRoot, "build-info.json");
    await assertFileSha256(
      manifestFile,
      attestation.yune.completeManifestSha256,
      "attested Yune serving manifest",
    );
    await assertFileSha256(
      buildInfoFile,
      attestation.yune.buildInfoSha256 ?? "",
      "attested Yune serving build-info",
    );
    const manifest = JSON.parse(await readFile(manifestFile, "utf8")) as YunePublicManifest;
    if (manifest.generatedFor !== "yune-web" || !Array.isArray(manifest.files)) {
      throw new Error("Attested Yune serving manifest has the wrong contract");
    }
    const metadata = await Promise.all([manifestFile, buildInfoFile].map(async file => {
      const bytes = await readFile(file);
      return {
        path: path.basename(file),
        size: bytes.byteLength,
        sha256: sha256(bytes),
      };
    }));
    return [
      ...manifest.files.map(member => ({
        path: safeArchivePath(member.path),
        size: member.bytes,
        sha256: member.sha256,
      })),
      ...metadata,
    ];
  }
  const manifestFile = path.join(attestation.peerPacketRoot, "artifact-manifest.json");
  await assertFileSha256(
    manifestFile,
    attestation.peer.completeManifestSha256,
    "attested peer serving manifest",
  );
  const manifest = JSON.parse(await readFile(manifestFile, "utf8")) as PeerArtifactManifest;
  if (manifest.manifestVersion !== 1 || manifest.treeSha256 !== attestation.peer.treeSha256
      || manifest.fileCount !== attestation.peer.fileCount || !Array.isArray(manifest.files)) {
    throw new Error("Attested peer serving manifest has the wrong identity");
  }
  return manifest.files.map(member => ({
    path: safeArchivePath(member.path),
    size: member.size,
    sha256: member.sha256,
  }));
}

export interface AttestWeb06PeerArtifactsOptions {
  mode: "binding" | "setup-only";
  outputRoot: string;
  repoRoot: string;
  yuneArchive: string;
  yuneArchiveSha256: string;
  yuneSourceTree: string;
  yuneBuildInfoSha256: string;
  yunePublicManifestSha256: string;
  yuneBuildNpmVersion: string;
  peerArchive: string;
  peerPacketRoot: string;
  finalRunEnvironmentPath: string;
  finalRunEnvironmentSha256: string;
  sourceFiles: {
    endpoint: string;
    browserEndpoint: string;
    lane: string;
    artifactAttestor: string;
    launcher: string;
    independentLogic: string;
    spec: string;
    config: string;
    verifier: string;
  };
  allowProvisionalSetupSourceMismatch: boolean;
}

export interface AttestWeb06PeerArtifactsResult {
  attestation: Web06PeerAttestation;
  liveEnvironmentDrift: string[];
  attestationText: string;
  negativeEssayControlText: string;
}

export function web06PeerSourceFiles(repoRoot: string): AttestWeb06PeerArtifactsOptions["sourceFiles"] {
  const e2eRoot = path.join(repoRoot, "apps", "yune-web", "e2e");
  return {
    endpoint: path.join(e2eRoot, "startup-benchmark", "comparator-endpoint.ts"),
    browserEndpoint: path.join(e2eRoot, "startup-benchmark", "comparator-browser-endpoint.ts"),
    lane: path.join(e2eRoot, "startup-benchmark", "web06-peer-lane.ts"),
    artifactAttestor: path.join(e2eRoot, "startup-benchmark", "web06-peer-artifacts.ts"),
    launcher: path.join(e2eRoot, "run-web06-peer-lane.ts"),
    independentLogic: path.join(e2eRoot, "startup-benchmark", "web06-peer-independent.ts"),
    spec: path.join(e2eRoot, "web06-peer-phase4.spec.ts"),
    config: path.join(e2eRoot, "playwright.web06-peer.config.ts"),
    verifier: path.join(e2eRoot, "verify-web06-peer-evidence.ts"),
  };
}

export async function capturePostRunIntegrity(
  attestation: Web06PeerAttestation,
  attestationText: string,
): Promise<Web06PeerPostRunIntegrity> {
  const repoIdentity = repositoryIdentity(attestation.runnerRoot);
  if (!repoIdentity.clean || repoIdentity.commit !== attestation.yune.sourceCommit
      || repoIdentity.tree !== attestation.yune.sourceTree) {
    throw new Error("Post-run runner source is dirty or differs from the FINAL artifact source");
  }
  const sourceFiles = web06PeerSourceFiles(attestation.runnerRoot);
  await assertAttestedSourceFileHashes(sourceFiles, attestation.toolchain);
  const toolchain = await liveToolchainIdentity(attestation.runnerRoot, sourceFiles, "binding");
  toolchain.runnerSourceCommit = repoIdentity.commit;
  toolchain.runnerSourceTree = repoIdentity.tree;
  toolchain.runnerSourceTreeState = "clean";
  if (canonicalJson(toolchain) !== canonicalJson(attestation.toolchain)) {
    throw new Error("Post-run runner/parser/endpoint/config/verifier/lock/toolchain bytes changed");
  }
  const verificationOptions = {
    mode: "binding" as const,
    outputRoot: attestation.outputRoot,
    repoRoot: attestation.runnerRoot,
    yuneArchive: attestation.yuneArchivePath,
    yuneArchiveSha256: attestation.yune.archiveSha256,
    yuneSourceTree: attestation.yune.sourceTree,
    yuneBuildInfoSha256: attestation.yune.buildInfoSha256 ?? "",
    yunePublicManifestSha256: attestation.yune.completeManifestSha256,
    yuneBuildNpmVersion: attestation.identityManifest.yune.packageManager.version,
    peerArchive: attestation.peerArchivePath,
    peerPacketRoot: attestation.peerPacketRoot,
    finalRunEnvironmentPath: "unused-post-run",
    finalRunEnvironmentSha256: attestation.finalRunEnvironmentSha256,
    sourceFiles,
    allowProvisionalSetupSourceMismatch: false,
  };
  const yune = await verifyYuneArtifact(verificationOptions, attestation.yuneRoot);
  const peer = await verifyPeerArtifact(
    attestation.peerArchivePath,
    attestation.peerPacketRoot,
    attestation.peerRoot,
  );
  if (canonicalJson(yune) !== canonicalJson(attestation.yune)
      || canonicalJson(peer) !== canonicalJson(attestation.peer)) {
    throw new Error("Post-run archive/extracted artifact identity changed");
  }
  const liveHost = await liveHostIdentity();
  if (environmentDrift(attestation.finalRunEnvironment, toolchain, liveHost).length > 0) {
    throw new Error("Post-run host/toolchain environment drifted from FINAL");
  }
  return {
    version: "web06-phase4-peer-post-run-integrity-v1",
    benchmarkAttempt: true,
    attestationSha256: sha256(attestationText),
    runnerSourceCommit: repoIdentity.commit,
    runnerSourceTree: repoIdentity.tree,
    runnerSourceTreeState: "clean",
    toolchainIdentitySha256: sha256(canonicalJson(toolchain)),
    yuneArtifactIdentitySha256: sha256(canonicalJson(yune)),
    peerArtifactIdentitySha256: sha256(canonicalJson(peer)),
    finalRunEnvironmentId: attestation.finalRunEnvironment.environmentId,
  };
}

export async function assertAttestedSourceFileHashes(
  sourceFiles: AttestWeb06PeerArtifactsOptions["sourceFiles"],
  toolchain: Pick<
    Web06PeerToolchainIdentity,
    | "endpointSourceSha256"
    | "browserEndpointSourceSha256"
    | "laneSourceSha256"
    | "artifactAttestorSourceSha256"
    | "launcherSourceSha256"
    | "independentLogicSourceSha256"
    | "specSourceSha256"
    | "configSourceSha256"
    | "verifierSourceSha256"
  >,
): Promise<void> {
  const expected = {
    endpoint: toolchain.endpointSourceSha256,
    browserEndpoint: toolchain.browserEndpointSourceSha256,
    lane: toolchain.laneSourceSha256,
    artifactAttestor: toolchain.artifactAttestorSourceSha256,
    launcher: toolchain.launcherSourceSha256,
    independentLogic: toolchain.independentLogicSourceSha256,
    spec: toolchain.specSourceSha256,
    config: toolchain.configSourceSha256,
    verifier: toolchain.verifierSourceSha256,
  } as const;
  for (const [name, file] of Object.entries(sourceFiles)) {
    const actual = sha256(await readFile(file));
    if (actual !== expected[name as keyof typeof expected]) {
      throw new Error(`Attested runner source changed after snapshot: ${name}`);
    }
  }
}

export async function resolveCreateNewExternalOutputRoot(
  repoRoot: string,
  requestedOutputRoot: string,
): Promise<{ repoReal: string; outputRoot: string }> {
  const repoReal = await realpath(repoRoot);
  const requested = path.resolve(requestedOutputRoot);
  const parentReal = await realpath(path.dirname(requested));
  assertOutsideRepository(repoReal, parentReal, "output parent");
  return { repoReal, outputRoot: path.join(parentReal, path.basename(requested)) };
}

export async function verifyCreatedExternalOutputRoot(
  repoReal: string,
  outputRoot: string,
): Promise<void> {
  const outputReal = await realpath(outputRoot);
  assertOutsideRepository(repoReal, outputReal, "created output root");
}

function assertOutsideRepository(repoReal: string, candidateReal: string, label: string): void {
  const relation = path.relative(repoReal, candidateReal);
  if (relation === "" || (!relation.startsWith("..") && !path.isAbsolute(relation))) {
    throw new Error(`Raw WEB-06 peer ${label} resolves inside the tracked repository`);
  }
}

export async function attestWeb06PeerArtifacts(
  options: AttestWeb06PeerArtifactsOptions,
): Promise<AttestWeb06PeerArtifactsResult> {
  assertSha256("Yune archive", options.yuneArchiveSha256);
  assertSha256("Yune source tree", options.yuneSourceTree, 40);
  assertSha256("Yune build-info", options.yuneBuildInfoSha256);
  assertSha256("Yune public manifest", options.yunePublicManifestSha256);
  assertSha256("FINAL run environment", options.finalRunEnvironmentSha256);
  const extractionRoot = path.join(options.outputRoot, "extracted");
  await mkdir(extractionRoot, { recursive: false, mode: 0o700 });
  const yuneExtractRoot = path.join(extractionRoot, "yune");
  const peerExtractRoot = path.join(extractionRoot, "peer");
  await safeExtractTar(options.yuneArchive, yuneExtractRoot);
  await safeExtractTar(options.peerArchive, peerExtractRoot);

  const yuneRoot = await findYuneRoot(yuneExtractRoot);
  const peerRoot = await findPeerRoot(peerExtractRoot);
  const yune = await verifyYuneArtifact(options, yuneRoot);
  const peer = await verifyPeerArtifact(options.peerArchive, options.peerPacketRoot, peerRoot);
  const finalRunEnvironment = await loadFinalRunEnvironment(
    options.finalRunEnvironmentPath,
    options.finalRunEnvironmentSha256,
  );
  const toolchain = await liveToolchainIdentity(options.repoRoot, options.sourceFiles, options.mode);
  const liveHost = await liveHostIdentity();
  const liveEnvironmentDrift = environmentDrift(finalRunEnvironment, toolchain, liveHost);
  if (options.mode === "binding" && liveEnvironmentDrift.length > 0) {
    throw new Error(`Binding peer run environment drifted from FINAL: ${liveEnvironmentDrift.join("; ")}`);
  }

  const repoIdentity = repositoryIdentity(options.repoRoot);
  const sourceMismatch = [
    ...(repoIdentity.commit === yune.sourceCommit ? [] : [`source-commit:${repoIdentity.commit}!=${yune.sourceCommit}`]),
    ...(repoIdentity.tree === yune.sourceTree ? [] : [`source-tree:${repoIdentity.tree}!=${yune.sourceTree}`]),
    ...(repoIdentity.clean ? [] : ["source-tree-dirty"]),
  ];
  if (sourceMismatch.length > 0
      && !(options.mode === "setup-only" && options.allowProvisionalSetupSourceMismatch)) {
    throw new Error(`Peer lane source is not the sealed Yune source: ${sourceMismatch.join("; ")}`);
  }
  toolchain.runnerSourceCommit = repoIdentity.commit;
  toolchain.runnerSourceTree = repoIdentity.tree;
  toolchain.runnerSourceTreeState = sourceMismatch.length === 0 ? "clean" : "provisional-setup-only";

  const { identityManifest, negativeEssayControl } = await phase4IdentityManifest(
    options,
    yune,
    yuneRoot,
  );
  const identityText = canonicalJson(identityManifest);
  const negativeEssayControlText = canonicalJson(negativeEssayControl);
  const identityManifestSha256 = sha256(identityText);
  const config = {
    lane: "WEB-06 Phase 4 source-pinned peer",
    mode: options.mode,
    endpointContract: "web06-comparator-endpoint-v1",
    oneChromiumExecutable: toolchain.chromiumExecutableSha256,
    browserMode: finalRunEnvironment.browserMode,
    samePhysicalHost: finalRunEnvironment.environmentId,
    viewport: web06PeerViewport,
    locale: web06PeerLocale,
    cacheRegime: web06PeerCacheRegime,
    inputRegistrySha256: web06PeerInputRegistrySha256(),
    selectorManifestSha256: web06PeerSelectorManifestSha256(),
    freshProfileEveryAttempt: true,
    pageSize: 6,
    cadenceMs: 60,
    validPairedRoundsPerRow: 5,
    maxRetainedPairsPerRow: 7,
    counterbalancing: "odd-yune-first-even-peer-first",
    validRedsRetainedAndCounted: true,
    packageAlignment: "DATA_CONFOUNDED",
    ratioStatus: "OMITTED",
  };
  const attestation: Web06PeerAttestation = {
    version: web06PeerAttestationVersion,
    phase: "phase4-peer",
    mode: options.mode,
    benchmarkAttempt: options.mode === "binding",
    createdAt: new Date().toISOString(),
    runnerRoot: options.repoRoot,
    outputRoot: options.outputRoot,
    yuneArchivePath: options.yuneArchive,
    peerArchivePath: options.peerArchive,
    yuneRoot,
    peerRoot,
    peerPacketRoot: options.peerPacketRoot,
    yune,
    peer,
    toolchain,
    host: finalRunEnvironment.host,
    finalRunEnvironment,
    finalRunEnvironmentSha256: options.finalRunEnvironmentSha256,
    identityManifest,
    identityManifestSha256,
    negativeEssayControlSha256: sha256(negativeEssayControlText),
    configSha256: sha256(canonicalJson(config)),
    inputRegistrySha256: web06PeerInputRegistrySha256(),
    selectorManifestSha256: web06PeerSelectorManifestSha256(),
    extraction: {
      yune: "SAFE_EXTRACTED_AND_FULLY_RECONCILED",
      peer: "SAFE_EXTRACTED_AND_FULLY_RECONCILED",
    },
  };
  const attestationText = canonicalJson({ ...attestation, liveEnvironmentDrift, sourceMismatch });
  return { attestation, liveEnvironmentDrift, attestationText, negativeEssayControlText };
}

export async function safeExtractTar(archive: string, destination: string): Promise<void> {
  const archivePath = path.resolve(archive);
  const archiveStat = await stat(archivePath);
  if (!archiveStat.isFile()) throw new Error(`Archive is not a regular file: ${archivePath}`);
  const names = execFileSync("tar", ["-tf", archivePath], { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 })
    .split("\n").filter(Boolean);
  const verbose = execFileSync("tar", ["-tvf", archivePath], { encoding: "utf8", maxBuffer: 32 * 1024 * 1024 })
    .split("\n").filter(Boolean);
  if (names.length === 0 || names.length !== verbose.length) {
    throw new Error("Tar archive listing is empty or internally inconsistent");
  }
  const seen = new Set<string>();
  for (const [index, rawName] of names.entries()) {
    const normalized = safeArchivePath(rawName);
    if (normalized !== "") {
      if (seen.has(normalized)) throw new Error(`Tar archive contains a duplicate path: ${normalized}`);
      seen.add(normalized);
    }
    const kind = verbose[index]?.at(0);
    if (kind !== "-" && kind !== "d") {
      throw new Error(`Tar archive contains a forbidden non-file entry: ${rawName}`);
    }
  }
  await mkdir(destination, { recursive: false, mode: 0o700 });
  execFileSync("tar", ["-xf", archivePath, "-C", destination], { stdio: "pipe", maxBuffer: 32 * 1024 * 1024 });
  await assertSafeExtractedTree(destination);
}

async function verifyYuneArtifact(
  options: AttestWeb06PeerArtifactsOptions,
  root: string,
): Promise<Web06PeerArtifactIdentity> {
  await assertFileSha256(options.yuneArchive, options.yuneArchiveSha256, "Yune archive");
  const buildInfoFile = path.join(root, "build-info.json");
  const manifestFile = path.join(root, "public-artifact-manifest.json");
  await assertFileSha256(buildInfoFile, options.yuneBuildInfoSha256, "Yune build-info");
  await assertFileSha256(manifestFile, options.yunePublicManifestSha256, "Yune public manifest");
  const buildInfo = JSON.parse(await readFile(buildInfoFile, "utf8")) as YuneBuildInfo;
  const manifest = JSON.parse(await readFile(manifestFile, "utf8")) as YunePublicManifest;
  if (buildInfo.generatedFor !== "yune-web" || buildInfo.sourceTreeState !== "clean") {
    throw new Error("Yune build-info does not identify a clean yune-web public artifact");
  }
  if (buildInfo.publicArtifactManifestSha256 !== options.yunePublicManifestSha256) {
    throw new Error("Yune build-info/public-manifest hash mismatch");
  }
  if (manifest.generatedFor !== "yune-web" || !Array.isArray(manifest.files)) {
    throw new Error("Yune public artifact manifest has the wrong contract");
  }
  const expected = new Map(manifest.files.map(file => [safeArchivePath(file.path), file]));
  if (expected.size !== manifest.files.length) throw new Error("Yune public manifest contains duplicate paths");
  const actualFiles = await regularFiles(root);
  const allowedUnlisted = new Set(["build-info.json", "public-artifact-manifest.json"]);
  const unlisted = actualFiles.filter(file => !expected.has(file) && !allowedUnlisted.has(file));
  const missing = [...expected.keys()].filter(file => !actualFiles.includes(file));
  const missingAttestation = [...allowedUnlisted].filter(file => !actualFiles.includes(file));
  if (unlisted.length || missing.length || missingAttestation.length) {
    throw new Error(`Yune artifact file reconciliation failed: unlisted=${unlisted}; missing=${missing}; attestation=${missingAttestation}`);
  }
  for (const [relative, item] of expected) {
    const file = path.join(root, relative);
    const info = await stat(file);
    if (info.size !== item.bytes) throw new Error(`Yune artifact size mismatch: ${relative}`);
    await assertFileSha256(file, item.sha256, `Yune artifact ${relative}`);
  }
  await assertFileSha256(path.join(root, "schema-asset-manifest.json"), buildInfo.schemaManifestSha256, "Yune schema manifest");
  await assertFileSha256(path.join(root, "yune-web.wasm"), buildInfo.wasmSha256, "Yune WASM");
  const derivedSourceTree = verifyArtifactSourceTree(
    options.repoRoot,
    buildInfo.sourceCommit,
    options.yuneSourceTree,
  );
  const treeSha256 = await genericTreeSha256(root, actualFiles);
  return {
    archiveSha256: options.yuneArchiveSha256,
    completeManifestSha256: options.yunePublicManifestSha256,
    treeSha256,
    fileCount: actualFiles.length,
    sourceCommit: buildInfo.sourceCommit,
    sourceTree: derivedSourceTree,
    sourceTreeState: "clean",
    buildInfoSha256: options.yuneBuildInfoSha256,
    schemaManifestSha256: buildInfo.schemaManifestSha256,
    wasmSha256: buildInfo.wasmSha256,
  };
}

async function verifyPeerArtifact(
  archive: string,
  packetRoot: string,
  root: string,
): Promise<Web06PeerArtifactIdentity> {
  await assertFileSha256(archive, web06PeerPinnedArchiveSha256, "pinned My RIME archive");
  await verifyPeerPacketChecksums(packetRoot);
  const manifestFile = path.join(packetRoot, "artifact-manifest.json");
  await assertFileSha256(manifestFile, web06PeerPinnedManifestSha256, "pinned My RIME manifest");
  await assertFileSha256(path.join(packetRoot, "web06-peer-data-v1.json"), web06PeerPinnedDataManifestSha256, "pinned My RIME data manifest");
  await assertFileSha256(path.join(packetRoot, "web06-comparator-projection-phase0-product.json"), web06PeerPinnedProjectionSha256, "pinned My RIME projection");
  const manifest = JSON.parse(await readFile(manifestFile, "utf8")) as PeerArtifactManifest;
  if (manifest.manifestVersion !== 1 || manifest.treeSha256 !== web06PeerPinnedTreeSha256) {
    throw new Error("Pinned My RIME artifact manifest identity mismatch");
  }
  const actualFiles = await regularFiles(root);
  const expected = new Map(manifest.files.map(file => [safeArchivePath(file.path), file]));
  if (actualFiles.length !== manifest.fileCount || expected.size !== manifest.fileCount) {
    throw new Error("Pinned My RIME artifact file-count mismatch");
  }
  if (actualFiles.some(file => !expected.has(file)) || [...expected.keys()].some(file => !actualFiles.includes(file))) {
    throw new Error("Pinned My RIME artifact full file-list mismatch");
  }
  const rows: string[] = [];
  for (const [relative, item] of expected) {
    const file = path.join(root, relative);
    const info = await stat(file);
    const mode = (info.mode & 0o777).toString(8);
    if (info.size !== item.size || mode !== item.mode) throw new Error(`Pinned My RIME metadata mismatch: ${relative}`);
    await assertFileSha256(file, item.sha256, `pinned My RIME ${relative}`);
    rows.push(`${relative}\0${item.size}\0${item.mode}\0${item.sha256}\n`);
  }
  const tree = sha256(rows.sort().join(""));
  if (tree !== manifest.treeSha256) throw new Error("Pinned My RIME artifact tree digest mismatch");
  const projection = parseComparatorIdentityManifest(
    await readFile(path.join(packetRoot, "web06-comparator-projection-phase0-product.json"), "utf8"),
  );
  if (projection.peer.upstreamPinnedCommit !== web06PeerPinnedUpstreamCommit) {
    throw new Error("Pinned My RIME upstream source commit mismatch");
  }
  return {
    archiveSha256: web06PeerPinnedArchiveSha256,
    completeManifestSha256: web06PeerPinnedManifestSha256,
    treeSha256: manifest.treeSha256,
    fileCount: manifest.fileCount,
    sourceCommit: projection.peer.artifactSourceCommit,
    sourceTree: projection.peer.artifactSourceTree,
    sourceTreeState: "clean",
    wasmSha256: projection.peer.compiledHashes.runtime,
  };
}

async function phase4IdentityManifest(
  options: AttestWeb06PeerArtifactsOptions,
  yune: Web06PeerArtifactIdentity,
  yuneRoot: string,
): Promise<{ identityManifest: ComparatorIdentityManifest; negativeEssayControl: ComparatorIdentityManifest }> {
  const projectionFile = path.join(options.peerPacketRoot, "web06-comparator-projection-phase0-product.json");
  const projection = JSON.parse(await readFile(projectionFile, "utf8")) as ComparatorIdentityManifest & {
    projectionStage?: string;
    projectionProvenance?: Record<string, unknown>;
  };
  const frozenYuneInputs: Array<[string, string]> = [
    ["schema/default.yaml", "31e321168824e1e5cbd5f1cca1dfa32a9daa951925e7d35ef6425dd6dd4bc9fa"],
    ["schema/build/default.yaml", "e078b77f10d89af546dba7874886e772b708dc4208e1be90cd609fe9d736784c"],
    ["schema/luna_pinyin.dict.yaml", "69c7142fcd67cf070677c6435333a8a35e2fc1232d4f0851ef8bac31324bf7ee"],
    ["schema/luna_pinyin.schema.yaml", "668b4d4957e4cc8f9cea32ed3f08f0ecf4a7be8cdba058700507487bd96dd5c7"],
    ["schema/pinyin.yaml", "caa220b05172bf35775db07fed6ead7719c742e6139232d9797040dbe65f746c"],
    ["schema/essay.txt", "09086a44204f469d2c16ad72784e1f567a6f016570dfc9aa79f868267a9c1385"],
    ["schema/luna_pinyin.table.bin", "f931cca5a3ed44f03b15f1d46171b459314b5bfba5404452fb15cf5c1706387f"],
    ["schema/luna_pinyin.prism.bin", "7fac0de0a1757b870447b25ea4316db2cb928251c015aa6e1465ea1725669eff"],
    ["schema/luna_pinyin.reverse.bin", "7d737f2738ed871df05aa1d0e25abae1b0737e8794f4e41fc2fe985a8ff14319"],
  ];
  for (const [relative, digest] of frozenYuneInputs) {
    await assertFileSha256(path.join(yuneRoot, relative), digest, `frozen Yune Luna input ${relative}`);
  }
  const buildInfo = JSON.parse(await readFile(path.join(yuneRoot, "build-info.json"), "utf8")) as YuneBuildInfo;
  projection.projectionStage = "phase4-final";
  projection.yune = {
    ...projection.yune,
    repositoryCommit: yune.sourceCommit,
    upstreamPinnedCommit: yune.sourceCommit,
    artifactSourceCommit: yune.sourceCommit,
    artifactSourceTree: yune.sourceTree,
    sourceTreeState: "clean",
    artifactSha256: yune.archiveSha256,
    generatedManifestSha256: buildInfo.schemaManifestSha256,
    completeArtifactManifestSha256: yune.completeManifestSha256,
    packageManager: {
      ...projection.yune.packageManager,
      version: options.yuneBuildNpmVersion,
    },
    toolchain: {
      nodeVersion: buildInfo.toolchain.nodeVersion,
      emscriptenVersion: buildInfo.toolchain.emsdkVersion,
      emscriptenCommit: buildInfo.toolchain.emscriptenReleaseCommit,
      compilerVersion: `${buildInfo.toolchain.emccVersion}; ${buildInfo.toolchain.rustcVersion}`,
    },
    compiledHashes: {
      table: frozenYuneInputs[6]?.[1] ?? "",
      prism: frozenYuneInputs[7]?.[1] ?? "",
      reverse: frozenYuneInputs[8]?.[1] ?? "",
      "data-model": "none",
      runtime: yune.wasmSha256,
    },
  };
  projection.sameEndpointObserver = true;
  projection.projectionProvenance = {
    ...(projection.projectionProvenance ?? {}),
    projectionOnly: true,
    declaredAlignment: "DATA_CONFOUNDED",
    bindingRatioEligible: false,
    ratioFormation: "REFUSED",
    phase4Final: {
      sourceCommit: yune.sourceCommit,
      sourceTree: yune.sourceTree,
      archiveSha256: yune.archiveSha256,
      artifactManifestSha256: yune.completeManifestSha256,
      buildInfoSha256: yune.buildInfoSha256,
      endpointObserver: "same reviewed comparator endpoint for both apps",
    },
    confounders: [
      "DEFAULT_CONFIG_MISMATCH",
      "STARTUP_OPTIONS_NOT_EQUIVALENT",
      "COMMENT_POSTURE_NOT_INDEPENDENTLY_PROVED",
    ],
  };
  const identityManifest = parseComparatorIdentityManifest(canonicalJson(projection));
  const alignment = evaluatePackageAlignment(identityManifest);
  if (alignment.packageAlignment !== "DATA_CONFOUNDED") {
    throw new Error("Phase-4 projection incorrectly made the sealed peer ratio-eligible");
  }
  const negativeEssayControl = structuredClone(identityManifest);
  const essay = negativeEssayControl.logicalInputs.find(item => item.id === "essay");
  if (!essay) throw new Error("Phase-4 peer identity omitted essay input");
  essay.peerSha256 = "0".repeat(64);
  const negativeAlignment = evaluatePackageAlignment(negativeEssayControl);
  if (negativeAlignment.packageAlignment !== "DATA_CONFOUNDED"
      || !negativeAlignment.reasons.includes("logical-input-different:essay")) {
    throw new Error("Essay negative control did not refuse alignment before measurement");
  }
  return { identityManifest, negativeEssayControl };
}

async function loadFinalRunEnvironment(
  file: string,
  expectedSha256: string,
): Promise<Web06PeerFinalRunEnvironment> {
  await assertFileSha256(file, expectedSha256, "FINAL run-environment manifest");
  const text = await readFile(file, "utf8");
  const value = JSON.parse(text) as Web06PeerFinalRunEnvironment;
  if (text !== canonicalJson(value)) throw new Error("FINAL run-environment manifest is not canonical JSON");
  if (value.version !== "web06-final-run-environment-v1") throw new Error("Wrong FINAL environment contract");
  const { environmentId, ...identity } = value;
  if (environmentId !== sha256(canonicalJson(identity))) {
    throw new Error("FINAL environmentId does not match its canonical manifest");
  }
  if (value.host.powerSource !== "AC Power" || value.host.lowPowerMode !== false
      || value.host.display.refreshRateHz !== 60
      || value.browserMode !== "headed-foreground"
      || canonicalJson(value.viewport) !== canonicalJson(web06PeerViewport)
      || value.locale !== web06PeerLocale || value.cacheRegime !== web06PeerCacheRegime
      || !value.nodeVersion || !value.playwrightVersion || !value.chromiumVersion
      || !/^[0-9a-f]{64}$/.test(value.chromiumExecutableSha256)) {
    throw new Error("FINAL environment manifest violates the frozen AC/60Hz/toolchain/viewport/cache contract");
  }
  return value;
}

async function liveToolchainIdentity(
  repoRoot: string,
  sourceFiles: AttestWeb06PeerArtifactsOptions["sourceFiles"],
  mode: "binding" | "setup-only",
): Promise<Web06PeerToolchainIdentity> {
  const executable = chromium.executablePath();
  const chromiumVersion = execFileSync(executable, ["--version"], { encoding: "utf8" }).trim();
  const playwrightPackage = JSON.parse(await readFile(
    path.join(repoRoot, "apps/yune-web/e2e/node_modules/@playwright/test/package.json"),
    "utf8",
  )) as { version: string };
  const sourceDigest = async (file: string) => sha256(await readFile(file));
  return {
    runnerSourceCommit: "pending",
    runnerSourceTree: "pending",
    runnerSourceTreeState: mode === "binding" ? "clean" : "provisional-setup-only",
    nodeVersion: process.version,
    npmVersion: execFileSync("npm", ["--version"], { encoding: "utf8" }).trim(),
    playwrightVersion: playwrightPackage.version,
    playwrightPackageLockSha256: await sourceDigest(path.join(repoRoot, "apps/yune-web/e2e/package-lock.json")),
    chromiumVersion,
    chromiumExecutableSha256: await sourceDigest(executable),
    endpointSourceSha256: await sourceDigest(sourceFiles.endpoint),
    browserEndpointSourceSha256: await sourceDigest(sourceFiles.browserEndpoint),
    laneSourceSha256: await sourceDigest(sourceFiles.lane),
    artifactAttestorSourceSha256: await sourceDigest(sourceFiles.artifactAttestor),
    launcherSourceSha256: await sourceDigest(sourceFiles.launcher),
    independentLogicSourceSha256: await sourceDigest(sourceFiles.independentLogic),
    specSourceSha256: await sourceDigest(sourceFiles.spec),
    configSourceSha256: await sourceDigest(sourceFiles.config),
    verifierSourceSha256: await sourceDigest(sourceFiles.verifier),
  };
}

async function liveHostIdentity(): Promise<Web06PeerHostIdentity> {
  if (process.platform !== "darwin") throw new Error("WEB-06 binding peer lane is pinned to the declared Mac host");
  const battery = execFileSync("pmset", ["-g", "batt"], { encoding: "utf8" });
  const powerSource = /Now drawing from 'AC Power'/.test(battery) ? "AC Power" : "Battery Power";
  const powerSettings = execFileSync("pmset", ["-g", "custom"], { encoding: "utf8" });
  const lowPowerMatches = [...powerSettings.matchAll(/lowpowermode\s+(\d+)/g)].map(match => Number(match[1]));
  const lowPowerMode = lowPowerMatches.some(value => value !== 0);
  const displayJson = JSON.parse(execFileSync(
    "system_profiler", ["SPDisplaysDataType", "-json"], { encoding: "utf8", maxBuffer: 8 * 1024 * 1024 },
  )) as unknown;
  const resolutions = findStringValues(displayJson, "_spdisplays_resolution");
  const matched = resolutions.map(value => /^(\d+) x (\d+) @ (\d+(?:\.\d+)?)Hz$/.exec(value)).find(Boolean);
  if (!matched) throw new Error("Could not attest the active display resolution/refresh rate");
  const cpus = os.cpus();
  return {
    platform: process.platform,
    release: os.release(),
    architecture: process.arch,
    cpuModel: cpus[0]?.model ?? "unknown",
    logicalCoreCount: cpus.length,
    totalMemoryBytes: os.totalmem(),
    powerSource: powerSource as "AC Power",
    lowPowerMode: lowPowerMode as false,
    display: {
      width: Number(matched[1]),
      height: Number(matched[2]),
      refreshRateHz: Number(matched[3]) as 60,
    },
  };
}

function environmentDrift(
  expected: Web06PeerFinalRunEnvironment,
  toolchain: Web06PeerToolchainIdentity,
  host: Web06PeerHostIdentity,
): string[] {
  const actual = {
    version: expected.version,
    browserMode: "headed-foreground" as const,
    nodeVersion: toolchain.nodeVersion,
    playwrightVersion: toolchain.playwrightVersion,
    chromiumVersion: toolchain.chromiumVersion,
    chromiumExecutableSha256: toolchain.chromiumExecutableSha256,
    host,
    viewport: web06PeerViewport,
    locale: web06PeerLocale,
    cacheRegime: web06PeerCacheRegime,
  };
  const { environmentId: _ignored, ...expectedWithoutId } = expected;
  return canonicalJson(actual) === canonicalJson(expectedWithoutId)
    ? []
    : [`expected-environment:${expected.environmentId}`, `actual-environment:${sha256(canonicalJson(actual))}`];
}

function repositoryIdentity(repoRoot: string): { commit: string; tree: string; clean: boolean } {
  const git = (...args: string[]) => execFileSync("git", args, { cwd: repoRoot, encoding: "utf8" }).trim();
  return {
    commit: git("rev-parse", "HEAD"),
    tree: git("rev-parse", "HEAD^{tree}"),
    clean: git("status", "--porcelain=v1", "--untracked-files=all") === "",
  };
}

async function verifyPeerPacketChecksums(packetRoot: string): Promise<void> {
  const manifest = path.join(packetRoot, "peer-packet-files.sha256");
  await assertFileSha256(manifest, web06PeerPinnedPacketChecksumsSha256, "pinned peer packet checksum manifest");
  const text = await readFile(manifest, "utf8");
  const rows = text.split("\n").filter(Boolean);
  if (text !== `${rows.join("\n")}\n` || rows.length !== pinnedPeerPacketPaths.length) {
    throw new Error("Pinned peer packet checksum allowlist is noncanonical or changed");
  }
  const seen = new Set<string>();
  for (const [index, row] of rows.entries()) {
    const matched = /^([0-9a-f]{64})  (.+)$/.exec(row);
    if (!matched) throw new Error(`Malformed peer packet checksum row: ${row}`);
    const relative = matched[2] ?? "";
    if (relative !== pinnedPeerPacketPaths[index] || seen.has(relative)) {
      throw new Error("Pinned peer packet checksum path order/identity changed");
    }
    seen.add(relative);
    const file = await resolvePinnedPeerPacketFile(packetRoot, relative);
    await assertFileSha256(file, matched[1] ?? "", `peer packet ${relative}`);
  }
}

const pinnedPeerPacketPaths = [
  "../evaluate-peer-alignment.mjs",
  "../verify-peer-artifact.mjs",
  "../generate-comparator-projection.mjs",
  "../verify-comparator-projection.mjs",
  "../run-network-none-replay.mjs",
  "../freeze-original-peer-build-date.cjs",
  "../capture-completed-network-none-inspect.mjs",
  "../network-none-replay-raw-evidence.sha256",
  "../browser-smoke-228a24b9/comparator-projection-validation.json",
  "../browser-smoke-228a24b9/sealed-peer-browser-smoke.json",
  "../browser-smoke-228a24b9/public-evidence-privacy-validation.json",
  "public-evidence-files.sha256",
  "README.md",
  "web06-peer-data-v1.json",
  "web06-peer-data-v1-negative-essay-control.json",
  "web06-comparator-projection-phase0-product.json",
  "web06-comparator-projection-phase0-product-negative-essay.json",
  "comparator-projection-verification.json",
  "alignment-verdict.json",
  "negative-essay-control-verdict.json",
  "artifact-verification.json",
  "http-readiness.json",
  "network-none-replay-attempt-01-setup-failure.json",
  "network-none-replay-attempt-02-verification.json",
  "artifact-manifest.json",
  "../sealed-my-rime-bc67a225.tar",
] as const;

export async function resolvePinnedPeerPacketFile(packetRoot: string, relative: string): Promise<string> {
  if (!(pinnedPeerPacketPaths as readonly string[]).includes(relative)) {
    throw new Error(`Peer packet checksum path is not in the exact sealed allowlist: ${relative}`);
  }
  const root = path.resolve(packetRoot);
  const parentReal = await realpath(path.dirname(root));
  const file = path.resolve(root, relative);
  const info = await lstat(file);
  if (!info.isFile() || info.isSymbolicLink()) {
    throw new Error(`Peer packet checksum target is not a regular non-link file: ${relative}`);
  }
  const fileReal = await realpath(file);
  const relation = path.relative(parentReal, fileReal);
  if (relation === "" || relation.startsWith("..") || path.isAbsolute(relation)) {
    throw new Error(`Peer packet checksum escapes its exact sealed parent: ${relative}`);
  }
  return fileReal;
}

export function verifyArtifactSourceTree(
  repoRoot: string,
  artifactSourceCommit: string,
  claimedSourceTree: string,
): string {
  assertSha256("artifact source commit", artifactSourceCommit, 40);
  assertSha256("claimed artifact source tree", claimedSourceTree, 40);
  let derived: string;
  try {
    derived = execFileSync(
      "git",
      ["rev-parse", "--verify", `${artifactSourceCommit}^{tree}`],
      { cwd: repoRoot, encoding: "utf8" },
    ).trim();
  } catch {
    throw new Error(`Artifact build-info source commit is not present in the source repository: ${artifactSourceCommit}`);
  }
  if (derived !== claimedSourceTree) {
    throw new Error(`Claimed source tree is not derived from artifact build-info commit: ${claimedSourceTree} != ${derived}`);
  }
  return derived;
}

function safeArchivePath(raw: string): string {
  if (raw.includes("\0") || raw.includes("\\")) throw new Error(`Unsafe archive path: ${raw}`);
  const stripped = raw.replace(/^\.\//, "").replace(/\/$/, "");
  if (stripped === "") return "";
  if (path.posix.isAbsolute(stripped)) throw new Error(`Absolute archive path: ${raw}`);
  const normalized = path.posix.normalize(stripped);
  if (normalized === ".." || normalized.startsWith("../") || normalized !== stripped) {
    throw new Error(`Traversing or noncanonical archive path: ${raw}`);
  }
  return normalized;
}

async function assertSafeExtractedTree(root: string): Promise<void> {
  const rootReal = await realpath(root);
  const visit = async (dir: string): Promise<void> => {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const file = path.join(dir, entry.name);
      const info = await lstat(file);
      const resolved = await realpath(file);
      if (path.relative(rootReal, resolved).startsWith("..")) throw new Error(`Extracted path escapes root: ${file}`);
      if (info.isSymbolicLink() || (!info.isDirectory() && !info.isFile())) {
        throw new Error(`Extracted tree contains a forbidden entry: ${file}`);
      }
      if (info.isDirectory()) await visit(file);
    }
  };
  await visit(root);
}

async function regularFiles(root: string): Promise<string[]> {
  const result: string[] = [];
  const visit = async (dir: string): Promise<void> => {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const file = path.join(dir, entry.name);
      if (entry.isDirectory()) await visit(file);
      else if (entry.isFile()) result.push(path.relative(root, file).split(path.sep).join("/"));
      else throw new Error(`Nonregular artifact entry: ${file}`);
    }
  };
  await visit(root);
  return result.sort();
}

async function genericTreeSha256(root: string, files: string[]): Promise<string> {
  const rows: string[] = [];
  for (const relative of files) {
    const file = path.join(root, relative);
    const info = await stat(file);
    rows.push(`${relative}\0${info.size}\0${(info.mode & 0o777).toString(8)}\0${sha256(await readFile(file))}\n`);
  }
  return sha256(rows.sort().join(""));
}

async function findYuneRoot(extracted: string): Promise<string> {
  if (await isFile(path.join(extracted, "build-info.json"))) return extracted;
  const children = await readdir(extracted, { withFileTypes: true });
  const candidates = children.filter(entry => entry.isDirectory()).map(entry => path.join(extracted, entry.name));
  const matches = [];
  for (const candidate of candidates) if (await isFile(path.join(candidate, "build-info.json"))) matches.push(candidate);
  if (matches.length !== 1) throw new Error("Yune archive must contain exactly one public artifact root");
  return matches[0] ?? extracted;
}

async function findPeerRoot(extracted: string): Promise<string> {
  const root = path.join(extracted, "dist");
  if (!await isFile(path.join(root, "index.html"))) throw new Error("Pinned peer archive must contain dist/index.html");
  const children = await readdir(extracted);
  if (children.length !== 1 || children[0] !== "dist") throw new Error("Pinned peer archive contains unsealed top-level entries");
  return root;
}

async function isFile(file: string): Promise<boolean> {
  try { return (await stat(file)).isFile(); } catch { return false; }
}

async function assertFileSha256(file: string, expected: string, label: string): Promise<void> {
  const actual = createHash("sha256").update(await readFile(file)).digest("hex");
  if (actual !== expected) throw new Error(`${label} SHA-256 mismatch: ${actual} != ${expected}`);
}

function assertSha256(label: string, value: string, length = 64): void {
  if (!new RegExp(`^[0-9a-f]{${length}}$`).test(value)) throw new Error(`${label} is not a lowercase ${length}-hex digest`);
}

function findStringValues(value: unknown, key: string): string[] {
  if (Array.isArray(value)) return value.flatMap(item => findStringValues(item, key));
  if (value === null || typeof value !== "object") return [];
  return Object.entries(value as Record<string, unknown>).flatMap(([childKey, child]) => [
    ...(childKey === key && typeof child === "string" ? [child] : []),
    ...findStringValues(child, key),
  ]);
}
