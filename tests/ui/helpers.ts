import { mkdirSync } from "node:fs";
import AxeBuilder from "@axe-core/playwright";
import type { Page } from "@playwright/test";
import type { AxeResults } from "axe-core";

export function ensureResultsDir(): void {
  mkdirSync("test-results/ui", { recursive: true });
}

/** <= 0 means no horizontal scroll (docs/impl/frontend.md > Checks). */
export async function horizontalOverflowPx(page: Page): Promise<number> {
  return page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
}

// Explicit return type: @axe-core/playwright bundles its own axe-core, so TS cannot
// portably name the inferred type of a bare `analyze()` call (TS2742).
export function runAxe(page: Page): Promise<AxeResults> {
  return new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze();
}
