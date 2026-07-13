import { createHash } from "node:crypto";
import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptRoot = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptRoot, "..");
const repoRoot = path.resolve(appRoot, "../..");
const publicSchemaRoot = path.join(appRoot, "public", "schema");
const expectedSourceRoot = "apps/yune-web/public/schema";
const expectedVersion = "web03-three-schema-launch-v1";
const manifestPaths = [
  path.join(appRoot, "public", "schema-asset-manifest.json"),
  path.join(appRoot, "public-demo", "schema-asset-manifest.json"),
];
const workerPath = path.join(appRoot, "src", "worker.ts");
const schemaOptionsPath = path.join(appRoot, "src", "consts.ts");
const coveragePath = path.join(appRoot, "schema-acceptance-coverage.json");
const expectedCoverageVersion = "m59-reach03-v1";

function repoRelative(file) {
  return path.relative(repoRoot, file).replaceAll(path.sep, "/");
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

async function sha256(file) {
  const data = await readFile(file);
  return createHash("sha256").update(data).digest("hex");
}

async function readJson(file) {
  return JSON.parse(await readFile(file, "utf8"));
}

function sortedStrings(values) {
  return [...values].sort((left, right) => left.localeCompare(right));
}

function assertSameStrings(actual, expected, message) {
  assert(
    JSON.stringify(sortedStrings(actual)) === JSON.stringify(sortedStrings(expected)),
    `${message}: actual=${JSON.stringify(sortedStrings(actual))}, expected=${JSON.stringify(sortedStrings(expected))}`,
  );
}

function declaredSchemaIds(source) {
  const uncommented = source
    .split(/\r?\n/)
    .filter(line => !line.trimStart().startsWith("#"))
    .join("\n");
  const pattern = /(?:^|[\s{,\[])(?:-\s*)?schema\s*:\s*(?:"([^"]+)"|'([^']+)'|([A-Za-z0-9_]+))(?=\s|[,}\]]|$)/gm;
  return [...uncommented.matchAll(pattern)].map(match => match[1] ?? match[2] ?? match[3]);
}

function sourceSchemaId(source) {
  return /^\s*schema_id:\s*([A-Za-z0-9_]+)\s*$/m.exec(source)?.[1];
}

assertSameStrings(
  declaredSchemaIds(`
schema_list:
  - schema: alpha
  - schema: "beta"
flow: [{ schema: 'gamma' }, {schema: delta}]
# - schema: ignored
`),
  ["alpha", "beta", "gamma", "delta"],
  "schema carrier parser must cover block, quoted, and flow forms",
);

function resolveRepoFile(relativePath, label) {
  assert(typeof relativePath === "string" && relativePath.length > 0, `${label} path must be non-empty`);
  assert(!relativePath.includes("\\"), `${label} path must use forward slashes`);
  assert(!path.isAbsolute(relativePath), `${label} path must be repository-relative`);
  assert(!relativePath.split("/").includes(".."), `${label} path must not escape the repository`);
  const resolved = path.resolve(repoRoot, ...relativePath.split("/"));
  assert(
    resolved === repoRoot || resolved.startsWith(`${repoRoot}${path.sep}`),
    `${label} path escapes the repository: ${relativePath}`,
  );
  return resolved;
}

async function treeSchemaAssetPaths() {
  const pending = [publicSchemaRoot];
  const assets = [];
  while (pending.length > 0) {
    const directory = pending.pop();
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const fullPath = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        pending.push(fullPath);
      } else if (entry.isFile() && entry.name.endsWith(".schema.yaml")) {
        assets.push(path.relative(publicSchemaRoot, fullPath).replaceAll(path.sep, "/"));
      }
    }
  }
  return sortedStrings(assets);
}

function configuredSchemaOptionIds(source) {
  const block = /SCHEMA_OPTIONS[^=]*=\s*\[([\s\S]*?)\n\];/m.exec(source)?.[1];
  assert(typeof block === "string", `${repoRelative(schemaOptionsPath)} must declare SCHEMA_OPTIONS`);
  return [...block.matchAll(/\bid\s*:\s*"([A-Za-z0-9_]+)"/g)].map(match => match[1]);
}

