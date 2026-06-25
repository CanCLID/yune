import type { YuneWebRuntime } from "./runtime.js";

export type YuneWebFilesystemSyncDirection = "fromPersistence" | "toPersistence";

export interface YuneWebFilesystem {
  mkdirTree?(path: string, mode?: number): void;
  mkdir?(path: string, mode?: number): void;
  writeFile(path: string, data: string | Uint8Array, opts?: { flags?: string }): void;
  readFile(path: string, opts?: { encoding?: "utf8" | "binary" }): string | Uint8Array;
  analyzePath(path: string, dontResolveLastLink?: boolean): { exists: boolean; error?: unknown };
  mount?(type: unknown, opts: Record<string, unknown>, mountpoint: string): void;
  syncfs?(populate: boolean, callback: (error?: unknown) => void): void;
}

export interface YuneWebFilesystemAssets {
  defaultYaml: string | Uint8Array;
  schemaYaml: string | Uint8Array;
  dictionaryYaml: string | Uint8Array;
  deployedDefaultYaml?: string | Uint8Array;
  deployedSchemaYaml?: string | Uint8Array;
}

export interface PrepareYuneWebFilesystemOptions {
  sharedDataDir: string;
  userDataDir: string;
  schemaId: string;
  dictionaryId: string;
  assets: YuneWebFilesystemAssets;
}

export class YuneWebFilesystemError extends Error {
  readonly direction?: YuneWebFilesystemSyncDirection;

  constructor(message: string, options: { cause?: unknown; direction?: YuneWebFilesystemSyncDirection } = {}) {
    super(message, options.cause === undefined ? undefined : { cause: options.cause });
    this.name = "YuneWebFilesystemError";
    this.direction = options.direction;
  }
}

export function isYuneWebLogicalId(id: string): boolean {
  return /^[A-Za-z0-9_-]+$/.test(id);
}

export function joinYuneWebVirtualPath(base: string, ...parts: string[]): string {
  const allParts = [base, ...parts]
    .map((part) => part.split("/").filter((segment) => segment.length > 0))
    .flat();
  const joined = allParts.join("/");
  return base.startsWith("/") ? `/${joined}` : joined;
}

export function yuneWebBuildDir(userDataDir: string): string {
  return joinYuneWebVirtualPath(userDataDir, "build");
}

export function requiredYuneWebAssetPaths(options: PrepareYuneWebFilesystemOptions): string[] {
  assertYuneWebLogicalId(options.schemaId, "schemaId");
  assertYuneWebLogicalId(options.dictionaryId, "dictionaryId");
  const buildDir = yuneWebBuildDir(options.userDataDir);
  return [
    joinYuneWebVirtualPath(options.sharedDataDir, "default.yaml"),
    joinYuneWebVirtualPath(options.sharedDataDir, `${options.schemaId}.schema.yaml`),
    joinYuneWebVirtualPath(options.sharedDataDir, `${options.dictionaryId}.dict.yaml`),
    joinYuneWebVirtualPath(buildDir, "default.yaml"),
    joinYuneWebVirtualPath(buildDir, `${options.schemaId}.schema.yaml`),
  ];
}

export function prepareYuneWebFilesystem(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
): void {
  assertYuneWebLogicalId(options.schemaId, "schemaId");
  assertYuneWebLogicalId(options.dictionaryId, "dictionaryId");

  const buildDir = yuneWebBuildDir(options.userDataDir);
  ensureYuneWebDirectory(fs, options.sharedDataDir);
  ensureYuneWebDirectory(fs, options.userDataDir);
  ensureYuneWebDirectory(fs, buildDir);

  fs.writeFile(joinYuneWebVirtualPath(options.sharedDataDir, "default.yaml"), options.assets.defaultYaml, {
    flags: "w",
  });
  fs.writeFile(
    joinYuneWebVirtualPath(options.sharedDataDir, `${options.schemaId}.schema.yaml`),
    options.assets.schemaYaml,
    { flags: "w" },
  );
  fs.writeFile(
    joinYuneWebVirtualPath(options.sharedDataDir, `${options.dictionaryId}.dict.yaml`),
    options.assets.dictionaryYaml,
    { flags: "w" },
  );
  fs.writeFile(joinYuneWebVirtualPath(buildDir, "default.yaml"), options.assets.deployedDefaultYaml ?? options.assets.defaultYaml, {
    flags: "w",
  });
  fs.writeFile(
    joinYuneWebVirtualPath(buildDir, `${options.schemaId}.schema.yaml`),
    options.assets.deployedSchemaYaml ?? options.assets.schemaYaml,
    {
      flags: "w",
    },
  );

  assertYuneWebAssetsReady(fs, options);
}

