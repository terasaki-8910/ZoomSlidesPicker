import { expect, test } from "@playwright/test";
import { ensureResultsDir, horizontalOverflowPx, runAxe } from "./helpers";

// Settings window size (docs/impl/ipc.md > Windows).
const VIEWPORT = { width: 560, height: 560 };
const THEMES = ["light", "dark"] as const;

test.beforeAll(() => {
  ensureResultsDir();
});

for (const theme of THEMES) {
  test.describe(`settings (${theme})`, () => {
    test.use({ viewport: VIEWPORT, colorScheme: theme });

    test("settings", async ({ page }) => {
      await page.goto("/#/settings");
      await page.getByTestId("settings-root").waitFor();

      expect(await horizontalOverflowPx(page)).toBeLessThanOrEqual(0);

      // A long save path is cut off; the button next to it stays one line, as tall as
      // the shortcut rows' buttons.
      const pathButton = await page.getByRole("button", { name: "変更…", exact: true }).boundingBox();
      const shortcutButton = await page.getByRole("button", { name: "変更", exact: true }).first().boundingBox();
      expect(pathButton && shortcutButton && pathButton.height <= shortcutButton.height + 1).toBe(true);

      const results = await runAxe(page);
      expect(results.violations).toEqual([]);

      await page.screenshot({ path: `test-results/ui/settings-${theme}.png` });
    });
  });
}

test.describe("settings (en, light smoke)", () => {
  test.use({ viewport: VIEWPORT, colorScheme: "light", locale: "en-US" });

  test("settings-en", async ({ page }) => {
    await page.goto("/#/settings");
    await page.getByTestId("settings-root").waitFor();
    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();

    const results = await runAxe(page);
    expect(results.violations).toEqual([]);

    await page.screenshot({ path: "test-results/ui/settings-en-light.png" });
  });
});
