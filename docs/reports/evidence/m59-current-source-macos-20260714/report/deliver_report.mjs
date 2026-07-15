#!/usr/bin/env node

import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

function usage() {
  return [
    "Usage: node deliver_report.mjs --plugin-root <data-analytics-plugin> --input <artifact.json> --output <report.html>",
    "",
    "Builds the portable report with the canonical Data Analytics delivery pipeline.",
    "A narrow header-width guard compensates for the packaged reader's 100vw",
    "desktop header while preserving each table's own horizontal scrolling.",
  ].join("\n");
}

function parseArguments(argv) {
  const values = {};
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--help" || argument === "-h") return { help: true };
    if (!["--plugin-root", "--input", "--output"].includes(argument)) {
      throw new Error(`Unexpected argument: ${argument}\n${usage()}`);
    }
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`Missing value for ${argument}.\n${usage()}`);
    }
    values[argument.slice(2)] = value;
    index += 1;
  }
  if (!values["plugin-root"] || !values.input || !values.output) {
    throw new Error(usage());
  }
  return values;
}

function addHeaderWidthGuard(html) {
  const marker = "</head>";
  const matches = html.split(marker).length - 1;
  if (matches !== 1) {
    throw new Error(
      `Expected one portable-reader head marker, found ${matches}; refusing an unverified rewrite.`,
    );
  }
  const guard = [
    '<style data-yune-report-overflow-guard="true">',
    "html,body,#data-analytics-portable-reader,.portable-fallback{",
    "max-width:100%!important;",
    "overflow-x:clip!important",
    "}",
    "@media screen and (min-width:761px){",
    "html body .portable-page-header{",
    "width:calc(100% + 64px)!important;",
    "margin-right:-32px!important;",
    "margin-left:-32px!important",
    "}",
    "}",
    "</style>",
  ].join("");
  return html.replace(marker, `${guard}${marker}`);
}

const options = parseArguments(process.argv.slice(2));
if (options.help) {
  process.stdout.write(`${usage()}\n`);
  process.exit(0);
}

const scripts = resolve(options["plugin-root"], "skills/build-report/scripts");
const { buildPortableArtifact } = await import(
  pathToFileURL(resolve(scripts, "build_portable_artifact.mjs")).href
);
const { deliverPortableArtifact } = await import(
  pathToFileURL(resolve(scripts, "deliver_portable_artifact.mjs")).href
);

try {
  const result = await deliverPortableArtifact(
    {
      inputPath: resolve(options.input),
      outputPath: resolve(options.output),
    },
    {
      build(input, buildOptions) {
        return addHeaderWidthGuard(buildPortableArtifact(input, buildOptions));
      },
    },
  );
  process.stdout.write(`${JSON.stringify(result)}\n`);
} catch (error) {
  const result = error?.deliveryResult ?? error?.verificationResult ?? {
    ok: false,
    code: error?.code ?? "delivery_failed",
    error: error?.message ?? String(error),
  };
  process.stderr.write(`${JSON.stringify(result)}\n`);
  process.exitCode = 1;
}
