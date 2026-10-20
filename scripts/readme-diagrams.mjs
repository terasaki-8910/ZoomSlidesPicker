// Render the approved icon-only diagrams from JSON specs and installed SVG icons.
// SVGs stay editable; PNGs are compatibility exports. No network/image-generation API.
// Run: node scripts/readme-diagrams.mjs
/* global document, XMLSerializer */
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import * as si from "react-icons/si";
import * as tb from "react-icons/tb";
import { chromium } from "@playwright/test";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const NAMES = ["architecture", "stack", "detection-flow"];
const escape = (value) => String(value).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll('"', "&quot;");

function component(node) {
  if (node.kind === "decision") {
    return `<g class="component decision" data-node="${node.id}"><path d="M${node.x} ${node.y}l130 55-130 55-130-55Z"/><text text-anchor="middle" x="${node.x}" y="${node.y + (node.sub?.length ? 46 : 61)}">${escape(node.title)}</text>${(node.sub ?? []).map((line, i) => `<text class="decision-sub" text-anchor="middle" x="${node.x}" y="${node.y + 72 + i * 18}">${escape(line)}</text>`).join("")}</g>`;
  }
  const [set, key] = node.icon.split(":");
  const Icon = { si, tb }[set]?.[key];
  if (!Icon) throw new Error(`Unknown icon ${node.icon}`);
  const glyph = renderToStaticMarkup(React.createElement(Icon, { size: 56 }));
  return `<g class="component" data-node="${node.id}"><g class="icon-node" data-icon="${node.icon}" transform="translate(${node.x - 28},${node.y + 4})" style="color:var(--${node.c})">${glyph}</g><text class="label" text-anchor="middle" x="${node.x}" y="${node.y + 90}">${escape(node.title)}</text>${(node.sub ?? []).map((line, i) => `<text class="sub" text-anchor="middle" x="${node.x}" y="${node.y + 113 + i * 21}">${escape(line)}</text>`).join("")}</g>`;
}

