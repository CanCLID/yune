import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  realpath,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  installPinnedWranglerRuntime,
  preparePinnedWranglerRuntime,
  WEB06_WRANGLER_VERSION,
  wranglerDeploymentEnvironment,
} from "./web06-wrangler-runtime.mjs";

const WRANGLER_RESOLVED =
  "https://registry.npmjs.org/wrangler/-/wrangler-4.111.0.tgz";
const WRANGLER_INTEGRITY =
  "sha512-bffpI9EyrnpKkF/1S+RaIv8oRD93GtbsA7TlfWwOsGJGB7VO3jVbdGzpC9TU7Bqom3z7jUxcte4Z9MPhaQ4HoQ==";
const sha256 = (bytes) =>
  createHash("sha256").update(bytes).digest("hex");

async function runtimeFixture() {
  const requested = await mkdtemp(
    path.join(tmpdir(), "web06-wrangler-runtime-"),
  );
  const root = await realpath(requested);
  const wranglerRoot = path.join(root, "node_modules", "wrangler");
  const wranglerDistRoot = path.join(root, "node_modules", "wrangler-dist");
  await mkdir(path.join(wranglerRoot, "bin"), { recursive: true });
  await mkdir(wranglerDistRoot, { recursive: true });
  const packagePayload = {
    name: "@yune-ime/yune-web-e2e",
    private: true,
    type: "module",
    devDependencies: { wrangler: WEB06_WRANGLER_VERSION },
  };
  const lockPayload = {
    name: packagePayload.name,
    lockfileVersion: 3,
    requires: true,
    packages: {
      "": {
        name: packagePayload.name,
        devDependencies: { wrangler: WEB06_WRANGLER_VERSION },
      },
      "node_modules/wrangler": {
        version: WEB06_WRANGLER_VERSION,
        resolved: WRANGLER_RESOLVED,
        integrity: WRANGLER_INTEGRITY,
        bin: { wrangler: "bin/wrangler.js" },
      },
    },
  };
  const wranglerPayload = {
    name: "wrangler",
    version: WEB06_WRANGLER_VERSION,
    bin: { wrangler: "bin/wrangler.js" },
  };
  const entrypoint = path.join(wranglerRoot, "bin", "wrangler.js");
  await writeFile(
    path.join(root, "package.json"),
    `${JSON.stringify(packagePayload)}\n`,
  );
  await writeFile(
    path.join(root, "package-lock.json"),
    `${JSON.stringify(lockPayload)}\n`,
  );
  await writeFile(
    path.join(wranglerDistRoot, "cli.js"),
    `export const version = "${WEB06_WRANGLER_VERSION}";\n`,
  );
  await writeFile(
    path.join(wranglerRoot, "package.json"),
    `${JSON.stringify(wranglerPayload)}\n`,
  );
  await writeFile(
    entrypoint,
    `import "../../wrangler-dist/cli.js";\nconsole.log("${WEB06_WRANGLER_VERSION}");\n`,
  );
  return {
    root,
    entrypoint,
    transitive: path.join(wranglerDistRoot, "cli.js"),
  };
}

function fakeVersionRunner(calls) {
  return async (command, arguments_, environment) => {
    calls.push({ command, arguments_, environment });
    if (arguments_.length === 1 && arguments_[0] === "--version") {
      return { stdout: `${process.version}\n`, stderr: "" };
    }
    return { stdout: `${WEB06_WRANGLER_VERSION}\n`, stderr: "" };
  };
}

