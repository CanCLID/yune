import { createHash } from "node:crypto";
import { createWriteStream } from "node:fs";
import { mkdir, readFile, rm, stat } from "node:fs/promises";
import { get } from "node:https";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appRoot = join(scriptDir, "..");
const repoRoot = join(appRoot, "..", "..");
const pinsPath = join(repoRoot, "docs", "reports", "evidence", "m54-native-octagram-grammar-support", "external-pins.json");
const outputPath = join(appRoot, "public", "schema", "dev", "octagram", "zh-hant-t-essay-bgw.gram");

const pins = JSON.parse(await readFile(pinsPath, "utf8"));
const lotem = pins.canonical_lotem_lane;
const model = lotem.hant_branch.models.find(entry => entry.path === lotem.selected_default_model);

if (!model) {
  throw new Error(`Cannot find ${lotem.selected_default_model} in ${pinsPath}`);
}

const url = `${lotem.config.url.replace("github.com", "raw.githubusercontent.com")}/${lotem.hant_branch.commit}/${model.path}`;
const expectedSha256 = model.sha256;
const expectedBytes = model.bytes;

await mkdir(dirname(outputPath), { recursive: true });
await download(url, outputPath);

const bytes = (await stat(outputPath)).size;
const sha256 = createHash("sha256").update(await readFile(outputPath)).digest("hex");

if (bytes !== expectedBytes || sha256 !== expectedSha256) {
  await rm(outputPath, { force: true });
  throw new Error(
    `Downloaded octagram model failed verification: bytes ${bytes}/${expectedBytes}, sha256 ${sha256}/${expectedSha256}`,
  );
}

console.log(JSON.stringify({
  model: model.path,
  source: lotem.config.url,
  branch: "hant",
  commit: lotem.hant_branch.commit,
  url,
  outputPath,
  bytes,
  sha256,
}, null, 2));

function download(url, path) {
  return new Promise((resolve, reject) => {
    const request = get(url, response => {
      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`GET ${url} failed with ${response.statusCode}`));
        return;
      }
      const file = createWriteStream(path, { flags: "w" });
      response.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", reject);
    });
    request.on("error", reject);
  });
}
