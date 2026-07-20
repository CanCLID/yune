import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import {
  lstat,
  mkdir,
  mkdtemp,
  readFile,
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
  resolveGateContract,
  validateArchive,
  validateRemoteFile,
  validateRemoteBuildInfo,
  validateRemoteMetadata,
} from "./run-public-web06-gate.mjs";

const execFileAsync = promisify(execFile);

const sourceCommit = "0123456789abcdef0123456789abcdef01234567";
const archiveSha256 = "a".repeat(64);

async function releaseFixture(root) {
  const distRoot = path.join(root, "dist");
  await mkdir(path.join(distRoot, "assets"), { recursive: true });
  const files = new Map([
    ["assets/app.js", Buffer.from("app\n")],
    ["index.html", Buffer.from("<main>Yune</main>\n")],
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
      sha256: createHash("sha256").update(bytes).digest("hex"),
    })),
  };
  const manifestBytes = Buffer.from(`${JSON.stringify(manifest)}\n`);
  const buildInfo = {
    sourceCommit,
    sourceTreeState: "clean",
    wasmSha256: manifest.files.find((file) => file.path === "yune-web.wasm")
      .sha256,
    schemaManifestSha256: manifest.files.find(
      (file) => file.path === "schema-asset-manifest.json",
    ).sha256,
    publicArtifactManifestSha256: createHash("sha256")
      .update(manifestBytes)
      .digest("hex"),
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
  await writeFile(
    path.join(distRoot, "public-artifact-manifest.json"),
    manifestBytes,
  );
  return { distRoot, files, manifestBytes, buildInfoBytes };
}

function baseEnvironment() {
  return {
    YUNE_WEB_EXPECTED_SOURCE_COMMIT: sourceCommit,
    YUNE_WEB_CERTIFIED_ARCHIVE: "/tmp/yune-web06-dist.tar.gz",
    YUNE_WEB_CERTIFIED_ARCHIVE_SHA256: archiveSha256,
    YUNE_WEB_WEB06_EVIDENCE_DIR: "/tmp/yune-web06-evidence",
  };
}

test("full scope binds the archive and reserves a runner-owned extraction root", () => {
  const contract = resolveGateContract({
    ...baseEnvironment(),
    YUNE_WEB_WEB06_PREVIEW_PORT: "4317",
  });
  assert.equal(contract.scope, "full");
  assert.equal(contract.grep, "@web06-full");
  assert.equal(contract.appUrl, "http://127.0.0.1:4317/");
  assert.equal(contract.distRoot, null);
  assert.equal(contract.archiveSha256, archiveSha256);
  assert.equal(contract.archivePath, "/tmp/yune-web06-dist.tar.gz");
});

test("preview canary is the frozen existing-normal-guard/rapid-jyutping scope", () => {
  const contract = resolveGateContract(
    {
      ...baseEnvironment(),
      YUNE_WEB_APP_URL: "https://preview.example.invalid/candidate/",
    },
    ["--scope", "preview-canary"],
  );
  assert.equal(contract.scope, "preview-canary");
  assert.equal(contract.grep, "@web06-preview-canary");
  assert.equal(contract.distRoot, null);
  assert.equal(contract.appUrl, "https://preview.example.invalid/candidate/");
  assert.equal(
    contract.expectedPreviewScenarios,
    "existing-normal-guard,rapid-jyutping",
  );
  assert.throws(
    () =>
      resolveGateContract(
        {
          ...baseEnvironment(),
          YUNE_WEB_APP_URL: "http://preview.example.invalid/",
        },
        ["--scope", "preview-canary"],
      ),
    /must be an HTTPS URL/,
  );
});

test("preview reconciliation preflight has no measurement scope", () => {
  const contract = resolveGateContract(
    {
      ...baseEnvironment(),
      YUNE_WEB_APP_URL: "https://preview.example.invalid/",
    },
    ["--scope=preview-canary", "--verify-only"],
  );
  assert.equal(contract.verifyOnly, true);
  assert.equal(contract.statusName, "web06-preview-reconciliation-status.json");
  assert.throws(
    () =>
      resolveGateContract(
        {
          ...baseEnvironment(),
        },
        ["--verify-only"],
      ),
    /supported only for preview-canary/,
  );
});

