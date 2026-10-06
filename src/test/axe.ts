import axe from "axe-core";

/**
 * Runs axe-core directly against a jsdom container (no vitest-axe/jest-axe dependency --
 * both are effectively unmaintained). Rules axe cannot evaluate without real layout
 * (notably parts of colour-contrast) come back as "incomplete", not "violations", so this
 * still catches real structural issues (missing labels/roles/alt text) under jsdom; the
 * full contrast check runs for real in tests/ui/*.spec.ts (scripts/ui-check.sh).
 */
export async function expectNoAxeViolations(container: Element): Promise<void> {
  const results = await axe.run(container, {
    runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21aa"] },
  });
  if (results.violations.length > 0) {
    const details = results.violations
      .map(
        (v) =>
          `${v.id} (${v.impact}): ${v.description}\n  ${v.nodes.map((n) => n.target.join(" ")).join("\n  ")}`
      )
      .join("\n\n");
    throw new Error(`axe found ${results.violations.length} violation(s):\n\n${details}`);
  }
}
