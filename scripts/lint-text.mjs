#!/usr/bin/env node
// Text-content checks that a syntax-aware linter would not catch:
//   (a) no emoji in UI/source directories (src/, crates/*/src/, src-tauri/src/, tests/)
//   (b) no raw colour literals in src/ outside src/styles/tokens.css
// See docs/impl/frontend.md > Checks and design_brief.md > しないこと.
import { readFileSync, readdirSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(import.meta.url), "..", "..");

const EMOJI_RE = /\p{Extended_Pictographic}/u;
// Longest alternatives first so e.g. an 8-digit hex is not mistaken for a 6-digit one.
const COLOR_RE =
  /#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{4}|[0-9a-fA-F]{3})(?![0-9a-fA-F])|\brgb\(|\brgba\(|\bhsl\(|\bhsla\(/;

const EMOJI_DIRS = ["src", "crates", join("src-tauri", "src"), "tests"];
const COLOR_DIR = "src";
const COLOR_EXEMPT = join("src", "styles", "tokens.css");

const TEXT_EXT = new Set([".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".css", ".rs", ".html", ".json", ".md"]);

function isTextFile(path) {
  const dot = path.lastIndexOf(".");
  return dot !== -1 && TEXT_EXT.has(path.slice(dot));
}

function listFiles(dir) {
  const out = [];
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return out; // directory does not exist (yet) -- nothing to check there
  }
  for (const entry of entries) {
    if (entry.name === "node_modules" || entry.name === "target" || entry.name.startsWith(".")) continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listFiles(full));
    else out.push(full);
  }
  return out;
}

function reportMatches(file, re, label, violations) {
  const content = readFileSync(file, "utf8");
  const lines = content.split("\n");
  lines.forEach((line, i) => {
    if (re.test(line)) {
      violations.push(`${relative(root, file)}:${i + 1}: ${label}: ${line.trim()}`);
    }
  });
}

const violations = [];

for (const dir of EMOJI_DIRS) {
  for (const file of listFiles(join(root, dir))) {
    if (!isTextFile(file)) continue;
    reportMatches(file, EMOJI_RE, "emoji found", violations);
  }
}

for (const file of listFiles(join(root, COLOR_DIR))) {
  if (!isTextFile(file)) continue;
  const rel = relative(root, file);
  if (rel === COLOR_EXEMPT || rel.startsWith(`src${sep}styles${sep}tokens.css`)) continue;
  reportMatches(file, COLOR_RE, "colour literal outside tokens.css", violations);
}

if (violations.length > 0) {
  console.error("lint-text: violations found\n");
  for (const v of violations) console.error(v);
  console.error(`\n${violations.length} violation(s)`);
  process.exit(1);
}

console.log("lint-text: ok");
