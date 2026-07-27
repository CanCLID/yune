import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmod,
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  realpath,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { promisify } from "node:util";

import {
  gateScopes,
  main,
  prepareOutputPaths,
  proveExactPublicGateRunnerSource,
  resolveGateContract,
  validateArchive,
  validateRemoteBuildInfo,
  validateRemoteFile,
  validateRemoteMetadata,
  withoutCloudflareCredentials,
} from "./run-public-web06-gate.mjs";
import {
  WEB06_COLLECTOR_CONTRACT_SHA256,
} from "./web06-collector.mjs";
import {
  createBaselineSetupNoGoBundle,
  writeCanonicalJson,
} from "./web06-artifact-set-test-fixture.mjs";
import {
  WEB06_BEHAVIOR_VERSION,
  WEB06_METRIC_VERSION,
  WEB06_SCENARIO_VERSION,
} from "./web06-suite-attestation.mjs";

const execFileAsync = promisify(execFile);
const sourceCommit = "0123456789abcdef0123456789abcdef01234567";
const sourceTree = "76543210fedcba9876543210fedcba9876543210";
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

function identityManifest(
  archiveSha256,
  artifactManifestSha256,
  buildInfoSha256,
) {
  return {
    version: "web06-target-identities-v1",
    metricContractVersion: WEB06_METRIC_VERSION,
    scenarioRegistryVersion: WEB06_SCENARIO_VERSION,
    behaviorPredicateVersion: WEB06_BEHAVIOR_VERSION,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    roles: {
      FINAL: {
        sourceCommit,
        sourceTree,
        sourceTreeState: "clean",
        archiveSha256,
        artifactManifestSha256,
        buildInfoSha256,
        selectedBranch: "A",
        disposition: "PRODUCTION_REDUCTION",
      },
    },
  };
}

async function releaseFixture(root) {
  const canonicalRoot = await realpath(root);
  const distRoot = path.join(root, "dist");
  await mkdir(path.join(distRoot, "assets"), { recursive: true });
  await mkdir(path.join(distRoot, "schema"), { recursive: true });
  const files = new Map([
    ["assets/app.js", Buffer.from("app\n")],
    ["index.html", Buffer.from("<main>Yune</main>\n")],
    ["schema/rare.bin", Buffer.from("previously-unverified-member\n")],
    ["schema-asset-manifest.json", Buffer.from("{}\n")],
    ["worker.js", Buffer.from("self.onmessage=()=>{};\n")],
    ["yune-web.wasm", Buffer.from([0, 97, 115, 109])],
  ]);
  for (const [relative, bytes] of files) {
    await writeFile(path.join(distRoot, ...relative.split("/")), bytes);
  }
  const manifest = {
    generatedFor: "yune-web",
    version: "web03-public-artifact-v1",
    files: [...files].map(([relative, bytes]) => ({
      path: relative,
      bytes: bytes.byteLength,
      sha256: sha256(bytes),
    })),
  };
  const manifestBytes = Buffer.from(`${JSON.stringify(manifest)}\n`);
  const buildInfo = {
    sourceCommit,
    sourceTreeState: "clean",
    wasmSha256: manifest.files.find((file) => file.path === "yune-web.wasm").sha256,
    schemaManifestSha256: manifest.files.find(
      (file) => file.path === "schema-asset-manifest.json",
    ).sha256,
    publicArtifactManifestSha256: sha256(manifestBytes),
    toolchain: {
      emsdkVersion: "4.0.23",
      emscriptenReleaseCommit: "aaa43392544d695232b70eda706d751f18980c2a",
      emsdkRepositoryCommit: "db04e88298d9916fc51fcd3743045ca3eb695127",
      emccVersion:
        "emcc (Emscripten gcc/clang-like replacement + linker emulating GNU ld) 4.0.23 (7a5d93b50f6a3a35e85a0d2fc9e667b8498e6aed)",
      rustcVersion: "rustc 1.96.1 (31fca3adb 2026-06-26)",
      nodeVersion: "v22.16.0",
    },
  };
  const buildInfoBytes = Buffer.from(`${JSON.stringify(buildInfo)}\n`);
  await writeFile(path.join(distRoot, "build-info.json"), buildInfoBytes);
  await writeFile(path.join(distRoot, "public-artifact-manifest.json"), manifestBytes);
  const archivePath = path.join(root, "yune-web-dist.tar.gz");
  await execFileAsync("tar", ["-C", distRoot, "-czf", archivePath, "."], {
    env: { ...process.env, COPYFILE_DISABLE: "1" },
  });
  const archiveSha256 = sha256(await readFile(archivePath));
  await writeFile(`${archivePath}.sha256`, `${archiveSha256}\n`);
  const identity = identityManifest(
    archiveSha256,
    sha256(manifestBytes),
    sha256(buildInfoBytes),
  );
  const finalSuiteEvidenceRoot = path.join(canonicalRoot, "final-collector");
  await mkdir(finalSuiteEvidenceRoot);
  const finalSuitePath = path.join(
    finalSuiteEvidenceRoot,
    "suite-attestation.json",
  );
  await writeFile(finalSuitePath, '{"fixture":"final-suite"}\n');
  return {
    archivePath,
    archiveSha256,
    buildInfoBytes,
    files,
    finalSuitePath,
    finalSuiteEvidenceRoot,
    identity,
    manifestBytes,
    finalSuiteAttestationSha256: sha256(await readFile(finalSuitePath)),
  };
}

