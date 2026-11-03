// node scripts/quality/eslint.js ROOT OUT_DIR — lint the finder page for quality.sh.
//
// ESLint 10 only lints files under its config file's directory, and the rules live here,
// beside their pinned plugins, so this drives the API with ROOT as the base path instead.
// Writes OUT_DIR/eslint.json (ESLint's JSON format), OUT_DIR/eslint-meta.json (each reported
// rule's type and docs link) and OUT_DIR/eslint-cognitive.json: the cognitive complexity of
// every function by sonarjs, SonarSource's own implementation of the metric.
import { writeFile } from "node:fs/promises";
import path from "node:path";
import { ESLint } from "eslint";
import sonarjs from "eslint-plugin-sonarjs";
import tseslint from "typescript-eslint";
import rules, { files, ignores } from "./eslint-rules.js";

const [root, out] = process.argv.slice(2);
if (!root || !out) {
  console.error("usage: node eslint.js ROOT OUT_DIR");
  process.exit(2);
}

function linter(overrideConfig) {
  return new ESLint({ cwd: path.resolve(root), overrideConfigFile: true, overrideConfig });
}

const eslint = linter(rules);
const results = await eslint.lintFiles(["crates/finder/web/src"]);
await writeFile(path.join(out, "eslint.json"), JSON.stringify(results, null, 1));
const meta = Object.fromEntries(
  Object.entries(eslint.getRulesMetaForResults(results)).map(([id, m]) => [id, { type: m.type, url: m.docs?.url }]),
);
await writeFile(path.join(out, "eslint-meta.json"), JSON.stringify(meta, null, 1));

// Threshold 0 makes the rule report every function that has any complexity at all.
const cognitive = await linter([
  {
    files,
    ignores,
    languageOptions: { parser: tseslint.parser },
    plugins: { sonarjs },
    rules: { "sonarjs/cognitive-complexity": ["error", 0] },
  },
]).lintFiles(["crates/finder/web/src"]);
const functions = cognitive.flatMap((r) =>
  r.messages
    .filter((m) => m.ruleId === "sonarjs/cognitive-complexity")
    .map((m) => ({
      path: path.relative(root, r.filePath),
      line: m.line,
      column: m.column,
      cognitive: Number(/from (\d+) to/.exec(m.message)?.[1]),
    })),
);
await writeFile(path.join(out, "eslint-cognitive.json"), JSON.stringify(functions, null, 1));
console.error(`eslint: ${results.reduce((n, r) => n + r.messages.length, 0)} findings in ${results.length} files`);
