import { describe, expect, it } from "vitest";

import {
  assertYuneWebAssetsReady,
  customizeAndSync,
  deployAndSync,
  isYuneWebLogicalId,
  mountYuneWebPersistence,
  prepareYuneWebFilesystem,
  requiredYuneWebAssetPaths,
  syncAfterUserDataChange,
  syncFromPersistenceBeforeInit,
  syncToPersistenceAfterMutation,
  YuneWebFilesystemError,
} from "../src/filesystem.js";
import { YuneWebRuntime } from "../src/runtime.js";
import { FakeYuneWebFilesystem } from "./fake-filesystem.js";
import { FakeYuneWebModule } from "./fake-module.js";

const defaultYaml = "config_version: yune-web\nschema_list:\n  - schema: yune_web_luna\n";
const schemaYaml = "schema:\n  schema_id: yune_web_luna\ntranslator:\n  dictionary: yune_web\n";
const dictionaryYaml = "---\nname: yune_web\nversion: '1'\n...\nba\t吧\t9\n";
const deployedDefaultYaml =
  "config_version: yune-web\nschema_list:\n  - schema: yune_web_luna\nmenu:\n  page_size: 50\n";
const deployedSchemaYaml =
  "schema:\n  schema_id: yune_web_luna\nmenu:\n  page_size: 50\ntranslator:\n  dictionary: yune_web\n";
const defaultInitPtr = 1;

function filesystemOptions(overrides: Partial<Parameters<typeof prepareYuneWebFilesystem>[1]> = {}) {
  return {
    sharedDataDir: "/yune/shared",
    userDataDir: "/yune/user",
    schemaId: "yune_web_luna",
    dictionaryId: "yune_web",
    assets: {
      defaultYaml,
      schemaYaml,
      dictionaryYaml,
    },
    ...overrides,
  };
}

function initializedRuntime(fake = new FakeYuneWebModule()): YuneWebRuntime {
  return YuneWebRuntime.init(fake, {
    sharedDataDir: "/yune/shared",
    userDataDir: "/yune/user",
    schemaId: "yune_web_luna",
  });
}

function filesystemWithRequiredAssetsExcept(missingPath: string): FakeYuneWebFilesystem {
  const fs = new FakeYuneWebFilesystem();
  fs.mkdirTree("/yune/shared");
  fs.mkdirTree("/yune/user/build");
  const assets = new Map<string, string>([
    ["/yune/shared/default.yaml", defaultYaml],
    ["/yune/shared/yune_web_luna.schema.yaml", schemaYaml],
    ["/yune/shared/yune_web.dict.yaml", dictionaryYaml],
    ["/yune/user/build/default.yaml", defaultYaml],
    ["/yune/user/build/yune_web_luna.schema.yaml", schemaYaml],
  ]);

  for (const [path, contents] of assets) {
    if (path !== missingPath) {
      fs.writeFile(path, contents);
    }
  }

  return fs;
}

function recordingFs(fs: FakeYuneWebFilesystem, order: string[]): FakeYuneWebFilesystem {
  return new Proxy(fs, {
    get(target, property, receiver) {
      if (property === "writeFile") {
        return (path: string, data: string | Uint8Array, opts?: { flags?: string }) => {
          order.push(`write:${path}`);
          target.writeFile(path, data, opts);
        };
      }
      if (property === "syncfs") {
        return (populate: boolean, callback: (error?: unknown) => void) => {
          order.push(`syncfs(${populate})`);
          target.syncfs(populate, callback);
        };
      }
      const value = Reflect.get(target, property, receiver);
      return typeof value === "function" ? value.bind(target) : value;
    },
  });
}

