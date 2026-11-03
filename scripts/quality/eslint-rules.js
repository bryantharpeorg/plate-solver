// The finder page's lint for scripts/quality.sh: code smells, report only.
//
// The page has no lint of its own (its checks are tsc, Prettier and its tests), and this does
// not add one: only quality.sh runs it (through eslint.js), and nothing fails on what it
// finds. The rules are eslint's and typescript-eslint's recommended sets, React's hooks
// rules, and SonarSource's own rules for JavaScript and TypeScript (eslint-plugin-sonarjs),
// the nearest thing to the SonarQube profile the report imitates. docs/quality.md explains
// the severities.
import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import sonarjs from "eslint-plugin-sonarjs";
import tseslint from "typescript-eslint";

export const files = ["crates/finder/web/src/**/*.{ts,tsx}"];
// What the coverage report leaves out too: the tests and their helpers.
export const ignores = ["**/*.test.{ts,tsx}", "crates/finder/web/src/test/**"];

export default [
  { files, ignores, ...js.configs.recommended },
  ...tseslint.configs.recommended.map((c) => ({ ...c, files, ignores })),
  { files, ignores, ...reactHooks.configs.flat.recommended },
  { files, ignores, ...sonarjs.configs.recommended },
  {
    files,
    ignores,
    rules: {
      // Complexity is measured by rust-code-analysis for both languages, one way
      // (docs/quality.md). eslint.js runs this rule on its own, as a cross-check.
      "sonarjs/cognitive-complexity": "off",
    },
  },
];
