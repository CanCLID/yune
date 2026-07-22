import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import * as publicRuntime from "../src/index.js";

const BASELINE_VALUE_EXPORTS = [
  "RIME_KEY",
  "RIME_MASK",
  "YUNE_WEB_EXPORTS",
  "YuneWebBindingError",
  "YuneWebFilesystemError",
  "YuneWebKeyError",
  "YuneWebLifecycleError",
  "YuneWebResponseError",
  "YuneWebRuntime",
  "assertYuneWebAssetsReady",
  "bindYuneWebModule",
  "customizeAndSync",
  "deployAndSync",
  "isYuneWebLogicalId",
  "joinYuneWebVirtualPath",
  "keyEventToRimeKey",
  "mountYuneWebPersistence",
  "prepareYuneWebFilesystem",
  "readYuneWebResponse",
  "requiredYuneWebAssetPaths",
  "syncAfterUserDataChange",
  "syncFromPersistenceBeforeInit",
  "syncToPersistenceAfterMutation",
  "syncYuneWebFilesystem",
  "yuneWebBuildDir",
] as const;

const BASELINE_DECLARATION_SHA256 = {
  "index.d.ts": "5ad752d4ed121754068efb778105261b4a98c62eaf62957d9292551e5664d306",
  "response.d.ts": "cbc54178ac87b3c107b8f5bff12d2955cd9c9a1b5464789676a1c8437a7df43e",
  "runtime.d.ts": "89ca5f0eae2ee039384eefb19c583a6d60abaaebd506f7f853abf2742f15ad2b",
} as const;

function sha256(bytes: Buffer): string {
  return createHash("sha256").update(bytes).digest("hex");
}

describe("public runtime value exports", () => {
  it("matches the frozen pre-WEB-06 package surface", () => {
    expect(Object.keys(publicRuntime).sort()).toEqual(BASELINE_VALUE_EXPORTS);
  });

  it("preserves the frozen JavaScript call arities", () => {
    expect(publicRuntime.readYuneWebResponse).toHaveLength(2);
    expect(publicRuntime.YuneWebRuntime.init).toHaveLength(2);
    expect(publicRuntime.YuneWebRuntime.prototype.processKey).toHaveLength(1);
    expect(publicRuntime.YuneWebRuntime.prototype.selectCandidate).toHaveLength(1);
    expect(publicRuntime.YuneWebRuntime.prototype.deleteCandidate).toHaveLength(1);
    expect(publicRuntime.YuneWebRuntime.prototype.flipPage).toHaveLength(0);
    expect(publicRuntime.YuneWebRuntime.prototype.deploy).toHaveLength(0);
    expect(publicRuntime.YuneWebRuntime.prototype.customize).toHaveLength(3);
    expect(publicRuntime.YuneWebRuntime.prototype.setOption).toHaveLength(2);
    expect(publicRuntime.YuneWebRuntime.prototype.setAiEnabled).toHaveLength(1);
    expect(publicRuntime.YuneWebRuntime.prototype.stageAi).toHaveLength(0);
    expect(publicRuntime.YuneWebRuntime.prototype.cleanup).toHaveLength(0);
  });

  it("byte-matches the frozen emitted declarations", () => {
    const packageRoot = fileURLToPath(new URL("../", import.meta.url));
    const outputRoot = mkdtempSync(path.join(tmpdir(), "yune-web06-runtime-api-"));
    try {
      execFileSync(
        process.execPath,
        [
          path.join(packageRoot, "node_modules", "typescript", "bin", "tsc"),
          "-p",
          path.join(packageRoot, "tsconfig.json"),
          "--outDir",
          outputRoot,
        ],
        { stdio: "pipe" },
      );
      const actual = Object.fromEntries(
        Object.keys(BASELINE_DECLARATION_SHA256).map((file) => [
          file,
          sha256(readFileSync(path.join(outputRoot, file))),
        ]),
      );
      expect(actual).toEqual(BASELINE_DECLARATION_SHA256);
    } finally {
      rmSync(outputRoot, { force: true, recursive: true });
    }
  });
});
