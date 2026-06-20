import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  timeout: 30_000,
  expect: {
    timeout: 5_000
  },
  use: {
    baseURL: "http://127.0.0.1:43219",
    trace: "on-first-retry"
  },
  webServer: {
    command:
      "cd ../../.. && cargo build -p harness-symphony && target/debug/harness-symphony web --host 127.0.0.1 --port 43219",
    url: "http://127.0.0.1:43219/health",
    reuseExistingServer: false,
    timeout: 30_000
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] }
    }
  ]
});

