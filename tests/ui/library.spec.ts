import { expect, test, type Page } from "@playwright/test";
import { ensureResultsDir, horizontalOverflowPx, runAxe } from "./helpers";

const THEMES = ["light", "dark"] as const;
const WIDE = { width: 1280, height: 800 };
const NARROW = { width: 760, height: 600 };

test.beforeAll(() => {
  ensureResultsDir();
});

// src/ipc/mock.ts's "第1回授業スライド.mp4" sample session (36 slides) -- selecting it is
// what makes the detail pane show 36 thumbnails below.
async function selectSession(page: Page): Promise<void> {
  await page.getByRole("option", { name: /第1回授業スライド\.mp4/ }).click();
  await expect(page.locator(".library-tile")).toHaveCount(36);
}

async function checkAndScreenshot(page: Page, name: string): Promise<void> {
  expect(await horizontalOverflowPx(page)).toBeLessThanOrEqual(0);
  const results = await runAxe(page);
  expect(results.violations).toEqual([]);
  await page.screenshot({ path: `test-results/ui/library-${name}.png` });
}

for (const theme of THEMES) {
  test.describe(`library, 1280x800 (${theme})`, () => {
    test.use({ viewport: WIDE, colorScheme: theme });

    test("layout A", async ({ page }) => {
      await page.goto("/?preset=stopped#/library");
      await page.getByTestId("library-root").waitFor();
      await selectSession(page);
      await checkAndScreenshot(page, `a-${theme}`);
    });

    test("confirm dialog open", async ({ page }) => {
      await page.goto("/?preset=stopped#/library");
      await page.getByTestId("library-root").waitFor();
      await selectSession(page);
      await page.getByRole("button", { name: "削除…" }).click();
      await page.getByRole("dialog").waitFor();
      await checkAndScreenshot(page, `confirm-${theme}`);
    });

    test("select mode", async ({ page }) => {
      await page.goto("/?preset=stopped#/library");
      await page.getByTestId("library-root").waitFor();
      await selectSession(page);
      await page.getByRole("button", { name: "画像を選ぶ" }).click();
      await page.getByRole("checkbox").first().waitFor();
      await checkAndScreenshot(page, `select-${theme}`);
    });
  });

  test.describe(`library, 760x600 (${theme})`, () => {
    test.use({ viewport: NARROW, colorScheme: theme });

    test("layout A narrow", async ({ page }) => {
      await page.goto("/?preset=stopped#/library");
      await page.getByTestId("library-root").waitFor();
      await selectSession(page);
      await checkAndScreenshot(page, `a-narrow-${theme}`);
    });
  });
}