const fixtureSuiteReader = (fixture) => async (file, expected) => {
  assert.equal(file, fixture.finalSuitePath);
  assert.equal(expected.expectation, "FINAL");
  assert.equal(expected.disposition, "PRODUCTION_REDUCTION");
  return {
    sha256: fixture.finalSuiteAttestationSha256,
  };
};

function environment(fixture, evidenceRoot) {
  return {
    YUNE_WEB06_EXPECTED_SOURCE_COMMIT: sourceCommit,
    YUNE_WEB06_EXPECTED_SOURCE_TREE: sourceTree,
    YUNE_WEB06_CERTIFIED_ARCHIVE: fixture.archivePath,
    YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256: fixture.archiveSha256,
    YUNE_WEB06_EVIDENCE_ROOT: evidenceRoot,
    YUNE_WEB06_SELECTED_BRANCH: "A",
    YUNE_WEB06_DISPOSITION: "PRODUCTION_REDUCTION",
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: JSON.stringify(fixture.identity),
    YUNE_WEB06_FINAL_SUITE_ATTESTATION: fixture.finalSuitePath,
    YUNE_WEB06_FINAL_SUITE_EVIDENCE_ROOT: fixture.finalSuiteEvidenceRoot,
    YUNE_WEB06_RUN_ENVIRONMENT_JSON: "{}",
  };
}

