import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
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

import {
  createSealedArtifactResponder,
  loadSealedArtifactSnapshot,
  startSealedArtifactServer,
} from "./web06-sealed-artifact-server.mjs";

const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");

async function artifactFixture(root) {
  const distRoot = path.join(root, "dist");
  await mkdir(path.join(distRoot, "assets"), { recursive: true });
  const files = new Map([
    ["assets/app.js", Buffer.from("console.log('sealed');\n")],
    ["index.html", Buffer.from("<script src=\"/assets/app.js\"></script>\n")],
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
      sha256: digest(bytes),
    })),
  };
  const manifestBytes = Buffer.from(`${JSON.stringify(manifest)}\n`);
  const buildInfoBytes = Buffer.from('{"sourceCommit":"fixture"}\n');
  await writeFile(path.join(distRoot, "build-info.json"), buildInfoBytes);
  await writeFile(
    path.join(distRoot, "public-artifact-manifest.json"),
    manifestBytes,
  );
  return {
    distRoot: await realpath(distRoot),
    files,
    local: { buildInfoBytes, manifestBytes, manifest },
  };
}

test("sealed server never reopens artifact paths after its final snapshot", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-sealed-server-"));
  let server = null;
  try {
    const fixture = await artifactFixture(root);
    const snapshot = await loadSealedArtifactSnapshot(
      fixture.distRoot,
      fixture.local,
    );
    server = await startSealedArtifactServer(snapshot, { port: 0 });
    const first = await fetch(new URL("assets/app.js", server.appUrl));
    assert.equal(first.status, 200);
    assert.deepEqual(
      Buffer.from(await first.arrayBuffer()),
      fixture.files.get("assets/app.js"),
    );

    const outside = path.join(root, "outside.js");
    await writeFile(outside, "console.log('unsealed');\n");
    await rm(path.join(fixture.distRoot, "assets", "app.js"));
    await symlink(outside, path.join(fixture.distRoot, "assets", "app.js"));

    const afterSwap = await fetch(new URL("assets/app.js", server.appUrl));
    assert.equal(afterSwap.status, 200);
    assert.deepEqual(
      Buffer.from(await afterSwap.arrayBuffer()),
      fixture.files.get("assets/app.js"),
    );
    server.assertHealthy();
  } finally {
    if (server !== null) await server.stop();
    await rm(root, { recursive: true, force: true });
  }
});

test("snapshot rejects a symlink even when its target has the expected bytes", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-sealed-symlink-"));
  try {
    const fixture = await artifactFixture(root);
    const member = path.join(fixture.distRoot, "assets", "app.js");
    const outside = path.join(root, "matching.js");
    await writeFile(outside, fixture.files.get("assets/app.js"));
    await rm(member);
    await symlink(outside, member);
    await assert.rejects(
      loadSealedArtifactSnapshot(fixture.distRoot, fixture.local),
      /symbolic link/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("a response hash failure is sticky even if snapshot bytes are restored", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-sealed-sticky-"));
  let server = null;
  try {
    const fixture = await artifactFixture(root);
    const snapshot = await loadSealedArtifactSnapshot(
      fixture.distRoot,
      fixture.local,
    );
    const entry = snapshot.entries.get("assets/app.js");
    const original = Buffer.from(entry.bytes);
    server = await startSealedArtifactServer(snapshot, { port: 0 });
    entry.bytes[0] ^= 0xff;
    const failed = await fetch(new URL("assets/app.js", server.appUrl));
    assert.equal(failed.status, 500);
    assert.throws(
      () => server.assertHealthy(),
      /response changed after snapshot/,
    );

    entry.bytes = original;
    const stillFailed = await fetch(new URL("assets/app.js", server.appUrl));
    assert.equal(stillFailed.status, 503);
    assert.throws(
      () => server.assertHealthy(),
      /response changed after snapshot/,
    );
  } finally {
    if (server !== null) await server.stop();
    await rm(root, { recursive: true, force: true });
  }
});

test("the responder hashes metadata and manifest members on every response", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "web06-sealed-metadata-"));
  try {
    const fixture = await artifactFixture(root);
    const snapshot = await loadSealedArtifactSnapshot(
      fixture.distRoot,
      fixture.local,
    );
    const responder = createSealedArtifactResponder(snapshot);
    const entry = snapshot.entries.get("public-artifact-manifest.json");
    entry.bytes = Buffer.from("{}\n");
    const response = {
      headersSent: false,
      status: null,
      writeHead(status) {
        this.status = status;
        this.headersSent = true;
      },
      end() {},
    };
    responder.handle(
      { method: "GET", url: "/public-artifact-manifest.json" },
      response,
    );
    assert.equal(response.status, 500);
    assert.throws(() => responder.assertHealthy(), /changed after snapshot/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