function svgFor(spec, palette) {
  const ids = spec.nodes.concat(spec.groups).map((item) => item.id);
  if (new Set(ids).size !== ids.length) throw new Error(`Duplicate node/group ID in ${spec.title}`);
  for (const edge of spec.edges) {
    if (!ids.includes(edge.from) || !ids.includes(edge.to)) throw new Error(`Unknown connector: ${edge.from} → ${edge.to}`);
  }
  const groups = spec.groups.map((group) => `<g class="group" data-group="${group.id}"><rect x="${group.x}" y="${group.y}" width="${group.w}" height="${group.h}"/><path class="group-mark" style="stroke:var(--${group.c})" d="M${group.x + 1.5} ${group.y}v32"/><text class="group-title" x="${group.x + 18}" y="${group.y + 24}">${escape(group.title)}</text></g>`).join("");
  let number = 0;
  const edges = spec.edges.map((edge) => {
    const label = edge.label ? `${++number}. ${edge.label}` : "";
    // The browser refines the label backing rectangles with actual font metrics.
    const width = [...label].reduce((sum, char) => sum + (/[\u3000-\u9fff\uff00-\uffef]/.test(char) ? 15 : 8), 16);
    return `<g class="edge" data-from="${edge.from}" data-to="${edge.to}"><path class="${edge.dashed ? "control" : "data"}" d="${edge.d}" marker-end="url(#arrow)" ${edge.both ? 'marker-start="url(#arrow)"' : ""}/>${label ? `<g class="edge-label" transform="translate(${edge.lx},${edge.ly})"><rect x="${-width / 2}" y="-13" width="${width}" height="26"/><text text-anchor="middle">${escape(label)}</text></g>` : ""}</g>`;
  }).join("");
  const colors = Object.entries(palette).map(([key, value]) => `--${key}:${value}`).join(";");
  const description = `${spec.nodes.map((node) => node.title).join("、")}。${spec.edges.filter((edge) => edge.label).map((edge) => edge.label).join("、")}。`;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${spec.width}" height="${spec.height}" viewBox="0 0 ${spec.width} ${spec.height}" role="img" aria-labelledby="title desc" style="${colors}">
<title id="title">${escape(spec.title)}</title><desc id="desc">${escape(description)}</desc>
<style>
svg[aria-labelledby]{font-family:-apple-system,BlinkMacSystemFont,'Hiragino Sans','Yu Gothic UI',sans-serif;color:var(--text)}
.background{fill:var(--bg)}text{fill:var(--text);font-size:17px}.heading{font-size:26px;font-weight:650}.caption{font-size:16px;fill:var(--muted)}
.group>rect{fill:none;stroke:var(--border);stroke-width:1.3}.group-mark{fill:none;stroke-width:3}.group-title{font-size:18px;font-weight:600}
.label{font-size:19px;font-weight:600}.sub{font-size:15px;fill:var(--muted)}.decision>path{fill:var(--bg);stroke:var(--border);stroke-width:1.5}.decision>text{font-size:19px;font-weight:600}.decision>.decision-sub{font-size:13px;font-weight:400;fill:var(--muted)}
.edge>path{fill:none;stroke:var(--line);stroke-width:1.7}.edge>.control{stroke-dasharray:7 5}.edge-label rect{fill:var(--bg)}.edge-label text{font-size:15px;dominant-baseline:middle}.note{font-size:15px;fill:var(--muted)}
</style>
<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M1 1L9 5L1 9" fill="none" stroke="var(--line)" stroke-width="1.5"/></marker></defs>
<rect class="background" width="${spec.width}" height="${spec.height}"/><text class="heading" x="40" y="44">${escape(spec.title)}</text><text class="caption" x="40" y="77">${escape(spec.caption)}</text>
${groups}${edges}${spec.nodes.map(component).join("")}${spec.notes.map((note) => `<text class="note" x="${note.x}" y="${note.y}">${escape(note.text)}</text>`).join("")}
</svg>\n`;
}

async function verify(page, spec) {
  return page.evaluate(({ nodeCount, iconCount, edgeCount }) => {
    const svg = document.documentElement;
    const canvas = svg.getBoundingClientRect();
    const texts = [...svg.querySelectorAll("text")].map((element) => ({
      text: element.textContent, box: element.getBoundingClientRect(),
    }));
    const outside = texts.filter(({ box }) => box.left < canvas.left || box.top < canvas.top || box.right > canvas.right || box.bottom > canvas.bottom);
    if (outside.length) throw new Error(`Text outside canvas: ${outside.map((item) => item.text)}`);
    const overlap = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 1 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 1;
    for (let i = 0; i < texts.length; i++) for (let j = i + 1; j < texts.length; j++) {
      if (overlap(texts[i].box, texts[j].box)) throw new Error(`Overlapping labels: ${texts[i].text} / ${texts[j].text}`);
    }
    for (const icon of svg.querySelectorAll(".icon-node")) {
      const box = icon.getBoundingClientRect();
      const hits = texts.filter((item) => overlap(box, item.box));
      if (hits.length) throw new Error(`Text overlaps icon ${icon.dataset.icon}: ${hits.map((item) => item.text)}`);
    }
    if (svg.querySelectorAll(".component").length !== nodeCount || svg.querySelectorAll(".icon-node svg").length !== iconCount || svg.querySelectorAll(".edge").length !== edgeCount) {
      throw new Error("Missing component, SVG icon or connector");
    }
    return { components: nodeCount, icons: iconCount, connectors: edgeCount, outsideCanvas: 0, overlappingLabels: 0 };
  }, { nodeCount: spec.nodes.length, iconCount: spec.nodes.filter((node) => node.icon).length, edgeCount: spec.edges.length });
}

export async function renderDiagrams({ browser, root = ROOT }) {
  const source = resolve(root, "docs/diagrams");
  const out = resolve(root, "docs/images");
  mkdirSync(out, { recursive: true });
  const palette = JSON.parse(readFileSync(resolve(source, "tokens.json"), "utf8"));
  const reports = [];
  for (const name of NAMES) {
    const spec = JSON.parse(readFileSync(resolve(source, `${name}.json`), "utf8"));
    for (const theme of ["light", "dark"]) {
      const file = resolve(out, `diagram-${name}-${theme}.svg`);
      writeFileSync(file, svgFor(spec, palette[theme]));
      const page = await browser.newPage({ viewport: { width: spec.width, height: spec.height }, deviceScaleFactor: 1.5 });
      try {
        await page.goto(pathToFileURL(file).href);
        await page.evaluate(async () => {
          await document.fonts.ready;
          for (const label of document.querySelectorAll(".edge-label")) {
            const box = label.querySelector("text").getBBox();
            const rect = label.querySelector("rect");
            rect.setAttribute("x", String(box.x - 8));
            rect.setAttribute("y", String(box.y - 4));
            rect.setAttribute("width", String(box.width + 16));
            rect.setAttribute("height", String(box.height + 8));
          }
        });
        reports.push({ name, theme, ...await verify(page, spec) });
        const serialized = await page.locator("svg").first().evaluate((element) => new XMLSerializer().serializeToString(element));
        writeFileSync(file, serialized + "\n");
        await page.locator("svg").first().screenshot({ path: file.replace(/\.svg$/, ".png") });
        console.log(`diagram ${name} ${theme}: SVG + PNG, geometry passed`);
      } finally { await page.close(); }
    }
  }
  return reports;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const browser = await chromium.launch(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {});
  try { await renderDiagrams({ browser }); } finally { await browser.close(); }
}
