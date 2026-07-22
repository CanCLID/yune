import { spawnSync } from "node:child_process";
import { mkdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  attestWeb06PeerArtifacts,
  capturePostRunIntegrity,
  resolveCreateNewExternalOutputRoot,
  verifyCreatedExternalOutputRoot,
  web06PeerSourceFiles,
} from "./startup-benchmark/web06-peer-artifacts.ts";
import { canonicalJson } from "./startup-benchmark/web06-peer-lane.ts";
import { verifyWeb06PeerEvidence } from "./verify-web06-peer-evidence.ts";

const e2eRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(e2eRoot, "../../..");
const options = parseArguments(process.argv.slice(2));
if (options.phase !== "phase4-peer") {
  throw new Error("WEB-06 peer launcher rejects BASELINE/FINAL; pass --phase phase4-peer");
}
if (options.mode !== "binding" && options.mode !== "setup-only") {
  throw new Error("Pass an explicit --mode binding or --mode setup-only");
}
if (options.provisional && options.mode !== "setup-only") {
  throw new Error("--allow-provisional-setup-source-mismatch is forbidden for binding runs");
}
const outputLocation = await resolveCreateNewExternalOutputRoot(
  repoRoot,
  required(options, "output-root"),
);
const outputRoot = outputLocation.outputRoot;
const parentInfo = await stat(path.dirname(outputRoot));
if (!parentInfo.isDirectory()) throw new Error("Output parent is not a directory");
await mkdir(outputRoot, { recursive: false, mode: 0o700 });
await verifyCreatedExternalOutputRoot(outputLocation.repoReal, outputRoot);
await mkdir(path.join(outputRoot, "raw"), { recursive: false, mode: 0o700 });
await mkdir(path.join(outputRoot, "compact"), { recursive: false, mode: 0o700 });

const sourceFiles = web06PeerSourceFiles(repoRoot);
const result = await attestWeb06PeerArtifacts({
  mode: options.mode,
  outputRoot,
  repoRoot,
  yuneArchive: path.resolve(required(options, "yune-archive")),
  yuneArchiveSha256: required(options, "yune-archive-sha256"),
  yuneSourceTree: required(options, "yune-source-tree"),
  yuneBuildInfoSha256: required(options, "yune-build-info-sha256"),
  yunePublicManifestSha256: required(options, "yune-public-manifest-sha256"),
  yuneBuildNpmVersion: required(options, "yune-build-npm-version"),
  peerArchive: path.resolve(required(options, "peer-archive")),
  peerPacketRoot: path.resolve(required(options, "peer-packet-root")),
  finalRunEnvironmentPath: path.resolve(required(options, "final-run-environment")),
  finalRunEnvironmentSha256: required(options, "final-run-environment-sha256"),
  sourceFiles,
  allowProvisionalSetupSourceMismatch: options.provisional,
});
const attestationPath = path.join(outputRoot, "raw", "attestation.json");
await writeFile(attestationPath, result.attestationText, { flag: "wx", mode: 0o600 });
await writeFile(
  path.join(outputRoot, "raw", "negative-essay-control.json"),
  result.negativeEssayControlText,
  { flag: "wx", mode: 0o600 },
);
const playwright = path.join(e2eRoot, "node_modules", ".bin", "playwright");
const playwrightResult = spawnSync(
  playwright,
  ["test", "--config", "playwright.web06-peer.config.ts"],
  {
    cwd: e2eRoot,
    env: {
      ...process.env,
      YUNE_WEB06_PEER_MODE: options.mode,
      YUNE_WEB06_PEER_ATTESTATION: attestationPath,
      YUNE_WEB06_PEER_OUTPUT_ROOT: outputRoot,
    },
    stdio: "inherit",
  },
);
if (playwrightResult.error) throw playwrightResult.error;
if (playwrightResult.status !== 0) {
  throw new Error(`WEB-06 peer Playwright lane failed with exit ${playwrightResult.status}`);
}
if (options.mode === "binding") {
  const postRunIntegrity = await capturePostRunIntegrity(result.attestation, result.attestationText);
  await writeFile(
    path.join(outputRoot, "raw", "post-run-integrity.json"),
    canonicalJson(postRunIntegrity),
    { flag: "wx", mode: 0o600 },
  );
  await verifyWeb06PeerEvidence(outputRoot, { writeIndependentVerification: true });
  await verifyWeb06PeerEvidence(outputRoot, { requireExistingIndependentVerification: true });
}
process.stdout.write(`${JSON.stringify({
  status: "PASS",
  mode: options.mode,
  phase: "phase4-peer",
  benchmarkAttempt: options.mode === "binding",
  outputRoot,
  liveEnvironmentDrift: result.liveEnvironmentDrift,
})}\n`);

interface ParsedArguments {
  mode: string;
  phase: string;
  provisional: boolean;
  values: Map<string, string>;
}

function parseArguments(args: string[]): ParsedArguments {
  const values = new Map<string, string>();
  let provisional = false;
  for (let index = 0; index < args.length; index += 1) {
    const item = args[index];
    if (item === "--allow-provisional-setup-source-mismatch") {
      provisional = true;
      continue;
    }
    if (!item?.startsWith("--")) throw new Error(`Unexpected peer-lane argument: ${item}`);
    const name = item.slice(2);
    const value = args[index + 1];
    if (!value || value.startsWith("--")) throw new Error(`Missing value for ${item}`);
    if (values.has(name)) throw new Error(`Duplicate peer-lane argument: ${item}`);
    values.set(name, value);
    index += 1;
  }
  const known = new Set([
    "mode", "phase", "output-root", "yune-archive", "yune-archive-sha256", "yune-source-tree",
    "yune-build-info-sha256", "yune-public-manifest-sha256", "yune-build-npm-version",
    "peer-archive", "peer-packet-root", "final-run-environment", "final-run-environment-sha256",
  ]);
  const unknown = [...values.keys()].filter(key => !known.has(key));
  if (unknown.length) throw new Error(`Unknown peer-lane arguments: ${unknown.join(", ")}`);
  return {
    mode: values.get("mode") ?? "",
    phase: values.get("phase") ?? "",
    provisional,
    values,
  };
}

function required(optionsValue: ParsedArguments, name: string): string {
  const value = optionsValue.values.get(name);
  if (!value) throw new Error(`Missing required --${name}`);
  return value;
}
