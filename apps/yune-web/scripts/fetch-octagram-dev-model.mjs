import { createHash } from "node:crypto";
import { mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appRoot = join(scriptDir, "..");
const repoRoot = join(appRoot, "..", "..");
const pinsPath = join(
  repoRoot,
  "docs",
  "reports",
  "evidence",
  "m54-native-octagram-grammar-support",
  "external-pins.json",
);
const outputPath = join(
  appRoot,
  "public",
  "schema",
  "dev",
  "octagram",
  "zh-hant-t-essay-bgw.gram",
);
const tmpPath = `${outputPath}.tmp-${process.pid}`;

const pins = JSON.parse(await readFile(pinsPath, "utf8"));
const lotem = pins.canonical_lotem_lane;
const model = lotem.hant_branch.models.find(
  entry => entry.path === lotem.selected_default_model,
);

if (!model) {
  throw new Error(`Cannot find ${lotem.selected_default_model} in ${pinsPath}`);
}

const url = `${lotem.config.url.replace(
  "github.com",
  "raw.githubusercontent.com",
)}/${lotem.hant_branch.commit}/${model.path}`;
const expectedSha256 = model.sha256;
const expectedBytes = model.bytes;

try {
  const data = await downloadBytes(url);
  const bytes = data.byteLength;
  const sha256 = sha256Hex(data);

  if (bytes !== expectedBytes || sha256 !== expectedSha256) {
    throw new Error(
      `Downloaded octagram model failed verification: bytes ${bytes}/${expectedBytes}, sha256 ${sha256}/${expectedSha256}`,
    );
  }

  await mkdir(dirname(outputPath), { recursive: true });
  await writeFile(tmpPath, data);
  await rename(tmpPath, outputPath);

  console.log(
    JSON.stringify(
      {
        model: model.path,
        source: lotem.config.url,
        branch: "hant",
        commit: lotem.hant_branch.commit,
        url,
        outputPath,
        bytes,
        sha256,
      },
      null,
      2,
    ),
  );
} finally {
  await rm(tmpPath, { force: true }).catch(() => {});
}

async function downloadBytes(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`GET ${url} failed with ${response.status}`);
  }
  return Buffer.from(await response.arrayBuffer());
}

function sha256Hex(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}