describe("YuneWeb browser filesystem preparation", () => {
  it("creates shared, user, and build directories in an Emscripten filesystem", () => {
    const fs = new FakeYuneWebFilesystem();

    prepareYuneWebFilesystem(fs, filesystemOptions());

    expect(fs.directories()).toEqual(["/", "/yune", "/yune/shared", "/yune/user", "/yune/user/build"]);
    expect(fs.calls("mkdirTree").map(([path]) => path)).toEqual([
      "/yune/shared",
      "/yune/user",
      "/yune/user/build",
    ]);
  });

  it("writes explicit shared and build assets at the native-required virtual paths", () => {
    const fs = new FakeYuneWebFilesystem();

    prepareYuneWebFilesystem(fs, filesystemOptions());

    expect(fs.readText("/yune/shared/default.yaml")).toBe(defaultYaml);
    expect(fs.readText("/yune/shared/yune_web_luna.schema.yaml")).toBe(schemaYaml);
    expect(fs.readText("/yune/shared/yune_web.dict.yaml")).toBe(dictionaryYaml);
    expect(fs.readText("/yune/user/build/default.yaml")).toBe(defaultYaml);
    expect(fs.readText("/yune/user/build/yune_web_luna.schema.yaml")).toBe(schemaYaml);
    expect(requiredYuneWebAssetPaths(filesystemOptions())).toEqual([
      "/yune/shared/default.yaml",
      "/yune/shared/yune_web_luna.schema.yaml",
      "/yune/shared/yune_web.dict.yaml",
      "/yune/user/build/default.yaml",
      "/yune/user/build/yune_web_luna.schema.yaml",
    ]);
  });

  it("keeps source assets shared while allowing deployed browser build assets", () => {
    const fs = new FakeYuneWebFilesystem();

    prepareYuneWebFilesystem(
      fs,
      filesystemOptions({
        assets: {
          defaultYaml,
          schemaYaml,
          dictionaryYaml,
          deployedDefaultYaml,
          deployedSchemaYaml,
        },
      }),
    );

    expect(fs.readText("/yune/shared/default.yaml")).toBe(defaultYaml);
    expect(fs.readText("/yune/shared/yune_web_luna.schema.yaml")).toBe(schemaYaml);
    expect(fs.readText("/yune/user/build/default.yaml")).toBe(deployedDefaultYaml);
    expect(fs.readText("/yune/user/build/yune_web_luna.schema.yaml")).toBe(deployedSchemaYaml);
  });

  it("creates absolute directories correctly when only mkdir is available", () => {
    const fs = new FakeYuneWebFilesystem();
    const mkdirOnlyFs = new Proxy(fs, {
      get(target, property, receiver) {
        if (property === "mkdirTree") {
          return undefined;
        }
        if (property === "mkdir") {
          return (path: string, mode?: number) => {
            if (path.startsWith("//")) {
              throw new Error(`unexpected doubled absolute path: ${path}`);
            }
            target.mkdir(path, mode);
          };
        }
        const value = Reflect.get(target, property, receiver);
        return typeof value === "function" ? value.bind(target) : value;
      },
    }) as FakeYuneWebFilesystem;

    prepareYuneWebFilesystem(mkdirOnlyFs, filesystemOptions());

    expect(fs.calls("mkdir").map(([path]) => path)).toEqual([
      "/yune",
      "/yune/shared",
      "/yune/user",
      "/yune/user/build",
    ]);
    expect(fs.readText("/yune/shared/default.yaml")).toBe(defaultYaml);
  });

  it("reports missing required assets without creating fallback files", () => {
    const fs = new FakeYuneWebFilesystem();
    fs.mkdirTree("/yune/shared");
    fs.mkdirTree("/yune/user/build");
    fs.writeFile("/yune/shared/yune_web_luna.schema.yaml", schemaYaml);

    expect(() => assertYuneWebAssetsReady(fs, filesystemOptions())).toThrow(YuneWebFilesystemError);
    expect(() => assertYuneWebAssetsReady(fs, filesystemOptions())).toThrow(
      "Missing YuneWeb filesystem assets: /yune/shared/default.yaml, /yune/shared/yune_web.dict.yaml, /yune/user/build/default.yaml, /yune/user/build/yune_web_luna.schema.yaml",
    );
    expect(fs.exists("/yune/shared/default.yaml")).toBe(false);
    expect(fs.exists("/yune/shared/yune_web.dict.yaml")).toBe(false);
    expect(fs.exists("/yune/user/build/default.yaml")).toBe(false);
    expect(fs.exists("/yune/user/build/yune_web_luna.schema.yaml")).toBe(false);
  });

  it("reports each missing preloaded asset with its virtual path", () => {
    const cases = [
      ["shared default", "/yune/shared/default.yaml"],
      ["shared schema", "/yune/shared/yune_web_luna.schema.yaml"],
      ["selected dictionary", "/yune/shared/yune_web.dict.yaml"],
      ["build default", "/yune/user/build/default.yaml"],
      ["build schema", "/yune/user/build/yune_web_luna.schema.yaml"],
    ] as const;

    for (const [label, missingPath] of cases) {
      const fs = filesystemWithRequiredAssetsExcept(missingPath);

      expect(() => assertYuneWebAssetsReady(fs, filesystemOptions()), label).toThrow(YuneWebFilesystemError);
      expect(() => assertYuneWebAssetsReady(fs, filesystemOptions()), label).toThrow(missingPath);
    }
  });

  it("treats a dictionary at the wrong shared path as a missing selected dictionary", () => {
    const fs = filesystemWithRequiredAssetsExcept("/yune/shared/yune_web.dict.yaml");
    fs.writeFile("/yune/shared/stray.dict.yaml", dictionaryYaml);

    expect(() => assertYuneWebAssetsReady(fs, filesystemOptions())).toThrow(
      "Missing YuneWeb filesystem assets: /yune/shared/yune_web.dict.yaml",
    );
    expect(fs.exists("/yune/shared/stray.dict.yaml")).toBe(true);
  });

  it("rejects invalid schema and dictionary ids before joining write paths", () => {
    const invalidIds = ["", "../yune_web_luna", "yune_web/luna", "yune_web\\luna"];

    for (const invalidId of invalidIds) {
      expect(isYuneWebLogicalId(invalidId)).toBe(false);

      const invalidSchemaFs = new FakeYuneWebFilesystem();
      expect(() =>
        prepareYuneWebFilesystem(invalidSchemaFs, filesystemOptions({ schemaId: invalidId })),
      ).toThrow(YuneWebFilesystemError);
      expect(invalidSchemaFs.calls("writeFile")).toEqual([]);

      const invalidDictionaryFs = new FakeYuneWebFilesystem();
      expect(() =>
        prepareYuneWebFilesystem(invalidDictionaryFs, filesystemOptions({ dictionaryId: invalidId })),
      ).toThrow(YuneWebFilesystemError);
      expect(invalidDictionaryFs.calls("writeFile")).toEqual([]);
    }

    expect(isYuneWebLogicalId("yune_web_luna-1")).toBe(true);
  });

  it("mounts a caller-provided persistence backend at the requested mountpoint", () => {
    const fs = new FakeYuneWebFilesystem();
    const type = { name: "IDBFS" };
    const opts = { root: "yune_web" };

    mountYuneWebPersistence(fs, type, opts, "/yune");

    expect(fs.directories()).toEqual(["/", "/yune"]);
    expect(fs.calls("mount")).toEqual([[type, opts, "/yune"]]);
  });

  it("wraps persistence mount backend failures with deterministic setup errors", () => {
    const fs = new FakeYuneWebFilesystem();
    fs.mountError = new Error("mount backend misconfigured");

    expect(() => mountYuneWebPersistence(fs, { name: "IDBFS" }, {}, "/yune")).toThrow(YuneWebFilesystemError);
    expect(() => mountYuneWebPersistence(fs, { name: "IDBFS" }, {}, "/yune")).toThrow(
      "YuneWeb persistence mount failed",
    );
    expect(fs.calls("mount")).toEqual([
      [{ name: "IDBFS" }, {}, "/yune"],
      [{ name: "IDBFS" }, {}, "/yune"],
    ]);
  });

  it("syncs from persistence before init using the populate direction", async () => {
    const fs = new FakeYuneWebFilesystem();

    await syncFromPersistenceBeforeInit(fs);

    expect(fs.calls("syncfs")).toEqual([[true]]);
  });

  it("syncs to persistence after mutation and explicit user data changes", async () => {
    const fs = new FakeYuneWebFilesystem();

    await syncToPersistenceAfterMutation(fs);
    await syncAfterUserDataChange(fs);

    expect(fs.calls("syncfs")).toEqual([[false], [false]]);
  });

  it("surfaces sync callback errors with deterministic direction details", async () => {
    const fs = new FakeYuneWebFilesystem();
    fs.syncError = new Error("fake sync failure");

    await expect(syncFromPersistenceBeforeInit(fs)).rejects.toMatchObject({
      name: "YuneWebFilesystemError",
      message: "YuneWeb filesystem sync failed",
      direction: "fromPersistence",
    });

    await expect(syncToPersistenceAfterMutation(fs)).rejects.toMatchObject({
      name: "YuneWebFilesystemError",
      message: "YuneWeb filesystem sync failed",
      direction: "toPersistence",
    });

    expect(fs.calls("syncfs")).toEqual([[true], [false]]);
  });

  it("wraps synchronous syncfs throws with deterministic direction details", async () => {
    const fs = new FakeYuneWebFilesystem();
    const throwingFs = new Proxy(fs, {
      get(target, property, receiver) {
        if (property === "syncfs") {
          return (populate: boolean) => {
            target.calls("syncfs").push([populate]);
            throw new Error("sync backend misconfigured");
          };
        }
        const value = Reflect.get(target, property, receiver);
        return typeof value === "function" ? value.bind(target) : value;
      },
    }) as FakeYuneWebFilesystem;

    await expect(syncFromPersistenceBeforeInit(throwingFs)).rejects.toMatchObject({
      name: "YuneWebFilesystemError",
      message: "YuneWeb filesystem sync failed",
      direction: "fromPersistence",
    });

    await expect(syncToPersistenceAfterMutation(throwingFs)).rejects.toMatchObject({
      name: "YuneWebFilesystemError",
      message: "YuneWeb filesystem sync failed",
      direction: "toPersistence",
    });

    expect(fs.calls("syncfs")).toEqual([[true], [false]]);
  });

  it("rejects a failed before-init sync before runtime init is attempted", async () => {
    const module = new FakeYuneWebModule();
    const fs = new FakeYuneWebFilesystem();
    fs.syncError = "persisted state unavailable";

    await expect(syncFromPersistenceBeforeInit(fs)).rejects.toMatchObject({
      name: "YuneWebFilesystemError",
      message: "YuneWeb filesystem sync failed",
      direction: "fromPersistence",
    });

    expect(fs.calls("syncfs")).toEqual([[true]]);
    expect(module.calls("yune_web_init")).toEqual([]);
  });

  it("deploys through the runtime before syncing to persistence and returns the deploy boolean", async () => {
    const module = new FakeYuneWebModule();
    module.deployResult = 1;
    const runtime = initializedRuntime(module);
    const fs = new FakeYuneWebFilesystem();

    await expect(deployAndSync(runtime, fs)).resolves.toBe(true);

    expect(module.calls("yune_web_deploy")).toEqual([[defaultInitPtr]]);
    expect(fs.calls("syncfs")).toEqual([[false]]);
  });

  it("throws sync failures after deploy while preserving the runtime mutation call", async () => {
    const module = new FakeYuneWebModule();
    module.deployResult = 0;
    const runtime = initializedRuntime(module);
    const fs = new FakeYuneWebFilesystem();
    fs.syncError = "persist failed";

    await expect(deployAndSync(runtime, fs)).rejects.toMatchObject({
      name: "YuneWebFilesystemError",
      message: "YuneWeb filesystem sync failed",
      direction: "toPersistence",
    });

    expect(module.calls("yune_web_deploy")).toEqual([[defaultInitPtr]]);
    expect(fs.calls("syncfs")).toEqual([[false]]);
  });

  it("customizes through the runtime before syncing and preserves adapter arguments", async () => {
    const module = new FakeYuneWebModule();
    module.customizeResult = 0;
    const runtime = initializedRuntime(module);
    const fs = new FakeYuneWebFilesystem();

    await expect(
      customizeAndSync(runtime, fs, "yune_web_luna.schema", "schema/name", "YuneWeb Luna Web"),
    ).resolves.toBe(false);

    expect(module.calls("yune_web_customize")).toEqual([
      [defaultInitPtr, "yune_web_luna.schema", "schema/name", "YuneWeb Luna Web"],
    ]);
    expect(fs.calls("syncfs")).toEqual([[false]]);
  });

  it("throws sync failures after customize while preserving the possible unpersisted runtime mutation", async () => {
    const module = new FakeYuneWebModule();
    module.customizeResult = 1;
    const runtime = initializedRuntime(module);
    const fs = new FakeYuneWebFilesystem();
    fs.syncError = "customization persisted state blocked";

    await expect(
      customizeAndSync(runtime, fs, "yune_web_luna.schema", "schema/name", "YuneWeb Luna Web"),
    ).rejects.toMatchObject({
      name: "YuneWebFilesystemError",
      message: "YuneWeb filesystem sync failed",
      direction: "toPersistence",
    });

    expect(module.calls("yune_web_customize")).toEqual([
      [defaultInitPtr, "yune_web_luna.schema", "schema/name", "YuneWeb Luna Web"],
    ]);
    expect(fs.calls("syncfs")).toEqual([[false]]);
  });

  it("keeps stale deployed config recovery in deterministic local-first order", async () => {
    const module = new FakeYuneWebModule();
    const fs = new FakeYuneWebFilesystem();
    const order: string[] = [];

    await syncFromPersistenceBeforeInit(recordingFs(fs, order));
    order.push("prepare");
    prepareYuneWebFilesystem(recordingFs(fs, order), filesystemOptions());
    order.push("verify");
    assertYuneWebAssetsReady(recordingFs(fs, order), filesystemOptions());
    order.push("init");
    const runtime = initializedRuntime(module);
    order.push("deploy");
    await deployAndSync(runtime, recordingFs(fs, order));
    runtime.cleanup();
    order.push("reinit");
    initializedRuntime(module);

    expect(order).toEqual([
      "syncfs(true)",
      "prepare",
      "write:/yune/shared/default.yaml",
      "write:/yune/shared/yune_web_luna.schema.yaml",
      "write:/yune/shared/yune_web.dict.yaml",
      "write:/yune/user/build/default.yaml",
      "write:/yune/user/build/yune_web_luna.schema.yaml",
      "verify",
      "init",
      "deploy",
      "syncfs(false)",
      "reinit",
    ]);
    expect(module.calls("yune_web_init")).toEqual([
      ["/yune/shared", "/yune/user", "yune_web_luna"],
      ["/yune/shared", "/yune/user", "yune_web_luna"],
    ]);
  });
});
