import { expect, test } from "@playwright/test";
import { ensureResultsDir, horizontalOverflowPx, runAxe } from "./helpers";

// Panel window size (docs/impl/ipc.md > Windows).
const VIEWPORT = { width: 320, height: 480 };
const THEMES = ["light", "dark"] as const;

const SCENARIOS: { name: string; preset: string }[] = [
  { name: "panel-stopped", preset: "stopped" },
  { name: "panel-stopped-last", preset: "stopped-last" },
  { name: "panel-running", preset: "running" },
  { name: "panel-no-permission", preset: "no-permission" },
];

test.beforeAll(() => {
  ensureResultsDir();
});

for (const theme of THEMES) {
  test.describe(`panel (${theme})`, () => {
    test.use({ viewport: VIEWPORT, colorScheme: theme });

    for (const scenario of SCENARIOS) {
      test(scenario.name, async ({ page }) => {
        await page.goto(`/?preset=${scenario.preset}#/panel`);
        await page.getByTestId("panel-root").waitFor();

        expect(await horizontalOverflowPx(page)).toBeLessThanOrEqual(0);

        const results = await runAxe(page);
        expect(results.violations).toEqual([]);

        await page.screenshot({ path: `test-results/ui/${scenario.name}-${theme}.png` });
      });
    }

    test("panel-picker", async ({ page }) => {
      await page.goto("/?preset=stopped#/panel");
      await page.getByRole("button", { name: "開始" }).click();
      await page.getByRole("listbox").waitFor();

      expect(await horizontalOverflowPx(page)).toBeLessThanOrEqual(0);

      const results = await runAxe(page);
      expect(results.violations).toEqual([]);

      await page.screenshot({ path: `test-results/ui/panel-picker-${theme}.png` });
    });
  });
}

// fit_panel: the panel reports its content height, not the window's, and that height
// fits the content without scrolling.
test.describe("panel fit", () => {
  test.use({ viewport: VIEWPORT, colorScheme: "light" });

  test("panel-fit", async ({ page }) => {
    const reported = () => page.evaluate(() => Number(document.documentElement.dataset.fitPanel ?? "NaN"));
    const bodyOverflow = () =>
      page.evaluate(() => {
        const body = document.querySelector(".panel-body");
        return body ? body.scrollHeight - body.clientHeight : Number.NaN;
      });

    // What the panel should report for the content as rendered right now.
    const natural = () =>
      page.evaluate(() => {
        const header = document.querySelector(".panel-header");
        const content = document.querySelector(".panel-content");
        if (!header || !content) return Number.NaN;
        return Math.ceil(header.getBoundingClientRect().height + content.getBoundingClientRect().height);
      });

    await page.goto("/?preset=stopped#/panel");
    await expect(page.getByRole("button", { name: "開始" })).toBeVisible();
    // The app state arrives after the first render, so wait for the report to catch up.
    await expect.poll(async () => (await reported()) - (await natural())).toBe(0);
    const stopped = await reported();
    expect(stopped).toBeLessThan(VIEWPORT.height);

    await page.setViewportSize({ width: VIEWPORT.width, height: stopped });
    expect(await bodyOverflow()).toBeLessThanOrEqual(0);

    await page.setViewportSize(VIEWPORT);
    await page.getByRole("button", { name: "開始" }).click();
    await page.getByRole("listbox").waitFor();
    await expect.poll(reported).toBeGreaterThan(stopped);
  });
});

// One English-locale smoke check per view type (see also region.spec.ts, settings.spec.ts)
// so the English strings get at least one real-browser + axe pass too, without doubling
// every scenario above.
test.describe("panel (en, light smoke)", () => {
  test.use({ viewport: VIEWPORT, colorScheme: "light", locale: "en-US" });

  test("panel-stopped-en", async ({ page }) => {
    await page.goto("/?preset=stopped#/panel");
    await page.getByTestId("panel-root").waitFor();
    await expect(page.getByRole("button", { name: "Start" })).toBeVisible();

    const results = await runAxe(page);
    expect(results.violations).toEqual([]);

    await page.screenshot({ path: "test-results/ui/panel-stopped-en-light.png" });
  });
});
