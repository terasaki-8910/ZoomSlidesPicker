import { expect, test } from "@playwright/test";
import { ensureResultsDir, horizontalOverflowPx, runAxe } from "./helpers";

// Region window size (docs/impl/ipc.md > Windows).
const VIEWPORT = { width: 1100, height: 760 };
const THEMES = ["light", "dark"] as const;

test.beforeAll(() => {
  ensureResultsDir();
});

for (const theme of THEMES) {
  test.describe(`region (${theme})`, () => {
    test.use({ viewport: VIEWPORT, colorScheme: theme });

    test("region", async ({ page }) => {
      await page.goto("/#/region");
      await page.getByTestId("region-root").waitFor();
      await page.locator(".region-rect").waitFor();

      expect(await horizontalOverflowPx(page)).toBeLessThanOrEqual(0);

      const results = await runAxe(page);
      expect(results.violations).toEqual([]);

      await page.screenshot({ path: `test-results/ui/region-${theme}.png` });
    });
  });
}

test.describe("region (en, light smoke)", () => {
  test.use({ viewport: VIEWPORT, colorScheme: "light", locale: "en-US" });

  test("region-en", async ({ page }) => {
    await page.goto("/#/region");
    await page.getByTestId("region-root").waitFor();
    await expect(page.getByRole("button", { name: "Start" })).toBeVisible();

    const results = await runAxe(page);
    expect(results.violations).toEqual([]);

    await page.screenshot({ path: "test-results/ui/region-en-light.png" });
  });
});
