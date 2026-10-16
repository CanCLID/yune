import { sha256 } from "./public-artifact-verifier.mjs";

// Cloudflare Pages reads these root files as deployment configuration and never
// serves them; a request for one falls through to index.html. Their bytes are
// still sealed and checked in the local archive.
const PAGES_CONFIGURATION_FILES = new Set(["_headers", "_redirects", "_routes.json"]);

export function productionManifestRows(manifest) {
  if (!Array.isArray(manifest?.files) || manifest.files.length === 0) {
    throw new Error("Production verification requires the complete certified manifest");
  }
  return manifest.files.filter((file) => !PAGES_CONFIGURATION_FILES.has(file?.path));
}

export function verifyProductionManifestMember(file, bytes) {
  if (
    typeof file?.path !== "string" ||
    !Number.isSafeInteger(file?.bytes) ||
    !/^[0-9a-f]{64}$/.test(file?.sha256 ?? "") ||
    bytes.byteLength !== file.bytes ||
    sha256(bytes) !== file.sha256
  ) {
    throw new Error(
      `Production artifact ${file?.path ?? "<invalid>"} differs from the certified manifest`,
    );
  }
}
