import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const testDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(testDir, "../../..");
const patchPath = resolve(repoRoot, "apps/yune-web/patches/yune-web-runtime.patch");

describe("yune-web maintained patch", () => {
  it("keeps added App.tsx lines as clean UTF-8 text", () => {
    const patchBytes = readFileSync(patchPath);
    const patchText = patchBytes.toString("utf8");
    const addedText = patchText
      .split(/\r?\n/)
      .filter((line) => line.startsWith("+") && !line.startsWith("+++"))
      .join("\n");

    expect(patchBytes.includes(Buffer.from([0xef, 0xbb, 0xbf]))).toBe(false);
    expect(addedText).not.toMatch(/\uFEFF|ï»¿|Â©|å•Ÿ|å¥—|ç¶²|é |é‡|æ–°|æ•´|ç†/);
    expect(addedText).toContain("套用設定");
    expect(addedText).toContain("yune-web");
    expect(addedText).not.toContain("YuneWeb 網頁版");
  });
});