test("scope contracts remain disjoint and fail closed", () => {
  assert.deepEqual(Object.keys(gateScopes), ["full", "preview-canary"]);
  assert.throws(
    () =>
      resolveGateContract({
        ...baseEnvironment(),
        YUNE_WEB_WEB06_DIST_ROOT: "/tmp/untrusted-dist",
      }),
    /runner-owned.*forbidden/,
  );
  assert.throws(
    () =>
      resolveGateContract(
        {
          ...baseEnvironment(),
          YUNE_WEB_WEB06_GATE_SCOPE: "informational",
        },
        [],
      ),
    /Unsupported YUNE_WEB_WEB06_GATE_SCOPE/,
  );
  assert.throws(
    () =>
      resolveGateContract(
        {
          ...baseEnvironment(),
          YUNE_WEB_APP_URL: "https://preview.example.invalid/",
          YUNE_WEB_WEB06_GATE_SCOPE: "full",
        },
        ["--scope=preview-canary"],
      ),
    /scopes disagree/,
  );
});

test("source and archive identity must be complete lowercase hashes", () => {
  assert.throws(
    () =>
      resolveGateContract({
        ...baseEnvironment(),
        YUNE_WEB_EXPECTED_SOURCE_COMMIT: sourceCommit.toUpperCase(),
      }),
    /40-character SHA/,
  );
  assert.throws(
    () =>
      resolveGateContract({
        ...baseEnvironment(),
        YUNE_WEB_CERTIFIED_ARCHIVE_SHA256: "a".repeat(63),
      }),
    /64-character SHA/,
  );
});

