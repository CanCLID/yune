import { createServer } from "node:http";
import { constants as fileConstants } from "node:fs";
import { lstat, open, realpath } from "node:fs/promises";
import path from "node:path";

import { manifestName, sha256 } from "./public-artifact-verifier.mjs";

function checkedRelativePath(relative) {
  if (
    typeof relative !== "string" ||
    relative === "" ||
    relative.includes("\\") ||
    relative.startsWith("/") ||
    relative.split("/").some((part) => part === "" || part === "." || part === "..")
  ) {
    throw new Error(`Invalid sealed artifact path: ${JSON.stringify(relative)}`);
  }
  return relative;
}

function sameFileIdentity(left, right) {
  return (
    left.dev === right.dev &&
    left.ino === right.ino &&
    left.size === right.size &&
    left.mtimeMs === right.mtimeMs &&
    left.ctimeMs === right.ctimeMs
  );
}

function assertStrictDescendant(root, member) {
  const relative = path.relative(root, member);
  if (
    relative === "" ||
    relative === ".." ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    throw new Error(`Sealed artifact member escaped its root: ${member}`);
  }
}

async function assertPlainMemberPath(root, relative) {
  let probe = root;
  const rootMetadata = await lstat(root);
  if (!rootMetadata.isDirectory() || rootMetadata.isSymbolicLink()) {
    throw new Error("Sealed artifact root must be a plain directory");
  }
  const components = relative.split("/");
  for (let index = 0; index < components.length; index += 1) {
    probe = path.join(probe, components[index]);
    const metadata = await lstat(probe);
    if (metadata.isSymbolicLink()) {
      throw new Error(`Sealed artifact member traverses a symbolic link: ${relative}`);
    }
    if (index < components.length - 1 && !metadata.isDirectory()) {
      throw new Error(`Sealed artifact parent is not a directory: ${relative}`);
    }
    if (index === components.length - 1 && !metadata.isFile()) {
      throw new Error(`Sealed artifact member is not a plain file: ${relative}`);
    }
  }
}

async function readFrozenMember(root, expected) {
  const relative = checkedRelativePath(expected.path);
  const absolute = path.join(root, ...relative.split("/"));
  assertStrictDescendant(root, absolute);
  await assertPlainMemberPath(root, relative);
  const canonicalBefore = await realpath(absolute);
  if (path.normalize(canonicalBefore) !== path.normalize(absolute)) {
    throw new Error(`Sealed artifact member changed canonical path: ${relative}`);
  }
  const pathBefore = await lstat(absolute);
  const noFollow = fileConstants.O_NOFOLLOW ?? 0;
  const handle = await open(absolute, fileConstants.O_RDONLY | noFollow);
  let bytes;
  let handleBefore;
  let handleAfter;
  try {
    handleBefore = await handle.stat();
    bytes = await handle.readFile();
    handleAfter = await handle.stat();
  } finally {
    await handle.close();
  }
  const [pathAfter, canonicalAfter] = await Promise.all([
    lstat(absolute),
    realpath(absolute),
  ]);
  if (
    !pathBefore.isFile() ||
    pathBefore.isSymbolicLink() ||
    !handleBefore.isFile() ||
    !handleAfter.isFile() ||
    !pathAfter.isFile() ||
    pathAfter.isSymbolicLink() ||
    !sameFileIdentity(pathBefore, handleBefore) ||
    !sameFileIdentity(handleBefore, handleAfter) ||
    !sameFileIdentity(handleAfter, pathAfter) ||
    path.normalize(canonicalAfter) !== path.normalize(absolute)
  ) {
    throw new Error(`Sealed artifact member changed while snapshotted: ${relative}`);
  }
  if (
    !Number.isSafeInteger(expected.bytes) ||
    expected.bytes < 0 ||
    !/^[0-9a-f]{64}$/.test(expected.sha256 ?? "") ||
    bytes.byteLength !== expected.bytes ||
    sha256(bytes) !== expected.sha256
  ) {
    throw new Error(`Sealed artifact member differs from its inventory: ${relative}`);
  }
  return {
    path: relative,
    bytes: Buffer.from(bytes),
    expectedBytes: expected.bytes,
    expectedSha256: expected.sha256,
  };
}

function snapshotRows(local) {
  if (
    !Buffer.isBuffer(local?.buildInfoBytes) ||
    !Buffer.isBuffer(local?.manifestBytes) ||
    !Array.isArray(local?.manifest?.files)
  ) {
    throw new Error("A validated local artifact bundle is required for serving");
  }
  return [
    {
      path: "build-info.json",
      bytes: local.buildInfoBytes.byteLength,
      sha256: sha256(local.buildInfoBytes),
    },
    {
      path: manifestName,
      bytes: local.manifestBytes.byteLength,
      sha256: sha256(local.manifestBytes),
    },
    ...local.manifest.files.map((entry) => ({
      path: entry?.path,
      bytes: entry?.bytes,
      sha256: entry?.sha256,
    })),
  ];
}

export function sealedArtifactResponseGuard(local) {
  const entries = snapshotRows(local);
  if (new Set(entries.map((entry) => entry.path)).size !== entries.length) {
    throw new Error("WEB06 artifact response guard contains duplicate paths");
  }
  return {
    version: "web06-artifact-response-guard-v1",
    rootDocumentPath: "index.html",
    entries,
  };
}

