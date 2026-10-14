import { expect, test, type Page } from "@playwright/test";
import { ensureResultsDir, horizontalOverflowPx, runAxe } from "./helpers";

// docs/impl/updates.md; the scenario comes from the mock's URL query (src/ipc/mock.ts).
const THEMES = ["light", "dark"] as const;

test.beforeAll(() => {
  ensureResultsDir();
});

async function checkAndScreenshot(page: Page, name: string): Promise<void> {
  expect(await horizontalOverflowPx(page)).toBeLessThanOrEqual(0);
  const results = await runAxe(page);
  expect(results.violations).toEqual([]);
  await page.screenshot({ path: `test-results/ui/update-${name}.png` });
}

for (const theme of THEMES) {
  test.describe(`update dialog, library 1280x800 (${theme})`, () => {
    test.use({ viewport: { width: 1280, height: 800 }, colorScheme: theme });

    test("offer", async ({ page }) => {
      await page.goto("/?preset=stopped&update=offer#/library");
      await page.getByRole("dialog", { name: "新しい版があります" }).waitFor();
      await expect(page.getByRole("button", { name: "今すぐ更新" })).toBeEnabled();
      await checkAndScreenshot(page, `offer-${theme}`);
    });

    test("busy", async ({ page }) => {
      await page.goto("/?preset=running&update=offer#/library");
      await page.getByRole("dialog", { name: "新しい版があります" }).waitFor();
      await expect(page.getByRole("button", { name: "今すぐ更新" })).toBeDisabled();
      await expect(page.getByText("記録中は更新できません。停止すると更新できます。")).toBeVisible();
      await checkAndScreenshot(page, `busy-${theme}`);
    });

    test("installing", async ({ page }) => {
      await page.goto("/?preset=stopped&update=offer#/library");
      await page.getByRole("button", { name: "今すぐ更新" }).click();
      await expect(page.getByText("ダウンロード中… 40%")).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(page.getByRole("heading", { name: "更新しています" })).toBeVisible();
      await checkAndScreenshot(page, `installing-${theme}`);
    });

    test("failed", async ({ page }) => {
      await page.goto("/?preset=stopped&update=offer&install=fail#/library");
      await page.getByRole("button", { name: "今すぐ更新" }).click();
      await expect(page.getByText("更新できませんでした：The signature does not match")).toBeVisible();
      await checkAndScreenshot(page, `failed-${theme}`);
    });
  });

  test.describe(`update in settings 560x560 (${theme})`, () => {
    test.use({ viewport: { width: 560, height: 560 }, colorScheme: theme });

    test("up to date", async ({ page }) => {
      await page.goto("/#/settings");
      await page.getByTestId("settings-root").waitFor();
      await expect(page.getByText("バージョン 0.1.6")).toBeVisible();
      await page.getByRole("button", { name: "アップデートを確認" }).click();
      await expect(page.getByText("最新の版です。")).toBeVisible();
      await checkAndScreenshot(page, `settings-uptodate-${theme}`);
    });

    test("available opens the dialog", async ({ page }) => {
      await page.goto("/?check=available#/settings");
      await page.getByTestId("settings-root").waitFor();
      await page.getByRole("button", { name: "アップデートを確認" }).click();
      await page.getByRole("dialog", { name: "新しい版があります" }).waitFor();
      await checkAndScreenshot(page, `settings-available-${theme}`);
      await page.getByRole("button", { name: "後で" }).click();
      await expect(page.getByText("0.2.0 があります。")).toBeVisible();
      await checkAndScreenshot(page, `settings-available-line-${theme}`);
    });

    test("error", async ({ page }) => {
      await page.goto("/?check=error#/settings");
      await page.getByTestId("settings-root").waitFor();
      await page.getByRole("button", { name: "アップデートを確認" }).click();
      await expect(page.getByText("確認できませんでした：ネットワークに接続できません")).toBeVisible();
      await checkAndScreenshot(page, `settings-error-${theme}`);
    });
  });
}

test.describe("update strings in English", () => {
  test.use({ viewport: { width: 560, height: 560 }, colorScheme: "light", locale: "en-US" });

  test("settings skipped, English", async ({ page }) => {
    await page.goto("/?check=skipped#/settings");
    await page.getByTestId("settings-root").waitFor();
    await page.getByRole("button", { name: "Check for updates" }).click();
    await page.getByRole("dialog", { name: "A new version is available" }).waitFor();
    await checkAndScreenshot(page, "settings-skipped-en");
    await page.getByRole("button", { name: "Later" }).click();
    await expect(page.getByText("You skipped 0.2.0.")).toBeVisible();
  });
});
