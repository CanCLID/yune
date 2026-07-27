import { sha256 } from "./public-artifact-verifier.mjs";

export function productionManifestRows(manifest) {
  if (!Array.isArray(manifest?.files) || manifest.files.length === 0) {
    throw new Error("Production verification requires the complete certified manifest");
  }
  return [...manifest.files];
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