test("release and preview scopes expose only the frozen focused canary", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-scope-"));
  try {
    const fixture = await releaseFixture(root);
    const local = resolveGateContract(environment(fixture, path.join(root, "local")));
    assert.deepEqual(Object.keys(gateScopes), ["release-certification", "preview-canary"]);
    assert.equal(local.scope, "release-certification");
    assert.equal(local.grep, "@web06-preview-canary");
    assert.equal(local.appUrl, "http://127.0.0.1:4174/");
    const remote = resolveGateContract(
      {
        ...environment(fixture, path.join(root, "remote")),
        YUNE_WEB_APP_URL: "https://preview.example.invalid/candidate/",
      },
      ["--scope=preview-canary"],
    );
    assert.deepEqual(remote.expectedPreviewScenarios, [
      "existing-normal-guard",
      "rapid-jyutping",
    ]);
    const legacyIgnored = resolveGateContract({
      ...environment(fixture, path.join(root, "legacy-ignored")),
      YUNE_WEB_WEB06_GATE_SCOPE: "full",
    });
    assert.equal(legacyIgnored.scope, "release-certification");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("public gate child environments cannot inherit deployment credentials", () => {
  assert.deepEqual(
    withoutCloudflareCredentials({
      PATH: "/usr/bin",
      YUNE_WEB06_RUN_ID: "safe",
      CLOUDFLARE_ACCOUNT_ID: "account",
      CLOUDFLARE_API_TOKEN: "token",
      CF_API_KEY: "key",
      WRANGLER_AUTH_DOMAIN: "secret.example",
    }),
    {
      PATH: "/usr/bin",
      YUNE_WEB06_RUN_ID: "safe",
    },
  );
});

test("public gate runner source is clean, exact, and credential-free", async () => {
  const contract = {
    expectedSourceCommit: sourceCommit,
    expectedSourceTree: sourceTree,
  };
  const execute = ({ head = sourceCommit, tree = sourceTree, status = "" } = {}) =>
    async (command, arguments_, options) => {
      assert.equal(command, "git");
      assert.equal(options.stdio, "pipe");
      assert.equal(
        Object.keys(options.env).some(
          (name) =>
            name.startsWith("CLOUDFLARE_") ||
            name.startsWith("CF_") ||
            name.startsWith("WRANGLER_"),
        ),
        false,
      );
      const key = arguments_.join(" ");
      if (key === "rev-parse HEAD") return { stdout: `${head}\n`, stderr: "" };
      if (key === "rev-parse HEAD^{tree}") return { stdout: `${tree}\n`, stderr: "" };
      if (key === "status --porcelain --untracked-files=all") {
        return { stdout: status, stderr: "" };
      }
      throw new Error(`unexpected git command: ${key}`);
    };
  assert.deepEqual(
    await proveExactPublicGateRunnerSource(
      contract,
      { CLOUDFLARE_API_TOKEN: "must-not-leak" },
      execute(),
    ),
    { sourceCommit, sourceTree },
  );
  await assert.rejects(
    proveExactPublicGateRunnerSource(contract, {}, execute({ status: " M runner.mjs\n" })),
    /clean exact source HEAD\/tree/,
  );
  await assert.rejects(
    proveExactPublicGateRunnerSource(contract, {}, execute({ head: "f".repeat(40) })),
    /clean exact source HEAD\/tree/,
  );
  await assert.rejects(
    proveExactPublicGateRunnerSource(contract, {}, execute({ tree: "e".repeat(40) })),
    /clean exact source HEAD\/tree/,
  );
});

test("archive bytes and sibling digest share one frozen identity", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-archive-"));
  try {
    const fixture = await releaseFixture(root);
    await validateArchive({ archivePath: fixture.archivePath, archiveSha256: fixture.archiveSha256 });
    await writeFile(fixture.archivePath, "tampered\n");
    await assert.rejects(
      validateArchive({ archivePath: fixture.archivePath, archiveSha256: fixture.archiveSha256 }),
      /do not match/,
    );
    const digestTarget = path.join(root, "digest.txt");
    await writeFile(digestTarget, `${fixture.archiveSha256}\n`);
    await rm(`${fixture.archivePath}.sha256`);
    await symlink(digestTarget, `${fixture.archivePath}.sha256`);
    await assert.rejects(
      validateArchive({ archivePath: fixture.archivePath, archiveSha256: fixture.archiveSha256 }),
      /sibling digest must be a plain file/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("output roots reject traversal and symbolic links", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-output-"));
  try {
    const fixture = await releaseFixture(root);
    assert.throws(
      () => resolveGateContract({
        ...environment(fixture, path.join(root, "evidence")),
        YUNE_WEB06_PLAYWRIGHT_OUTPUT_DIR: path.join(root, "outside"),
      }),
      /strict descendant/,
    );
    const evidence = path.join(root, "linked-evidence");
    const outside = path.join(root, "outside");
    await mkdir(outside);
    await symlink(outside, evidence, "dir");
    const contract = resolveGateContract(environment(fixture, evidence));
    await assert.rejects(prepareOutputPaths(contract), /evidence root traverses a symbolic link/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("output reservation is exclusive, private, owner-bound, and race-safe", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-output-reservation-"));
  const originalGetuid = process.getuid;
  try {
    const fixture = await releaseFixture(root);

    const reserved = resolveGateContract(
      environment(fixture, path.join(root, "reserved")),
    );
    await prepareOutputPaths(reserved);
    assert.equal((await lstat(reserved.evidenceDir)).mode & 0o777, 0o700);
    assert.equal((await lstat(reserved.outputDir)).mode & 0o777, 0o700);

    const preexisting = resolveGateContract(
      environment(fixture, path.join(root, "preexisting")),
    );
    await mkdir(preexisting.evidenceDir, { mode: 0o700 });
    await assert.rejects(
      prepareOutputPaths(preexisting),
      /evidence root must be create-new/,
    );

    const raced = resolveGateContract(
      environment(fixture, path.join(root, "raced")),
    );
    await assert.rejects(
      prepareOutputPaths(raced, process.env, {
        beforeEvidenceMkdir: () => mkdir(raced.evidenceDir, { mode: 0o700 }),
      }),
      /evidence root must be create-new/,
    );

    const permissiveParent = await mkdtemp(
      path.join(tmpdir(), "web06-output-permissive-"),
    );
    await chmod(permissiveParent, 0o770);
    const permissive = resolveGateContract(
      environment(fixture, path.join(permissiveParent, "evidence")),
    );
    await assert.rejects(
      prepareOutputPaths(permissive),
      /trusted evidence parent must have mode 0700/,
    );
    await rm(permissiveParent, { recursive: true, force: true });

    const foreign = resolveGateContract(
      environment(fixture, path.join(root, "foreign-owner")),
    );
    process.getuid = () => originalGetuid.call(process) + 1;
    await assert.rejects(
      prepareOutputPaths(foreign),
      /trusted evidence parent must be owned by the current uid/,
    );
  } finally {
    process.getuid = originalGetuid;
    await rm(root, { recursive: true, force: true });
  }
});

test("remote metadata and individual manifest members fail on byte drift", () => {
  const manifestBytes = Buffer.from('{"files":[]}\n');
  const buildInfo = {
    sourceCommit,
    sourceTreeState: "clean",
    publicArtifactManifestSha256: sha256(manifestBytes),
  };
  const buildInfoBytes = Buffer.from(`${JSON.stringify(buildInfo)}\n`);
  assert.equal(validateRemoteBuildInfo(buildInfo, sourceCommit), buildInfo);
  assert.deepEqual(
    validateRemoteMetadata(
      buildInfoBytes,
      manifestBytes,
      { buildInfoBytes, manifestBytes },
      sourceCommit,
    ),
    buildInfo,
  );
  const bytes = Buffer.from("member\n");
  const row = { path: "schema/rare.bin", bytes: bytes.length, sha256: sha256(bytes) };
  assert.doesNotThrow(() => validateRemoteFile(row, bytes));
  assert.throws(() => validateRemoteFile(row, Buffer.from("drift\n")), /differs/);
});

test("preview reconciliation verifies every manifest member and writes an immutable receipt", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-reconcile-"));
  const originalFetch = globalThis.fetch;
  try {
    const fixture = await releaseFixture(root);
    const served = new Map([
      ["build-info.json", fixture.buildInfoBytes],
      ["public-artifact-manifest.json", fixture.manifestBytes],
      ...fixture.files,
    ]);
    globalThis.fetch = async (value) => {
      const relative = decodeURIComponent(new URL(value).pathname.replace(/^\//, ""));
      const bytes = served.get(relative);
      return bytes === undefined ? new Response("missing\n", { status: 404 }) : new Response(bytes);
    };
    const evidenceRoot = path.join(root, "evidence");
    const env = {
      ...environment(fixture, evidenceRoot),
      YUNE_WEB_APP_URL: "https://preview.example.invalid/",
    };
    await main(
      env,
      ["--scope=preview-canary", "--verify-only"],
      { readAndValidateSuiteAttestation: fixtureSuiteReader(fixture) },
    );
    const statusPath = path.join(evidenceRoot, "web06-preview-reconciliation-status.json");
    const original = await readFile(statusPath);
    const status = JSON.parse(original);
    assert.equal(status.status, "reconciled");
    assert.equal(status.artifactFileCount, 6);
    assert.equal(status.canaryStarted, false);
    await assert.rejects(
      main(
        env,
        ["--scope=preview-canary", "--verify-only"],
        { readAndValidateSuiteAttestation: fixtureSuiteReader(fixture) },
      ),
      { code: "EEXIST" },
    );
    assert.deepEqual(await readFile(statusPath), original);
  } finally {
    globalThis.fetch = originalFetch;
    await rm(root, { recursive: true, force: true });
  }
});

test("tampering a formerly unverified manifest member fails preview reconciliation", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-remote-tamper-"));
  const originalFetch = globalThis.fetch;
  try {
    const fixture = await releaseFixture(root);
    const served = new Map([
      ["build-info.json", fixture.buildInfoBytes],
      ["public-artifact-manifest.json", fixture.manifestBytes],
      ...fixture.files,
    ]);
    served.set("schema/rare.bin", Buffer.from("tampered remote member\n"));
    globalThis.fetch = async (value) => {
      const relative = decodeURIComponent(new URL(value).pathname.replace(/^\//, ""));
      return new Response(served.get(relative) ?? "missing\n", {
        status: served.has(relative) ? 200 : 404,
      });
    };
    const evidenceRoot = path.join(root, "evidence");
    await assert.rejects(
      main(
        {
          ...environment(fixture, evidenceRoot),
          YUNE_WEB_APP_URL: "https://preview.example.invalid/",
        },
        ["--scope=preview-canary", "--verify-only"],
        { readAndValidateSuiteAttestation: fixtureSuiteReader(fixture) },
      ),
      /schema\/rare\.bin differs/,
    );
    const status = JSON.parse(
      await readFile(path.join(evidenceRoot, "web06-preview-reconciliation-status.json"), "utf8"),
    );
    assert.equal(status.status, "failed");
  } finally {
    globalThis.fetch = originalFetch;
    await rm(root, { recursive: true, force: true });
  }
});

test("artifact-set mutation is rejected before remote fetch, server, or canary", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-real-suite-mutation-"));
  const bundle = await createBaselineSetupNoGoBundle();
  const originalFetch = globalThis.fetch;
  try {
    const fixture = await releaseFixture(root);
    const attestation = JSON.parse(
      await readFile(bundle.paths.attestation, "utf8"),
    );
    attestation.collectorOutput.bytes += 1;
    await writeCanonicalJson(bundle.paths.attestation, attestation);
    let fetchCount = 0;
    globalThis.fetch = async () => {
      fetchCount += 1;
      throw new Error("network must not begin");
    };
    await assert.rejects(
      main(
        {
          ...environment(fixture, path.join(root, "evidence")),
          YUNE_WEB_APP_URL: "https://preview.example.invalid/",
        },
        ["--scope=preview-canary", "--verify-only"],
        {
          readAndValidateSuiteAttestation: () => bundle.verify(),
        },
      ),
      /collector output byte count changed/,
    );
    assert.equal(fetchCount, 0);
    const status = JSON.parse(
      await readFile(
        path.join(
          root,
          "evidence",
          "web06-preview-reconciliation-status.json",
        ),
        "utf8",
      ),
    );
    assert.equal(status.canaryStarted, false);
  } finally {
    globalThis.fetch = originalFetch;
    await bundle.cleanup();
    await rm(root, { recursive: true, force: true });
  }
});

test("a missing FINAL suite attestation fails before any canary", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-missing-suite-"));
  const originalFetch = globalThis.fetch;
  try {
    const fixture = await releaseFixture(root);
    await rm(fixture.finalSuitePath);
    const served = new Map([
      ["build-info.json", fixture.buildInfoBytes],
      ["public-artifact-manifest.json", fixture.manifestBytes],
      ...fixture.files,
    ]);
    globalThis.fetch = async (value) => {
      const relative = decodeURIComponent(new URL(value).pathname.replace(/^\//, ""));
      return new Response(served.get(relative) ?? "missing\n", {
        status: served.has(relative) ? 200 : 404,
      });
    };
    const evidenceRoot = path.join(root, "evidence");
    await assert.rejects(
      main(
        {
          ...environment(fixture, evidenceRoot),
          YUNE_WEB_APP_URL: "https://preview.example.invalid/",
        },
        ["--scope=preview-canary", "--verify-only"],
      ),
      /ENOENT/,
    );
    const status = JSON.parse(
      await readFile(path.join(evidenceRoot, "web06-preview-reconciliation-status.json"), "utf8"),
    );
    assert.equal(status.canaryStarted, false);
  } finally {
    globalThis.fetch = originalFetch;
    await rm(root, { recursive: true, force: true });
  }
});

test("archive traversal is rejected before extraction or canary", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-traversal-"));
  try {
    const fixture = await releaseFixture(root);
    const malicious = path.join(root, "malicious.tar.gz");
    await execFileAsync("python3", ["-c", [
      "import io,tarfile,sys",
      "with tarfile.open(sys.argv[1], 'w:gz') as t:",
      " i=tarfile.TarInfo('../escape'); b=b'escape'; i.size=len(b); t.addfile(i,io.BytesIO(b))",
    ].join("\n"), malicious]);
    fixture.archivePath = malicious;
    fixture.archiveSha256 = sha256(await readFile(malicious));
    await writeFile(`${malicious}.sha256`, `${fixture.archiveSha256}\n`);
    const evidenceRoot = path.join(root, "evidence");
    await assert.rejects(
      main(
        {
          ...environment(fixture, evidenceRoot),
          YUNE_WEB_APP_URL: "https://preview.example.invalid/",
        },
        ["--scope=preview-canary", "--verify-only"],
      ),
      /unsafe archive member/,
    );
    await assert.rejects(lstat(path.join(root, "escape")), { code: "ENOENT" });
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
