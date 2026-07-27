import { defineConfig } from "@playwright/test";

for (const legacy of [
  "YUNE_WEB_WEB06_GATE_SCOPE",
  "YUNE_WEB_WEB06_EVIDENCE_DIR",
  "YUNE_WEB_WEB06_OUTPUT_DIR",
  "YUNE_WEB_WEB06_DIST_ROOT",
]) {
  if (process.env[legacy]?.trim()) {
    throw new Error(`Legacy WEB06 environment is forbidden: ${legacy}`);
  }
}

function required(name: string): string {
  const value = process.env[name]?.trim();
  if (!value || /[\0\r\n]/.test(value)) {
    throw new Error(`${name} is required and must be single-line`);
  }
  return value;
}

const outputDir = required("YUNE_WEB06_PLAYWRIGHT_OUTPUT_DIR");
const workers = required("YUNE_WEB06_PLAYWRIGHT_WORKERS");
const retries = required("YUNE_WEB06_PLAYWRIGHT_RETRIES");
if (workers !== "1" || retries !== "0") {
  throw new Error("WEB06 requires exactly one Playwright worker and zero retries");
}
for (const name of [
  "YUNE_WEB06_EXPECTATION",
  "YUNE_WEB06_RUN_KIND",
  "YUNE_WEB06_SELECTED_BRANCH",
  "YUNE_WEB06_DISPOSITION",
  "YUNE_WEB06_IDENTITY_MANIFEST_JSON",
  "YUNE_WEB06_TARGETS_JSON",
  "YUNE_WEB06_TARGET_ORDER_JSON",
  "YUNE_WEB06_SCENARIOS_JSON",
  "YUNE_WEB06_RUN_ID",
  "YUNE_WEB06_EVIDENCE_ROOT",
  "YUNE_WEB06_SUITE_ATTESTATION_PATH",
  "YUNE_WEB06_COLLECTOR_OUTPUT_PATH",
  "YUNE_WEB06_INDEPENDENT_RECOMPUTE_PATH",
]) {
  required(name);
}

export default defineConfig({
  testDir: ".",
  testMatch: "yune-web06-smoothness.spec.ts",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  outputDir,
  use: {
    baseURL: process.env.YUNE_WEB_APP_URL?.trim() || undefined,
  },
});
