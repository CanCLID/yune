import { createHash } from "node:crypto";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import assert from "node:assert/strict";
import { validateLocalBundle } from "./public-artifact-verifier.mjs";

const sourceCommit = "0123456789abcdef0123456789abcdef01234567";
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

async function fixture() {
  const root = await mkdtemp(path.join(tmpdir(), "yune-public-artifact-"));
  await mkdir(path.join(root, "assets"));
  const files = new Map([
    ["assets/app.js", Buffer.from("app\n")],
    ["index.html", Buffer.from("<main>Yune</main>\n")],
    ["schema-asset-manifest.json", Buffer.from("{}\n")],
    ["worker.js", Buffer.from("self.onmessage=()=>{};\n")],
    ["yune-web.wasm", Buffer.from([0, 97, 115, 109])],
  ]);
  for (const [relative, bytes] of files) {
    await writeFile(path.join(root, ...relative.split("/")), bytes);
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
  await writeFile(path.join(root, "public-artifact-manifest.json"), manifestBytes);
  await writeFile(
    path.join(root, "build-info.json"),
    `${JSON.stringify({
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
    })}\n`,
  );
  return root;
}

test("full local artifact reconciliation passes and detects tampering", async () => {
  const root = await fixture();
  try {
    const result = await validateLocalBundle(root, sourceCommit);
    assert.equal(result.manifest.files.length, 5);
    await writeFile(path.join(root, "index.html"), "tampered\n");
    await assert.rejects(
      validateLocalBundle(root, sourceCommit),
      /differs from its inventory row/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