export async function loadSealedArtifactSnapshot(distRootValue, local) {
  const requestedRoot = path.resolve(distRootValue);
  const canonicalRoot = await realpath(requestedRoot);
  if (path.normalize(canonicalRoot) !== path.normalize(requestedRoot)) {
    throw new Error("Sealed artifact root changed canonical path");
  }
  const rows = snapshotRows(local);
  const entries = new Map();
  for (const row of rows) {
    const relative = checkedRelativePath(row?.path);
    if (entries.has(relative)) {
      throw new Error(`Duplicate sealed artifact path: ${relative}`);
    }
    entries.set(relative, await readFrozenMember(canonicalRoot, row));
  }
  if (!entries.has("index.html")) {
    throw new Error("Sealed artifact snapshot is missing index.html");
  }
  return {
    version: "web06-sealed-artifact-snapshot-v1",
    root: canonicalRoot,
    entries,
  };
}

function contentType(relative) {
  const extension = path.extname(relative).toLowerCase();
  return ({
    ".avif": "image/avif",
    ".bin": "application/octet-stream",
    ".css": "text/css; charset=utf-8",
    ".gif": "image/gif",
    ".html": "text/html; charset=utf-8",
    ".ico": "image/x-icon",
    ".jpeg": "image/jpeg",
    ".jpg": "image/jpeg",
    ".js": "text/javascript; charset=utf-8",
    ".json": "application/json; charset=utf-8",
    ".map": "application/json; charset=utf-8",
    ".mjs": "text/javascript; charset=utf-8",
    ".png": "image/png",
    ".svg": "image/svg+xml",
    ".txt": "text/plain; charset=utf-8",
    ".wasm": "application/wasm",
    ".webp": "image/webp",
    ".woff": "font/woff",
    ".woff2": "font/woff2",
    ".yaml": "application/yaml; charset=utf-8",
    ".yml": "application/yaml; charset=utf-8",
  })[extension] ?? "application/octet-stream";
}

function requestedArtifactPath(requestUrl) {
  const parsed = new URL(requestUrl ?? "/", "http://127.0.0.1/");
  if (parsed.pathname === "/") return "index.html";
  const encodedComponents = parsed.pathname.slice(1).split("/");
  const components = encodedComponents.map((component) => {
    let decoded;
    try {
      decoded = decodeURIComponent(component);
    } catch {
      throw new Error("Artifact request path has invalid percent encoding");
    }
    if (
      decoded === "" ||
      decoded === "." ||
      decoded === ".." ||
      decoded.includes("/") ||
      decoded.includes("\\") ||
      decoded.includes("\0")
    ) {
      throw new Error("Artifact request path is unsafe");
    }
    return decoded;
  });
  return components.join("/");
}

export function createSealedArtifactResponder(snapshot) {
  if (
    snapshot?.version !== "web06-sealed-artifact-snapshot-v1" ||
    !(snapshot.entries instanceof Map)
  ) {
    throw new Error("A sealed artifact snapshot is required");
  }
  let stickyFailure = null;
  const failSticky = (error) => {
    if (stickyFailure === null) {
      stickyFailure = error instanceof Error ? error : new Error(String(error));
    }
    return stickyFailure;
  };
  return {
    handle(request, response) {
      if (stickyFailure !== null) {
        response.writeHead(503, { "Cache-Control": "no-store" });
        response.end("sealed artifact integrity failure\n");
        return;
      }
      if (request.method !== "GET" && request.method !== "HEAD") {
        response.writeHead(405, {
          Allow: "GET, HEAD",
          "Cache-Control": "no-store",
        });
        response.end();
        return;
      }
      let relative;
      try {
        relative = requestedArtifactPath(request.url);
      } catch {
        response.writeHead(400, { "Cache-Control": "no-store" });
        response.end("invalid artifact path\n");
        return;
      }
      const entry = snapshot.entries.get(relative);
      if (entry === undefined) {
        response.writeHead(404, { "Cache-Control": "no-store" });
        response.end("not found\n");
        return;
      }
      try {
        const responseBytes = Buffer.from(entry.bytes);
        if (
          responseBytes.byteLength !== entry.expectedBytes ||
          sha256(responseBytes) !== entry.expectedSha256
        ) {
          throw new Error(`Sealed artifact response changed after snapshot: ${relative}`);
        }
        response.writeHead(200, {
          "Cache-Control": "no-store",
          "Content-Length": String(responseBytes.byteLength),
          "Content-Type": contentType(relative),
          "X-Content-Type-Options": "nosniff",
        });
        response.end(request.method === "HEAD" ? undefined : responseBytes);
      } catch (error) {
        failSticky(error);
        if (!response.headersSent) {
          response.writeHead(500, { "Cache-Control": "no-store" });
        }
        response.end("sealed artifact integrity failure\n");
      }
    },
    failSticky,
    assertHealthy() {
      if (stickyFailure !== null) throw stickyFailure;
    },
  };
}

export async function startSealedArtifactServer(
  snapshot,
  { host = "127.0.0.1", port } = {},
) {
  if (!Number.isInteger(port) || port < 0 || port > 65535) {
    throw new Error(`Invalid sealed artifact server port: ${port}`);
  }
  const responder = createSealedArtifactResponder(snapshot);
  const server = createServer((request, response) => {
    responder.handle(request, response);
  });
  server.on("error", (error) => responder.failSticky(error));
  await new Promise((resolve, reject) => {
    const onError = (error) => {
      server.off("listening", onListening);
      reject(error);
    };
    const onListening = () => {
      server.off("error", onError);
      resolve();
    };
    server.once("error", onError);
    server.once("listening", onListening);
    server.listen(port, host);
  });
  const address = server.address();
  if (!address || typeof address === "string") {
    await new Promise((resolve) => server.close(resolve));
    throw new Error("Sealed artifact server did not bind a TCP address");
  }
  let stopped = false;
  return {
    appUrl: `http://${host}:${address.port}/`,
    assertHealthy: () => responder.assertHealthy(),
    async stop() {
      if (stopped) return;
      stopped = true;
      await new Promise((resolve, reject) => {
        server.close((error) => (error ? reject(error) : resolve()));
      });
    },
  };
}
