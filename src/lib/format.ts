import type { Lang } from "../ipc/types";

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

// ---- Library (history) window date/duration formatting --------------------
// docs/impl/library.md > Data / Wording. A session's startedAt/endedAt are RFC 3339
// timestamps carrying their own offset (this app records in the machine's local time,
// Asia/Tokyo for the mock data below) -- these helpers read the year/month/day/hour/
// minute written in the string itself, rather than `new Date(iso).getHours()` etc.,
// which would silently re-render the timestamp in whatever timezone the code happens to
// run under (wrong: a lecture recorded at 12:42 JST should always display as 12:42,
// including when the test runner or a future user's machine is not on JST).

const WEEKDAY_JA = ["日", "月", "火", "水", "木", "金", "土"];
const WEEKDAY_EN = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTH_EN = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

interface WallClock {
  y: number;
  mo: number;
  d: number;
  h: number;
  mi: number;
}

function parseRfc3339(iso: string): WallClock {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(iso);
  if (!m) throw new Error(`not an RFC 3339 timestamp: ${iso}`);
  return { y: Number(m[1]), mo: Number(m[2]), d: Number(m[3]), h: Number(m[4]), mi: Number(m[5]) };
}

/** 0 (Sunday) - 6 (Saturday) for a calendar date, independent of any timezone. */
function weekdayIndex(y: number, mo: number, d: number): number {
  return new Date(Date.UTC(y, mo - 1, d)).getUTCDay();
}

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

/** "10月7日（水）12:42" / "Oct 7 (Wed) 12:42" (library.md > Wording > Dates). */
export function formatSessionStart(iso: string, lang: Lang): string {
  const { y, mo, d, h, mi } = parseRfc3339(iso);
  const time = `${h}:${pad2(mi)}`;
  const weekday = WEEKDAY_JA[weekdayIndex(y, mo, d)];
  return lang === "ja" ? `${mo}月${d}日（${weekday}）${time}` : `${MONTH_EN[mo - 1]} ${d} (${WEEKDAY_EN[weekdayIndex(y, mo, d)]}) ${time}`;
}

/**
 * Same, plus the end time as a range ("12:42–13:15"); falls back to the start-only form
 * while `endedAt` is null (the session is still recording).
 */
export function formatSessionRange(startIso: string, endIso: string | null, lang: Lang): string {
  const start = formatSessionStart(startIso, lang);
  if (!endIso) return start;
  const { h, mi } = parseRfc3339(endIso);
  return `${start}–${h}:${pad2(mi)}`;
}

/** "33 分" / "33 min", or "1 時間 30 分" / "1 h 30 min" at an hour or more. */
export function formatDuration(totalSeconds: number, lang: Lang): string {
  const totalMin = Math.max(0, Math.round(totalSeconds / 60));
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  if (h === 0) return lang === "ja" ? `${m} 分` : `${m} min`;
  if (m === 0) return lang === "ja" ? `${h} 時間` : `${h} h`;
  return lang === "ja" ? `${h} 時間 ${m} 分` : `${h} h ${m} min`;
}

/**
 * The three forms the library search also matches a session's date against
 * (docs/impl/library.md > Search): "10/7", "10月7日", "2026-10-07".
 */
export function sessionDateSearchForms(iso: string): string[] {
  const { y, mo, d } = parseRfc3339(iso);
  return [`${mo}/${d}`, `${mo}月${d}日`, `${String(y).padStart(4, "0")}-${pad2(mo)}-${pad2(d)}`];
}