export function assertYuneWebAssetsReady(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
): void {
  const missing = requiredYuneWebAssetPaths(options).filter((path) => !fs.analyzePath(path).exists);
  if (missing.length > 0) {
    throw new YuneWebFilesystemError(`Missing YuneWeb filesystem assets: ${missing.join(", ")}`);
  }
}

export async function syncYuneWebFilesystem(
  fs: YuneWebFilesystem,
  direction: YuneWebFilesystemSyncDirection,
): Promise<void> {
  if (fs.syncfs === undefined) {
    throw new YuneWebFilesystemError("Emscripten FS.syncfs is unavailable", { direction });
  }
  const populate = direction === "fromPersistence";
  await new Promise<void>((resolve, reject) => {
    try {
      fs.syncfs!(populate, (error?: unknown) => {
        if (error !== undefined && error !== null) {
          reject(new YuneWebFilesystemError("YuneWeb filesystem sync failed", { cause: error, direction }));
          return;
        }
        resolve();
      });
    } catch (error) {
      reject(new YuneWebFilesystemError("YuneWeb filesystem sync failed", { cause: error, direction }));
    }
  });
}

export async function syncFromPersistenceBeforeInit(fs: YuneWebFilesystem): Promise<void> {
  const marker = "syncFromPersistenceBeforeInit";
  performance?.mark?.(`${marker}:start`);
  await syncYuneWebFilesystem(fs, "fromPersistence");
  performance?.mark?.(`${marker}:end`);
  performance?.measure?.(marker, `${marker}:start`, `${marker}:end`);
  console.info(`${marker}: PASS`);
}

export async function syncToPersistenceAfterMutation(fs: YuneWebFilesystem): Promise<void> {
  const marker = "syncToPersistenceAfterMutation";
  performance?.mark?.(`${marker}:start`);
  await syncYuneWebFilesystem(fs, "toPersistence");
  performance?.mark?.(`${marker}:end`);
  performance?.measure?.(marker, `${marker}:start`, `${marker}:end`);
  console.info(`${marker}: PASS`);
}

export async function syncAfterUserDataChange(fs: YuneWebFilesystem): Promise<void> {
  await syncToPersistenceAfterMutation(fs);
}

export function mountYuneWebPersistence(
  fs: YuneWebFilesystem,
  type: unknown,
  opts: Record<string, unknown>,
  mountpoint: string,
): void {
  ensureYuneWebDirectory(fs, mountpoint);
  if (fs.mount === undefined) {
    throw new YuneWebFilesystemError("Emscripten FS.mount is unavailable");
  }
  try {
    fs.mount(type, opts, mountpoint);
  } catch (error) {
    throw new YuneWebFilesystemError("YuneWeb persistence mount failed", { cause: error });
  }
}

export async function deployAndSync(runtime: YuneWebRuntime, fs: YuneWebFilesystem): Promise<boolean> {
  const deployed = runtime.deploy();
  await syncToPersistenceAfterMutation(fs);
  return deployed;
}

export async function customizeAndSync(
  runtime: YuneWebRuntime,
  fs: YuneWebFilesystem,
  configId: string,
  key: string,
  value: string,
): Promise<boolean> {
  const customized = runtime.customize(configId, key, value);
  await syncToPersistenceAfterMutation(fs);
  return customized;
}

function ensureYuneWebDirectory(fs: YuneWebFilesystem, path: string): void {
  if (fs.analyzePath(path).exists) {
    return;
  }
  if (fs.mkdirTree !== undefined) {
    fs.mkdirTree(path);
    return;
  }
  if (fs.mkdir === undefined) {
    throw new YuneWebFilesystemError("Emscripten filesystem directory creation is unavailable");
  }
  const segments = path.split("/").filter((segment) => segment.length > 0);
  let current = path.startsWith("/") ? "/" : "";
  for (const segment of segments) {
    current = current === "/" || current === "" ? `${current}${segment}` : `${current}/${segment}`;
    if (!fs.analyzePath(current).exists) {
      fs.mkdir(current);
    }
  }
}

function assertYuneWebLogicalId(id: string, label: string): void {
  if (!isYuneWebLogicalId(id)) {
    throw new YuneWebFilesystemError(`Invalid YuneWeb logical id for ${label}: ${id}`);
  }
}
