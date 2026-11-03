import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// `npm run dev` serves the page with hot reload and forwards the API to a running
// `cargo run -p finder` (port 8080). `npm run build` writes dist/, which the Rust binary
// embeds; commit dist/ with the source change. `npm test` runs the page's unit and
// component tests in jsdom; `npm run e2e` drives the real page in Chromium (e2e/).
// `npm run coverage` runs the same unit tests with coverage and writes the report beside
// the Rust one, in target/coverage/web (docs/coverage.md).
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: { "/api": "http://127.0.0.1:8080" },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["src/test/setup.ts"],
    coverage: {
      provider: "v8",
      // Every source file counts, imported by a test or not, so an untested file shows
      // as 0% instead of dropping out of the denominator.
      include: ["src/**/*.{ts,tsx}"],
      // Tests, their helpers and fixtures are not the page. main.tsx only mounts <App />
      // in a real browser, so only the Playwright suite can reach it.
      exclude: ["src/**/*.test.{ts,tsx}", "src/test/**", "src/main.tsx"],
      reporter: ["text", "html", "lcovonly", "json-summary", "json"],
      reportsDirectory: "../../../target/coverage/web",
    },
  },
});