function playgroundSchemaEntries(source) {
  const block = /const PLAYGROUND_SCHEMAS[^=]*=\s*{([\s\S]*?)\n};/m.exec(source)?.[1];
  assert(typeof block === "string", `${repoRelative(workerPath)} must declare PLAYGROUND_SCHEMAS`);
  const entries = [];
  for (const match of block.matchAll(/^\s*([A-Za-z0-9_]+):\s*{([\s\S]*?)^\s*},?\s*$/gm)) {
    const body = match[2];
    const runtimeId = /\bruntimeId\s*:\s*"([A-Za-z0-9_]+)"/.exec(body)?.[1];
    assert(typeof runtimeId === "string", `PLAYGROUND_SCHEMAS.${match[1]} must declare runtimeId`);
    entries.push({
      optionId: match[1],
      runtimeId,
      deployedSchemaPath: /\bdeployedSchemaPath\s*:\s*"([^"]+\.schema\.yaml)"/.exec(body)?.[1],
    });
  }
  assert(entries.length > 0, `${repoRelative(workerPath)} PLAYGROUND_SCHEMAS must not be empty`);
  return entries;
}

function canonicalAsset(asset) {
  return {
    path: asset.path,
    sha256: asset.sha256,
    bytes: asset.bytes,
    tier: asset.tier,
    required: asset.required === true,
  };
}

async function validateManifest(file) {
  const manifest = await readJson(file);
  assert(manifest.version === expectedVersion, `${repoRelative(file)} has unexpected version`);
  assert(manifest.generatedFor === "yune-web", `${repoRelative(file)} has unexpected generatedFor`);
  assert(manifest.sourceRoot === expectedSourceRoot, `${repoRelative(file)} has unexpected sourceRoot`);
  assert(Array.isArray(manifest.assets), `${repoRelative(file)} must contain an assets array`);

  const seen = new Set();
  for (const asset of manifest.assets) {
    assert(typeof asset.path === "string" && asset.path.length > 0, "manifest asset path must be non-empty");
    assert(!asset.path.includes("\\"), `${asset.path} must use forward slashes`);
    assert(!asset.path.startsWith("/") && !asset.path.split("/").includes(".."), `${asset.path} must be relative`);
    assert(!asset.path.endsWith(".poet.bin"), `${asset.path} is optional poet storage and must not be public payload`);
    assert(!seen.has(asset.path), `duplicate manifest asset ${asset.path}`);
    seen.add(asset.path);

    const source = path.join(publicSchemaRoot, ...asset.path.split("/"));
    const fileStat = await stat(source);
    assert(fileStat.isFile(), `${asset.path} is not a file`);
    assert(fileStat.size === asset.bytes, `${asset.path} bytes mismatch: manifest ${asset.bytes}, tree ${fileStat.size}`);
    const actualSha = await sha256(source);
    assert(actualSha === asset.sha256, `${asset.path} sha256 mismatch: manifest ${asset.sha256}, tree ${actualSha}`);
  }
  return manifest.assets.map(canonicalAsset);
}

function workerLiteralSchemaAssets(source) {
  const assets = new Set();
  const literalPattern = /["']([^"']+\.(?:yaml|txt|bin|ocd2|json))["']/g;
  let match;
  while ((match = literalPattern.exec(source)) !== null) {
    const candidate = match[1];
    if (
      candidate === "schema-asset-manifest.json" ||
      candidate.includes("${") ||
      candidate.startsWith("http:")
    ) {
      continue;
    }
    if (candidate.startsWith("schema/")) {
      assets.add(candidate.slice("schema/".length));
    } else {
      assets.add(candidate);
    }
  }
  return [...assets].sort();
}

async function assertNoPoetPayloads() {
  const pending = [publicSchemaRoot];
  while (pending.length > 0) {
    const dir = pending.pop();
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        pending.push(fullPath);
      } else if (entry.name.endsWith(".poet.bin")) {
        throw new Error(`${repoRelative(fullPath)} is optional poet storage and must not be committed as public payload`);
      }
    }
  }
}