test("pinned Wrangler binds lock, package, entrypoint, and Node bytes", async () => {
  const fixture = await runtimeFixture();
  try {
    const calls = [];
    const runtime = await preparePinnedWranglerRuntime({
      root: fixture.root,
      executeVersion: fakeVersionRunner(calls),
      baseEnvironment: {
        PATH: "/attacker",
        NODE_OPTIONS: "--require=/attacker.js",
        NODE_PATH: "/attacker",
        npm_config_registry: "https://attacker.invalid",
        CLOUDFLARE_API_TOKEN: "secret",
      },
    });
    assert.equal(runtime.command, await realpath(process.execPath));
    assert.deepEqual(runtime.argumentsPrefix, [fixture.entrypoint]);
    assert.equal(runtime.identity.version, WEB06_WRANGLER_VERSION);
    assert.equal(runtime.identity.resolved, WRANGLER_RESOLVED);
    assert.equal(runtime.identity.integrity, WRANGLER_INTEGRITY);
    assert.equal(runtime.identity.nodeVersion, process.version);
    assert.equal(
      runtime.identity.entrypointSha256,
      sha256(await readFile(fixture.entrypoint)),
    );
    assert.equal(calls.length, 2);
    for (const call of calls) {
      assert.equal(path.isAbsolute(call.command), true);
      assert.deepEqual(
        Object.keys(call.environment).sort(),
        ["LANG", "LC_ALL", "NO_COLOR", "WRANGLER_SEND_METRICS"].sort(),
      );
    }
    await runtime.assertCurrent();
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test("runtime mutation after credential-free admission fails closed", async () => {
  const fixture = await runtimeFixture();
  try {
    const runtime = await preparePinnedWranglerRuntime({
      root: fixture.root,
      executeVersion: fakeVersionRunner([]),
    });
    await writeFile(fixture.entrypoint, "console.log('mutated');\n");
    await assert.rejects(
      runtime.assertCurrent(),
      /runtime bytes changed after admission/,
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test("transitive runtime mutation after admission fails closed", async () => {
  const fixture = await runtimeFixture();
  try {
    const runtime = await preparePinnedWranglerRuntime({
      root: fixture.root,
      executeVersion: fakeVersionRunner([]),
    });
    await writeFile(
      fixture.transitive,
      "console.log(process.env.CLOUDFLARE_API_TOKEN);\n",
    );
    await assert.rejects(
      runtime.assertCurrent(),
      /runtime bytes changed after admission/,
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test("lock drift and symbolic-link entrypoints are rejected", async () => {
  const fixture = await runtimeFixture();
  try {
    const lockPath = path.join(fixture.root, "package-lock.json");
    const lock = JSON.parse(await readFile(lockPath, "utf8"));
    lock.packages["node_modules/wrangler"].integrity =
      "sha512-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    await writeFile(lockPath, `${JSON.stringify(lock)}\n`);
    await assert.rejects(
      preparePinnedWranglerRuntime({
        root: fixture.root,
        executeVersion: fakeVersionRunner([]),
      }),
      /lock identity is invalid/,
    );

    lock.packages["node_modules/wrangler"].integrity = WRANGLER_INTEGRITY;
    await writeFile(lockPath, `${JSON.stringify(lock)}\n`);
    const target = path.join(fixture.root, "attacker.mjs");
    await writeFile(target, `console.log("${WEB06_WRANGLER_VERSION}");\n`);
    await rm(fixture.entrypoint);
    await symlink(target, fixture.entrypoint);
    await assert.rejects(
      preparePinnedWranglerRuntime({
        root: fixture.root,
        executeVersion: fakeVersionRunner([]),
      }),
      /path is not canonical|plain file/,
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test("token-bearing Wrangler environment is a fixed minimal allowlist", () => {
  const environment = wranglerDeploymentEnvironment({
    accountId: "account",
    apiToken: "token",
    home: "/private/run/home",
    temporaryDirectory: "/private/run/tmp",
    baseEnvironment: {
      PATH: "/attacker",
      NODE_OPTIONS: "--require=/attacker.js",
      NODE_PATH: "/attacker",
      npm_config_registry: "https://attacker.invalid",
      CLOUDFLARE_API_KEY: "wrong-key",
      CF_API_TOKEN: "wrong-token",
      WRANGLER_AUTH_DOMAIN: "attacker.invalid",
      LANG: "en_US.UTF-8",
    },
  });
  assert.deepEqual(environment, {
    CI: "1",
    CLOUDFLARE_ACCOUNT_ID: "account",
    CLOUDFLARE_API_TOKEN: "token",
    HOME: "/private/run/home",
    LANG: "en_US.UTF-8",
    LC_ALL: "C",
    NO_COLOR: "1",
    TMPDIR: "/private/run/tmp",
    WRANGLER_SEND_METRICS: "false",
  });
});

test("credential-free install uses a create-new private root and scrubbed npm environment", async () => {
  const source = await runtimeFixture();
  const parent = await mkdtemp(
    path.join(tmpdir(), "web06-wrangler-install-parent-"),
  );
  const installationRoot = path.join(await realpath(parent), "runtime");
  try {
    const calls = [];
    const installed = await installPinnedWranglerRuntime({
      installationRoot,
      sourceRoot: source.root,
      baseEnvironment: {
        PATH: "/trusted/bin",
        NODE_OPTIONS: "--require=/attacker.js",
        NODE_PATH: "/attacker",
        npm_config_registry: "https://attacker.invalid",
        CLOUDFLARE_API_TOKEN: "secret",
        CF_API_KEY: "secret",
        WRANGLER_AUTH_DOMAIN: "secret",
      },
      executeInstall: async (command, arguments_, options) => {
        calls.push({ command, arguments_, options });
        await mkdir(path.join(options.cwd, "node_modules"));
        await writeFile(
          path.join(options.cwd, "node_modules", "installed.txt"),
          "installed\n",
        );
        return { stdout: "", stderr: "" };
      },
    });
    assert.equal(installed.root, installationRoot);
    assert.equal((await lstat(installed.root)).mode & 0o777, 0o700);
    assert.match(installed.identity.npmVersion, /^\d+\.\d+\.\d+/u);
    assert.match(installed.identity.npmCliSha256, /^[0-9a-f]{64}$/u);
    assert.equal(calls.length, 1);
    assert.equal(calls[0].command, await realpath(process.execPath));
    assert.equal(path.isAbsolute(calls[0].arguments_[0]), true);
    assert.deepEqual(calls[0].arguments_.slice(1), [
      "ci",
      "--ignore-scripts",
      "--no-audit",
      "--no-fund",
    ]);
    assert.equal(calls[0].options.cwd, installationRoot);
    assert.equal(
      calls[0].options.env.PATH,
      `${path.dirname(await realpath(process.execPath))}:/usr/bin:/bin`,
    );
    assert.equal(
      Object.keys(calls[0].options.env).some(
        (name) =>
          name === "NODE_OPTIONS" ||
          name === "NODE_PATH" ||
          name.startsWith("CLOUDFLARE_") ||
          name.startsWith("CF_") ||
          name.startsWith("WRANGLER_") ||
          name === "npm_config_registry",
      ),
      false,
    );
    await assert.rejects(
      installPinnedWranglerRuntime({
        installationRoot,
        sourceRoot: source.root,
        executeInstall: async () => assert.fail("install must not run"),
      }),
      /installation root must be create-new/,
    );
  } finally {
    await rm(source.root, { recursive: true, force: true });
    await rm(parent, { recursive: true, force: true });
  }
});
