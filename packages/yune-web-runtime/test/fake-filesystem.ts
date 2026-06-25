import type { YuneWebFilesystem } from "../src/filesystem";

type CallMap = Record<string, unknown[][]>;

export class FakeYuneWebFilesystem implements YuneWebFilesystem {
  #directories = new Set<string>(["/"]);
  #files = new Map<string, string | Uint8Array>();
  #calls: CallMap = {
    analyzePath: [],
    mkdir: [],
    mkdirTree: [],
    mount: [],
    readFile: [],
    syncfs: [],
    writeFile: [],
  };

  mountError?: unknown;
  syncError?: unknown;

  calls(name: string): unknown[][] {
    return this.#calls[name] ?? [];
  }

  directories(): string[] {
    return [...this.#directories].sort();
  }

  exists(path: string): boolean {
    return this.#directories.has(path) || this.#files.has(path);
  }

  readText(path: string): string {
    const value = this.readFile(path, { encoding: "utf8" });
    if (typeof value !== "string") {
      throw new Error(`Expected text file at ${path}`);
    }
    return value;
  }

  mkdirTree(path: string, mode?: number): void {
    this.record("mkdirTree", [path, mode]);
    const normalized = normalizeFakePath(path);
    const parts = normalized.split("/").filter(Boolean);
    let current = "";
    for (const part of parts) {
      current = `${current}/${part}`;
      this.#directories.add(current);
    }
    if (normalized === "/") {
      this.#directories.add("/");
    }
  }

  mkdir(path: string, mode?: number): void {
    this.record("mkdir", [path, mode]);
    const normalized = normalizeFakePath(path);
    const parent = parentPath(normalized);
    if (!this.#directories.has(parent)) {
      throw new Error(`Missing fake parent directory: ${parent}`);
    }
    this.#directories.add(normalized);
  }

  writeFile(path: string, data: string | Uint8Array, opts?: { flags?: string }): void {
    this.record("writeFile", [path, data, opts]);
    const normalized = normalizeFakePath(path);
    const parent = parentPath(normalized);
    if (!this.#directories.has(parent)) {
      throw new Error(`Missing fake parent directory: ${parent}`);
    }
    this.#files.set(normalized, data);
  }

  readFile(path: string, opts?: { encoding?: "utf8" | "binary" }): string | Uint8Array {
    this.record("readFile", [path, opts]);
    const normalized = normalizeFakePath(path);
    const value = this.#files.get(normalized);
    if (value === undefined) {
      throw new Error(`Missing fake file: ${normalized}`);
    }
    if (opts?.encoding === "utf8") {
      return typeof value === "string" ? value : new TextDecoder().decode(value);
    }
    return value;
  }

  analyzePath(path: string): { exists: boolean; error?: unknown } {
    this.record("analyzePath", [path]);
    const normalized = normalizeFakePath(path);
    return { exists: this.exists(normalized) };
  }

  mount(type: unknown, opts: Record<string, unknown>, mountpoint: string): void {
    this.record("mount", [type, opts, mountpoint]);
    if (this.mountError !== undefined) {
      throw this.mountError;
    }
    const normalized = normalizeFakePath(mountpoint);
    if (!this.#directories.has(normalized)) {
      throw new Error(`Missing fake mountpoint: ${normalized}`);
    }
  }

  syncfs(populate: boolean, callback: (error?: unknown) => void): void {
    this.record("syncfs", [populate]);
    callback(this.syncError);
  }

  private record(name: string, args: unknown[]): void {
    (this.#calls[name] ??= []).push(args);
  }
}

function normalizeFakePath(path: string): string {
  const normalized = path.replace(/\/+/g, "/").replace(/\/+$|^(?!.\/)/g, "");
  if (normalized === "") {
    return "/";
  }
  return normalized.startsWith("/") ? normalized : `/${normalized}`;
}

function parentPath(path: string): string {
  if (path === "/") {
    return "/";
  }
  const index = path.lastIndexOf("/");
  return index <= 0 ? "/" : path.slice(0, index);
}
