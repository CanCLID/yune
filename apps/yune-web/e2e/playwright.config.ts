import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  testMatch: ["yune-web.spec.ts", "yune-web-startup-benchmark.spec.ts"],
  fullyParallel: true,
  workers: process.env.CI ? 2 : 4,
  reporter: "line",
  use: {
    baseURL: process.env.YUNE_WEB_APP_URL || "http://localhost:5173",
  },
});
