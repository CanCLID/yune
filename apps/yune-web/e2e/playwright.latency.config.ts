import { defineConfig } from "@playwright/test";

const appUrl = process.env.YUNE_WEB_APP_URL;
if (!appUrl) {
  throw new Error(
    "YUNE_WEB_APP_URL must identify an already-built public preview or deployed canary",
  );
}

export default defineConfig({
  testDir: ".",
  testMatch: "yune-web-input-latency.spec.ts",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 360_000,
  outputDir:
    process.env.YUNE_WEB_LATENCY_OUTPUT_DIR || "test-results/input-latency",
  expect: {
    timeout: 30_000,
  },
  reporter: "line",
  use: {
    baseURL: appUrl,
    trace:
      process.env.YUNE_WEB_LATENCY_TRACE === "1"
        ? "retain-on-failure"
        : "off",
  },
  projects: [
    {
      name: "chromium",
      use: { browserName: "chromium" },
    },
  ],
});
