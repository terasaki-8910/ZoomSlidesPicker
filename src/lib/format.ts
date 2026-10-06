/** "h:mm:ss" -- pair with the "tabular-nums" class (design_brief.md: 等幅の数字). */
export function formatElapsed(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  return `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
}

/** Rounds a fractional RegionSel dimension to whole pixels of the captured window. */
export function roundPx(fraction: number, totalPx: number): number {
  return Math.round(fraction * totalPx);
}
