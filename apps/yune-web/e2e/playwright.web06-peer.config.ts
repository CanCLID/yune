import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  testMatch: [
    "web06-peer-lane-contract.spec.ts",
    "web06-peer-phase4.spec.ts",
  ],
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: "line",
  use: {
    headless: false,
    viewport: { width: 1365, height: 900 },
    locale: "zh-HK",
    serviceWorkers: "block",
  },
});
