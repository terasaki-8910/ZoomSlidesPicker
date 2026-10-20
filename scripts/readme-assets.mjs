// Capture the real React views with the existing, invented IPC mock; render the
// README diagrams from editable JSON/SVG and installed React Icons. No lecture data.
// Run: node scripts/readme-assets.mjs [screenshots|diagrams]
/* global document, innerWidth */
import { spawn, execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium, expect } from "@playwright/test";
import { renderDiagrams } from "./readme-diagrams.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "docs/images");
const selected = process.argv.slice(2);
if (selected.some((part) => !["screenshots", "diagrams"].includes(part))) {
  throw new Error("Usage: node scripts/readme-assets.mjs [screenshots|diagrams]");
}
mkdirSync(out, { recursive: true });
const browser = await chromium.launch(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {});

async function assertFits(page) {
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
  if (overflow > 0) throw new Error(`Horizontal overflow: ${overflow}`);
  await page.evaluate(async () => {
    await document.fonts.ready;
    await Promise.all([...document.images].map(async (image) => {
      await image.decode();
      if (!image.naturalWidth) throw new Error(`Image failed: ${image.src}`);
    }));
  });
}

async function startMock() {
  const port = 4391;
  const child = spawn("pnpm", ["exec", "vite", "--host", "127.0.0.1", "--port", String(port), "--strictPort"], {
    cwd: root, env: { ...process.env, VITE_IPC_MOCK: "1" }, stdio: "pipe",
  });
  let log = "";
  child.stdout.on("data", (data) => { log += data; });
  child.stderr.on("data", (data) => { log += data; });
  child.on("error", (error) => { log += error.message; });
  const url = `http://127.0.0.1:${port}`;
  for (let i = 0; i < 80; i++) {
    if (child.exitCode !== null) throw new Error(`Mock server failed: ${log}`);
    try {
      if ((await fetch(url)).ok) {
        const servedByChild = log.includes(`:${port}/`);
        if (!servedByChild) throw new Error(`Port ${port} already in use`);
        return { url, stop: () => child.kill() };
      }
    } catch (error) {
      if (error.message.includes("already in use")) { child.kill(); throw error; }
    }
    await new Promise((done) => setTimeout(done, 250));
  }
  child.kill();
  throw new Error(`Mock server timeout: ${log}`);
}

async function screenshots() {
  const server = await startMock();
  const frames = mkdtempSync(join(tmpdir(), "zsp-readme-"));
  try {
    for (const theme of ["light", "dark"]) {
      const context = await browser.newContext({ locale: "ja-JP", colorScheme: theme, viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 });
      const page = await context.newPage();
      // Match the mock's fixed "now" so the active sample never ages into a
      // multi-day lecture in regenerated screenshots. Timers still run normally.
      await page.clock.setFixedTime(new Date("2026-10-07T06:00:00Z"));
      page.on("pageerror", (error) => { throw error; });
      let scene = 0;
      async function open(view, preset = "stopped") {
        // The app uses one fixed view per native window; changing only the hash
        // does not remount App. A fresh query forces a real document navigation.
        await page.goto(`${server.url}/?preset=${preset}&scene=${++scene}#/${view}`);
        await page.getByTestId(`${view}-root`).waitFor();
      }
      async function shot(name, locator) {
        await assertFits(page);
        await (locator ?? page).screenshot({ path: join(out, `${name}-${theme}.png`), animations: "disabled" });
      }
      await open("library");
      const session = page.getByRole("option", { name: /第1回授業スライド\.mp4/ });
      await session.click();
      await expect(page.locator(".library-tile")).toHaveCount(36);
      await shot("hero");
      await page.getByRole("button", { name: "画像を選ぶ" }).click();
      await page.getByRole("checkbox").nth(2).click();
      await page.getByRole("checkbox").nth(7).click({ modifiers: ["Shift"] });
      await expect(page.locator(".library-tile-check:checked")).toHaveCount(6);
      await shot("feature-selection");
      await open("library");
      await session.click();
      await page.getByRole("button", { name: "PDF を書き出す…" }).click();
      await expect(page.getByText("約 50 MB")).toBeVisible();
      await shot("feature-export", page.getByRole("dialog"));
      await page.getByRole("button", { name: "やめる" }).click();
      await session.click({ button: "right" });
      await shot("feature-menu", page.getByRole("menu"));
      await page.setViewportSize({ width: 1100, height: 760 });
      await open("region");
      await page.locator(".region-frame").waitFor();
      const box = await page.locator(".region-frame").boundingBox();
      await page.mouse.move(box.x + box.width * .08, box.y + box.height * .09);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width * .92, box.y + box.height * .88, { steps: 15 });
      await page.mouse.up();
      await shot("feature-region");
      await page.setViewportSize({ width: 640, height: 720 });
      await open("settings");
      await expect(page.getByRole("button", { name: "アップデートを確認" })).toBeVisible();
      await shot("feature-settings");
      await page.setViewportSize({ width: 320, height: 480 });
      await open("panel", "running");
      await expect(page.getByRole("button", { name: "今すぐ保存" })).toBeVisible();
      await expect.poll(() => page.evaluate(() => Number(document.documentElement.dataset.fitPanel))).toBeGreaterThan(0);
      const height = await page.evaluate(() => Number(document.documentElement.dataset.fitPanel));
      await page.setViewportSize({ width: 320, height });
      await shot("feature-running");
      await context.close();
      console.log(`screenshots ${theme}: ok`);
    }
    // Honest UI demo: mocked save/pause/resume/stop commands, not a detector benchmark.
    const context = await browser.newContext({ locale: "ja-JP", colorScheme: "light", viewport: { width: 320, height: 480 } });
    const page = await context.newPage();
    await page.clock.setFixedTime(new Date("2026-10-07T06:00:00Z"));
    await page.goto(`${server.url}/?preset=running#/panel`);
    await expect(page.getByRole("button", { name: "今すぐ保存" })).toBeVisible();
    const sequence = [null, "今すぐ保存", "一時停止", "再開", "停止"];
    let index = 0;
    for (const action of sequence) {
      if (action) await page.getByRole("button", { name: action, exact: true }).click();
      if (action === "停止") await expect(page.getByRole("button", { name: "開始", exact: true })).toBeVisible();
      await assertFits(page);
      await page.screenshot({ path: join(frames, `${String(index++).padStart(2, "0")}.png`) });
    }
    await context.close();
    execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-y", "-framerate", "0.5", "-i", join(frames, "%02d.png"), "-vf", "split[a][b];[a]palettegen[p];[b][p]paletteuse", "-loop", "0", join(out, "demo.gif")]);
    console.log(`demo.gif: ${readdirSync(frames).length} scenes, ok`);
  } finally {
    server.stop();
    rmSync(frames, { recursive: true, force: true });
  }
}

try {
  if (!selected.length || selected.includes("diagrams")) await renderDiagrams({ browser, root });
  if (!selected.length || selected.includes("screenshots")) await screenshots();
} finally {
  await browser.close();
}
