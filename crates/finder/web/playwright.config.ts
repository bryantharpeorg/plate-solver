import { defineConfig } from "@playwright/test";

// End to end: the real `finder` binary, its database and sample frames, serving the built
// page (dist/), driven in Chromium. Set FINDER_URL to test a server that is already
// running; otherwise a release build is started here (the first run compiles it).
const externalURL = process.env.FINDER_URL;
const consoleURL = "http://127.0.0.1:8097";
const sessionURL = "http://127.0.0.1:8098";
const simulateURL = "http://127.0.0.1:8099";

function server(port: number, name: string, extraArgs = "") {
  return {
    // Create state at server startup, never in the user's normal state directory.
    // mktemp gives each server and each run a separate folder, even after a failure.
    command: [
      "mkdir -p target",
      `state_dir=$(mktemp -d target/finder-e2e-${name}.XXXXXX)`,
      `cargo run --release --locked -p finder -- --listen 127.0.0.1:${port} --state "$state_dir"${extraArgs ? ` ${extraArgs}` : ""}`,
    ].join(" && "),
    cwd: "../../..",
    url: `http://127.0.0.1:${port}/api/health`,
    timeout: 20 * 60_000,
    reuseExistingServer: false,
  };
}

export default defineConfig({
  testDir: "e2e",
  timeout: 60_000,
  forbidOnly: !!process.env.CI,
  reporter: "list",
  use: { baseURL: externalURL ?? consoleURL, browserName: "chromium" },
  projects: [
    { name: "default", testIgnore: ["**/session.spec.ts", "**/simulate.spec.ts"] },
    {
      name: "session",
      testMatch: "**/session.spec.ts",
      use: { baseURL: externalURL ?? sessionURL },
      // An explicit URL shares one server: finish the existing suite before edits.
      dependencies: externalURL ? ["default"] : [],
    },
    {
      name: "simulate",
      testMatch: "**/simulate.spec.ts",
      use: { baseURL: externalURL ?? simulateURL },
      dependencies: externalURL ? ["session"] : [],
    },
  ],
  webServer: externalURL
    ? undefined
    : [
        server(8097, "console"),
        server(8098, "session"),
        server(8099, "simulate", "--simulate --sim-start 2026-01-16T02:00:00Z"),
      ],
});