async function validateAcceptanceCoverage(publicAssets) {
  const coverage = await readJson(coveragePath);
  assert(coverage.version === expectedCoverageVersion, `${repoRelative(coveragePath)} has unexpected version`);
  assert(
    coverage.manifest === "apps/yune-web/public/schema-asset-manifest.json",
    `${repoRelative(coveragePath)} must bind the canonical public manifest`,
  );
  assert(
    typeof coverage.mechanismContract?.schemaGeneralDefault === "string" &&
      coverage.mechanismContract.schemaGeneralDefault.includes("leading_syllable_reachability"),
    "coverage registry must document the schema-general reachability owner",
  );
  assert(
    typeof coverage.mechanismContract?.profilePrecedence === "string" &&
      coverage.mechanismContract.profilePrecedence.includes("prefix_fallback"),
    "coverage registry must document TypeDuck profile precedence",
  );

  const schemaAssetPaths = publicAssets
    .map(asset => asset.path)
    .filter(assetPath => assetPath.endsWith(".schema.yaml"));
  assert(Array.isArray(coverage.schemaAssets), "coverage registry must contain schemaAssets");
  assert(
    JSON.stringify(coverage.schemaAssets.map(row => row.asset)) === JSON.stringify(schemaAssetPaths),
    `coverage registry must follow manifest schema-asset order exactly: manifest=${JSON.stringify(schemaAssetPaths)}, coverage=${JSON.stringify(coverage.schemaAssets.map(row => row.asset))}`,
  );

  const seenAssets = new Set();
  const coverageByAsset = new Map(coverage.schemaAssets.map(row => [row.asset, row]));
  const acceptanceById = new Map();
  const allowedDispositions = new Set([
    "top_level_selectable",
    "runtime_alias",
    "deployed_runtime_mirror",
    "dev_selectable",
    "dependency_only",
  ]);
  for (const row of coverage.schemaAssets) {
    assert(typeof row.asset === "string" && row.asset.length > 0, "coverage asset must be non-empty");
    assert(!seenAssets.has(row.asset), `duplicate coverage asset ${row.asset}`);
    seenAssets.add(row.asset);
    assert(row.status === "accepted", `${row.asset} coverage is unresolved (${row.status ?? "missing status"})`);
    assert(allowedDispositions.has(row.disposition), `${row.asset} has unknown disposition ${row.disposition}`);
    assert(typeof row.schemaId === "string" && row.schemaId.length > 0, `${row.asset} must name schemaId`);
    assert(typeof row.acceptanceId === "string" && row.acceptanceId.length > 0, `${row.asset} must name acceptanceId`);

    const schemaSourcePath = path.join(publicSchemaRoot, ...row.asset.split("/"));
    const schemaSource = await readFile(schemaSourcePath, "utf8");
    assert(
      sourceSchemaId(schemaSource) === row.schemaId,
      `${row.asset} schema_id does not match coverage schemaId ${row.schemaId}`,
    );

    if (row.disposition === "dependency_only") {
      assert(row.acceptance === undefined, `${row.asset} dependency-only row must not invent a top-level acceptance`);
      assert(
        Array.isArray(row.dependencyOwners) && row.dependencyOwners.length > 0,
        `${row.asset} dependency-only row must name its top-level owner`,
      );
      for (const owner of row.dependencyOwners) {
        const ownerRow = coverageByAsset.get(owner);
        assert(ownerRow !== undefined, `${row.asset} names unknown dependency owner ${owner}`);
        assert(ownerRow.disposition !== "dependency_only", `${row.asset} dependency owner ${owner} is not top-level`);
      }
      assert(typeof row.reason === "string" && row.reason.length > 0, `${row.asset} dependency disposition needs a reason`);
      continue;
    }

    const acceptance = row.acceptance;
    assert(acceptance !== null && typeof acceptance === "object", `${row.asset} must bind executable acceptance`);
    for (const field of [
      "testFile",
      "testName",
      "caseLabel",
      "oracleFixture",
      "input",
      "expectedFinal",
      "defaultOn",
      "explicitFalse",
    ]) {
      assert(typeof acceptance[field] === "string" && acceptance[field].length > 0, `${row.asset} acceptance.${field} is required`);
    }
    const testPath = resolveRepoFile(acceptance.testFile, `${row.asset} acceptance test`);
    const testSource = await readFile(testPath, "utf8");
    assert(
      testSource.includes(`fn ${acceptance.testName}(`),
      `${row.asset} acceptance test ${acceptance.testName} is missing from ${acceptance.testFile}`,
    );
    assert(
      testSource.includes(`label: "${acceptance.caseLabel}"`),
      `${row.asset} acceptance case ${acceptance.caseLabel} is missing from ${acceptance.testFile}`,
    );
    assert(testSource.includes(acceptance.input), `${row.asset} acceptance input is not bound by its test source`);
    assert(testSource.includes(acceptance.expectedFinal), `${row.asset} expected output is not bound by its test source`);
    const fixturePath = resolveRepoFile(acceptance.oracleFixture, `${row.asset} oracle fixture`);
    assert((await stat(fixturePath)).isFile(), `${row.asset} oracle fixture is not a file: ${acceptance.oracleFixture}`);

    const priorAcceptance = acceptanceById.get(row.acceptanceId);
    if (priorAcceptance === undefined) {
      acceptanceById.set(row.acceptanceId, acceptance);
    } else {
      assert(
        JSON.stringify(priorAcceptance) === JSON.stringify(acceptance),
        `${row.asset} reuses acceptanceId ${row.acceptanceId} with different evidence`,
      );
    }
  }

  const carrierAssetPaths = publicAssets
    .map(asset => asset.path)
    .filter(assetPath => /(^|\/)default(?:\.custom)?\.yaml$/.test(assetPath));
  assert(Array.isArray(coverage.configurationCarriers), "coverage registry must name configuration carriers");
  assertSameStrings(
    coverage.configurationCarriers.map(carrier => carrier.asset),
    carrierAssetPaths,
    "coverage configuration carriers must match manifested default YAML assets",
  );
  for (const carrier of coverage.configurationCarriers) {
    assert(Array.isArray(carrier.declaredSchemaIds), `${carrier.asset} must declare schema ids`);
    const carrierPath = path.join(publicSchemaRoot, ...carrier.asset.split("/"));
    const source = await readFile(carrierPath, "utf8");
    assertSameStrings(
      declaredSchemaIds(source),
      carrier.declaredSchemaIds,
      `${carrier.asset} declared schema ids changed`,
    );
    for (const schemaId of carrier.declaredSchemaIds) {
      const accepted = coverage.schemaAssets.some(
        row => row.schemaId === schemaId && row.status === "accepted" && row.disposition !== "dependency_only" && row.acceptance,
      );
      assert(accepted, `${carrier.asset} promotes ${schemaId} without accepted top-level real-path coverage`);
    }
  }

  const schemaOptionSource = await readFile(schemaOptionsPath, "utf8");
  const schemaOptionIds = configuredSchemaOptionIds(schemaOptionSource);
  for (const schemaId of schemaOptionIds) {
    const accepted = coverage.schemaAssets.some(
      row => row.schemaId === schemaId && row.status === "accepted" && row.disposition !== "dependency_only" && row.acceptance,
    );
    assert(accepted, `SCHEMA_OPTIONS exposes ${schemaId} without accepted real-path coverage`);
  }
  const workerSource = await readFile(workerPath, "utf8");
  const playgroundSchemas = playgroundSchemaEntries(workerSource);
  assertSameStrings(
    playgroundSchemas.map(entry => entry.optionId),
    schemaOptionIds,
    "SCHEMA_OPTIONS and PLAYGROUND_SCHEMAS keys must match",
  );
  for (const entry of playgroundSchemas) {
    const runtimeAccepted = coverage.schemaAssets.some(
      row => row.schemaId === entry.runtimeId && row.status === "accepted" && row.disposition !== "dependency_only" && row.acceptance,
    );
    assert(
      runtimeAccepted,
      `PLAYGROUND_SCHEMAS.${entry.optionId} runtimeId ${entry.runtimeId} lacks accepted real-path coverage`,
    );
    if (entry.deployedSchemaPath !== undefined) {
      const deployed = coverage.schemaAssets.find(row => row.asset === entry.deployedSchemaPath);
      assert(
        deployed?.status === "accepted" && deployed.disposition !== "dependency_only" && deployed.acceptance,
        `PLAYGROUND_SCHEMAS.${entry.optionId} deployed schema ${entry.deployedSchemaPath} lacks accepted coverage`,
      );
    }
  }

  const requiredValidationLabels = [
    "product-jyutping",
    "canonical-jyutping",
    "cangjie5",
    "luna-pinyin",
    "luna-pinyin-octagram",
    "double-pinyin",
    "bopomofo",
    "stroke-null-map",
    "product-jyutping-plain",
  ];
  assert(Array.isArray(coverage.validationRows), "coverage registry must bind the mandatory deployment matrix");
  assertSameStrings(
    coverage.validationRows.map(row => row.caseLabel),
    requiredValidationLabels,
    "coverage validation rows must match the mandatory deployment matrix",
  );
  const validationTestPath = resolveRepoFile(coverage.validationTestFile, "validation test");
  const validationTestSource = await readFile(validationTestPath, "utf8");
  assert(
    validationTestSource.includes("SCHEMA_ACCEPTANCE_COVERAGE"),
    "deployment matrix test must consume the checked-in coverage registry",
  );
  const seenValidationLabels = new Set();
  const validationByAcceptanceId = new Map();
  for (const row of coverage.validationRows) {
    assert(!seenValidationLabels.has(row.caseLabel), `duplicate validation row ${row.caseLabel}`);
    seenValidationLabels.add(row.caseLabel);
    for (const field of [
      "caseLabel",
      "schemaId",
      "acceptanceId",
      "scope",
      "testName",
      "evidenceFile",
      "input",
      "expectedFinal",
      "defaultOn",
      "explicitFalse",
    ]) {
      assert(typeof row[field] === "string" && row[field].length > 0, `validation ${row.caseLabel}.${field} is required`);
    }
    assert(row.status === "accepted", `validation ${row.caseLabel} is unresolved (${row.status ?? "missing status"})`);
    assert(
      !validationByAcceptanceId.has(row.acceptanceId),
      `duplicate validation acceptanceId ${row.acceptanceId}`,
    );
    validationByAcceptanceId.set(row.acceptanceId, row);
    const evidencePath = resolveRepoFile(row.evidenceFile, `validation ${row.caseLabel} evidence`);
    assert((await stat(evidencePath)).isFile(), `validation ${row.caseLabel} evidence is not a file`);
    assert(
      validationTestSource.includes(`fn ${row.testName}(`),
      `validation ${row.caseLabel} test ${row.testName} is missing`,
    );
    assert(
      validationTestSource.includes(`label: "${row.caseLabel}"`),
      `validation ${row.caseLabel} case is missing from the executable matrix`,
    );
  }
  for (const schemaRow of coverage.schemaAssets.filter(row => row.disposition !== "dependency_only")) {
    const validation = validationByAcceptanceId.get(schemaRow.acceptanceId);
    assert(
      validation !== undefined,
      `${schemaRow.asset} acceptanceId ${schemaRow.acceptanceId} does not resolve to an executable validation row`,
    );
    const acceptance = schemaRow.acceptance;
    for (const [validationField, acceptanceField] of [
      ["caseLabel", "caseLabel"],
      ["testName", "testName"],
      ["evidenceFile", "oracleFixture"],
      ["input", "input"],
      ["expectedFinal", "expectedFinal"],
      ["defaultOn", "defaultOn"],
      ["explicitFalse", "explicitFalse"],
    ]) {
      assert(
        validation[validationField] === acceptance[acceptanceField],
        `${schemaRow.asset} ${acceptanceField} does not match validation ${schemaRow.acceptanceId}`,
      );
    }
    assert(
      validation.schemaId === schemaRow.schemaId,
      `${schemaRow.asset} schemaId does not match validation ${schemaRow.acceptanceId}`,
    );
  }
}

const [publicAssets, publicDemoAssets] = await Promise.all(manifestPaths.map(validateManifest));
assert(
  JSON.stringify(publicAssets) === JSON.stringify(publicDemoAssets),
  "public and public-demo schema asset manifests must be identical",
);

const manifestAssetPaths = new Set(publicAssets.map(asset => asset.path));
assertSameStrings(
  publicAssets.map(asset => asset.path).filter(assetPath => assetPath.endsWith(".schema.yaml")),
  await treeSchemaAssetPaths(),
  "manifest schema assets must match every tracked public schema asset",
);
const workerSource = await readFile(workerPath, "utf8");
for (const assetPath of workerLiteralSchemaAssets(workerSource)) {
  assert(
    manifestAssetPaths.has(assetPath),
    `worker references ${assetPath}, but it is missing from schema-asset-manifest.json`,
  );
}
await assertNoPoetPayloads();
await validateAcceptanceCoverage(publicAssets);

console.log(
  `Schema asset manifests verified: ${publicAssets.length} assets, ${manifestPaths.map(repoRelative).join(", ")}; M59 REACH-03 coverage accepted`,
);
