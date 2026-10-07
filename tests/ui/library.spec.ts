import { expect, test, type Page } from "@playwright/test";
import { ensureResultsDir, horizontalOverflowPx, runAxe } from "./helpers";

const THEMES = ["light", "dark"] as const;
const WIDE = { width: 1280, height: 800 };
const NARROW = { width: 760, height: 600 };

test.beforeAll(() => {
  ensureResultsDir();
});

// docs/impl/library.md's mock session #2 ("第1回授業スライド.mp4", 36 slides) -- selecting
// it is what makes the detail pane show 36 thumbnails for the "a"/"a-narrow" screenshots.
async function selectSecondSession(page: Page): Promise<void> {
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
  test.describe(`library layout A, 1280x800 (${theme})`, () => {
    test.use({ viewport: WIDE, colorScheme: theme });

    test("a", async ({ page }) => {
      await page.goto("/?preset=stopped#/library");
      await page.getByTestId("library-root").waitFor();
      await selectSecondSession(page);
      await checkAndScreenshot(page, `a-${theme}`);
    });
  });

  test.describe(`library layout A, 760x600 (${theme})`, () => {
    test.use({ viewport: NARROW, colorScheme: theme });

    test("a-narrow", async ({ page }) => {
      await page.goto("/?preset=stopped#/library");
      await page.getByTestId("library-root").waitFor();
      await selectSecondSession(page);
      await checkAndScreenshot(page, `a-narrow-${theme}`);
    });
  });

  test.describe(`library layout B, 1280x800 (${theme})`, () => {
    test.use({ viewport: WIDE, colorScheme: theme });

    test("b", async ({ page }) => {
      await page.goto("/?preset=stopped&layout=list#/library");
      await page.getByTestId("library-root").waitFor();
      await page.getByRole("button", { name: "開く" }).first().waitFor();
      await checkAndScreenshot(page, `b-${theme}`);
    });

    test("b-open", async ({ page }) => {
      await page.goto("/?preset=stopped&layout=list#/library");
      await page.getByTestId("library-root").waitFor();
      await page.getByRole("button", { name: "開く" }).first().click();
      await page.getByRole("button", { name: "戻る" }).waitFor();
      await checkAndScreenshot(page, `b-open-${theme}`);
    });
  });
}
