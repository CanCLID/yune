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

describe("public runtime value exports", () => {
  it("matches the frozen pre-WEB-06 package surface", () => {
    expect(Object.keys(publicRuntime).sort()).toEqual(BASELINE_VALUE_EXPORTS);
  });
});