test("archive bytes and sibling digest must share the frozen identity", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "yune-web06-runner-"));
  try {
    const archivePath = path.join(root, "yune-web-dist.tar.gz");
    const bytes = Buffer.from("sealed archive bytes\n");
    const digest = createHash("sha256").update(bytes).digest("hex");
    await writeFile(archivePath, bytes);
    await writeFile(`${archivePath}.sha256`, `${digest}\n`);
    await validateArchive({ archivePath, archiveSha256: digest });
    await writeFile(archivePath, "tampered\n");
    await assert.rejects(
      validateArchive({ archivePath, archiveSha256: digest }),
      /do not match their frozen SHA-256 identity/,
    );
    await writeFile(archivePath, bytes);
    const digestTarget = path.join(root, "digest.txt");
    await writeFile(digestTarget, `${digest}\n`);
    await rm(`${archivePath}.sha256`);
    await symlink(digestTarget, `${archivePath}.sha256`);
    await assert.rejects(
      validateArchive({ archivePath, archiveSha256: digest }),
      /sibling digest must be a plain file/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("Playwright output must be a strict lexical descendant of evidence", () => {
  assert.throws(
    () =>
      resolveGateContract({
        ...baseEnvironment(),
        YUNE_WEB_WEB06_OUTPUT_DIR: "/tmp/unrelated-output",
      }),
    /strict descendant of the external evidence root/,
  );
  assert.throws(
    () =>
      resolveGateContract({
        ...baseEnvironment(),
        YUNE_WEB_WEB06_OUTPUT_DIR: "/tmp/yune-web06-evidence",
      }),
    /strict descendant of the external evidence root/,
  );
});

test("Playwright output rejects a symlink component before any cleanup", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "yune-web06-output-"));
  try {
    const evidenceDir = path.join(root, "evidence");
    const outside = path.join(root, "outside");
    await mkdir(evidenceDir);
    await mkdir(outside);
    await symlink(outside, path.join(evidenceDir, "playwright"), "dir");
    const contract = resolveGateContract({
      ...baseEnvironment(),
      YUNE_WEB_WEB06_EVIDENCE_DIR: evidenceDir,
    });
    await assert.rejects(
      prepareOutputPaths(contract),
      /Playwright output traverses a symbolic link/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("evidence root rejects a caller-supplied symlink before canonicalization", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "yune-web06-evidence-link-"));
  try {
    const outside = path.join(root, "outside");
    const evidenceLink = path.join(root, "evidence-link");
    await mkdir(outside);
    await symlink(outside, evidenceLink, "dir");
    const contract = resolveGateContract({
      ...baseEnvironment(),
      YUNE_WEB_WEB06_EVIDENCE_DIR: evidenceLink,
    });
    await assert.rejects(
      prepareOutputPaths(contract),
      /evidence root traverses a symbolic link/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("preview identity requires the exact clean source and manifest hash", () => {
  const buildInfo = {
    sourceCommit,
    sourceTreeState: "clean",
    publicArtifactManifestSha256: "b".repeat(64),
  };
  assert.equal(validateRemoteBuildInfo(buildInfo, sourceCommit), buildInfo);
  assert.throws(
    () => validateRemoteBuildInfo({ ...buildInfo, sourceCommit: "f".repeat(40) }, sourceCommit),
    /expected clean source commit/,
  );
  assert.throws(
    () => validateRemoteBuildInfo({ ...buildInfo, publicArtifactManifestSha256: "missing" }, sourceCommit),
    /manifest hash/,
  );
});

test("preview metadata and every served file must match sealed bytes", () => {
  const buildInfo = {
    sourceCommit,
    sourceTreeState: "clean",
    publicArtifactManifestSha256: "placeholder",
  };
  const manifestBytes = Buffer.from('{"files":[]}\n');
  buildInfo.publicArtifactManifestSha256 = createHash("sha256")
    .update(manifestBytes)
    .digest("hex");
  const buildInfoBytes = Buffer.from(`${JSON.stringify(buildInfo)}\n`);
  const local = { buildInfoBytes, manifestBytes };
  assert.deepEqual(
    validateRemoteMetadata(
      buildInfoBytes,
      manifestBytes,
      local,
      sourceCommit,
    ),
    buildInfo,
  );
  assert.throws(
    () =>
      validateRemoteMetadata(
        Buffer.from(`${JSON.stringify({ ...buildInfo, sourceCommit: "f".repeat(40) })}\n`),
        manifestBytes,
        local,
        sourceCommit,
      ),
    /expected clean source commit/,
  );

  const bytes = Buffer.from("served artifact\n");
  const file = {
    path: "assets/app.js",
    bytes: bytes.byteLength,
    sha256: createHash("sha256")
      .update(bytes)
      .digest("hex"),
  };
  assert.doesNotThrow(() => validateRemoteFile(file, bytes));
  assert.throws(
    () => validateRemoteFile(file, Buffer.from("tampered\n")),
    /differs from the sealed inventory/,
  );
});

test("preview reconciliation preflight verifies the archive, local root, and every remote file", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "yune-web06-preflight-"));
  const originalFetch = globalThis.fetch;
  try {
    const fixture = await releaseFixture(root);
    const archivePath = path.join(root, "yune-web-dist.tar.gz");
    await execFileAsync(
      "tar",
      ["-C", fixture.distRoot, "-czf", archivePath, "."],
      { env: { ...process.env, COPYFILE_DISABLE: "1" } },
    );
    const archiveBytes = await readFile(archivePath);
    const digest = createHash("sha256").update(archiveBytes).digest("hex");
    await writeFile(`${archivePath}.sha256`, `${digest}\n`);
    const served = new Map([
      ["build-info.json", fixture.buildInfoBytes],
      ["public-artifact-manifest.json", fixture.manifestBytes],
      ...fixture.files,
    ]);
    globalThis.fetch = async (value) => {
      const url = new URL(value);
      const relative = decodeURIComponent(url.pathname.replace(/^\//, ""));
      const bytes = served.get(relative);
      return bytes === undefined
        ? new Response("missing\n", { status: 404 })
        : new Response(bytes, { status: 200 });
    };
    const evidenceDir = path.join(root, "evidence");
    await main(
      {
        ...process.env,
        YUNE_WEB_APP_URL: "https://preview.example.invalid/",
        YUNE_WEB_EXPECTED_SOURCE_COMMIT: sourceCommit,
        YUNE_WEB_CERTIFIED_ARCHIVE: archivePath,
        YUNE_WEB_CERTIFIED_ARCHIVE_SHA256: digest,
        YUNE_WEB_WEB06_EVIDENCE_DIR: evidenceDir,
      },
      ["--scope=preview-canary", "--verify-only"],
    );
    const status = JSON.parse(
      await readFile(
        path.join(evidenceDir, "web06-preview-reconciliation-status.json"),
        "utf8",
      ),
    );
    assert.equal(status.status, "reconciled");
    assert.equal(status.measurementStarted, false);
    assert.equal(status.archiveSha256, digest);
    assert.equal(status.artifactFileCount, 5);
    assert.equal(status.extractedArtifactRetained, false);
    await assert.rejects(
      lstat(
        path.join(
          evidenceDir,
          ".web06-sealed-artifact-preview-reconciliation",
        ),
      ),
      { code: "ENOENT" },
    );
    const statusPath = path.join(
      evidenceDir,
      "web06-preview-reconciliation-status.json",
    );
    const preservedStatus = await readFile(statusPath);
    await assert.rejects(
      main(
        {
          ...process.env,
          YUNE_WEB_APP_URL: "https://preview.example.invalid/",
          YUNE_WEB_EXPECTED_SOURCE_COMMIT: sourceCommit,
          YUNE_WEB_CERTIFIED_ARCHIVE: archivePath,
          YUNE_WEB_CERTIFIED_ARCHIVE_SHA256: digest,
          YUNE_WEB_WEB06_EVIDENCE_DIR: evidenceDir,
        },
        ["--scope=preview-canary", "--verify-only"],
      ),
      { code: "EEXIST" },
    );
    assert.deepEqual(await readFile(statusPath), preservedStatus);
  } finally {
    globalThis.fetch = originalFetch;
    await rm(root, { recursive: true, force: true });
  }
});

test("an unrelated archive cannot borrow a separately supplied good dist", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "yune-web06-fake-archive-"));
  try {
    const unrelated = path.join(root, "unrelated");
    await mkdir(unrelated);
    await writeFile(path.join(unrelated, "not-yune.txt"), "unrelated\n");
    const archivePath = path.join(root, "unrelated.tar.gz");
    await execFileAsync(
      "tar",
      ["-C", unrelated, "-czf", archivePath, "."],
      { env: { ...process.env, COPYFILE_DISABLE: "1" } },
    );
    const archiveBytes = await readFile(archivePath);
    const digest = createHash("sha256").update(archiveBytes).digest("hex");
    await writeFile(`${archivePath}.sha256`, `${digest}\n`);
    const evidenceDir = path.join(root, "evidence");

    await assert.rejects(
      main(
        {
          ...process.env,
          YUNE_WEB_APP_URL: "https://preview.example.invalid/",
          YUNE_WEB_EXPECTED_SOURCE_COMMIT: sourceCommit,
          YUNE_WEB_CERTIFIED_ARCHIVE: archivePath,
          YUNE_WEB_CERTIFIED_ARCHIVE_SHA256: digest,
          YUNE_WEB_WEB06_EVIDENCE_DIR: evidenceDir,
        },
        ["--scope=preview-canary", "--verify-only"],
      ),
      /build-info\.json|ENOENT/,
    );
    const status = JSON.parse(
      await readFile(
        path.join(evidenceDir, "web06-preview-reconciliation-status.json"),
        "utf8",
      ),
    );
    assert.equal(status.status, "failed");
    assert.equal(status.extractedArtifactRetained, true);
    assert.match(
      status.retainedExtractedArtifactRoot,
      /\.web06-sealed-artifact-preview-reconciliation$/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
