// In-memory implementation of docs/impl/ipc.md, used when VITE_IPC_MOCK=1 (pnpm dev,
// pnpm build:mock / preview:mock, and scripts/ui-check.sh). Keeps state and emits
// app-state/settings to listeners so flows work end to end in a plain browser: start a
// session, pause, save now, undo, stop, and so on.
//
// All sample data below is invented for this mock -- window titles, thumbnails and the
// snapshot image are generated SVGs with neutral shapes and placeholder labels, never
// real screen content.
// Leaf dictionaries only (no runtime imports of their own) -- see the comment above
// libT() below for why this mock reuses them instead of duplicating their text.
import { en } from "../i18n/en";
import { ja } from "../i18n/ja";
import { isMac } from "../lib/platform";
import type {
  AppState,
  CaptureView,
  Lang,
  LangSetting,
  PdfEstimate,
  PdfExport,
  PdfQuality,
  PermissionView,
  RegionSel,
  SessionDetail,
  SessionState,
  SessionSummary,
  SessionView,
  Settings,
  SettingsView,
  Shortcuts,
  Snapshot,
  UpdateAnswer,
  UpdateCheck,
  UpdateInfo,
  UpdateProgress,
  WindowEntry,
} from "./types";

type UnlistenFn = () => void;

// ---- invented sample artwork (SVG, neutral greys only) --------------------

function escapeXml(s: string): string {
  const map: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&apos;" };
  return s.replace(/[&<>"']/g, (c) => map[c] ?? c);
}

function svgDataUrl(svg: string): string {
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/** A generic "slide" thumbnail: a title bar and a few body bars, never real content. */
function slideSvg(width: number, height: number, label: string, barWidths: number[]): string {
  const pad = Math.round(width * 0.08);
  const barHeight = Math.max(4, Math.round(height * 0.07));
  const bars = barWidths
    .map((w, i) => {
      const y = pad + Math.round(height * 0.22) + i * (barHeight + Math.round(height * 0.06));
      return `<rect x="${pad}" y="${y}" width="${Math.round((width - pad * 2) * w)}" height="${barHeight}" rx="${Math.round(barHeight / 3)}" fill="lightgray" />`;
    })
    .join("");
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
    <rect width="${width}" height="${height}" fill="white" />
    <rect x="${pad}" y="${pad}" width="${width - pad * 2}" height="${Math.round(height * 0.12)}" fill="gainsboro" />
    ${bars}
    <text x="${pad}" y="${height - Math.round(pad * 0.6)}" font-family="sans-serif" font-size="${Math.max(9, Math.round(height * 0.085))}" fill="dimgray">${escapeXml(label)}</text>
  </svg>`;
  return svgDataUrl(svg);
}

function windowThumb(label: string, variant: number): string {
  return slideSvg(240, 135, label, [0.6 - variant * 0.05, 0.75, 0.4 + variant * 0.1]);
}

function snapshotImage(width: number, height: number, label: string): string {
  return slideSvg(width, height, label, [0.5, 0.68, 0.3, 0.58]);
}

function captureThumb(count: number): string {
  return slideSvg(280, 158, `Lecture video (slide ${count})`, [0.55, 0.72, 0.35, 0.6, 0.42]);
}

/**
 * A varied "slide" thumbnail for the library (history) window's mock sessions: a white
 * background, a title text line, three to five grey body bars, and -- on roughly a
 * third of slides, for visual variety -- an outlined box standing in for an embedded
 * image or chart (docs/impl/library.md > Data). Colours here (white/dimgray/lightgray/
 * darkgray) describe this SVG's own fixed, light pixel content, not a themed UI
 * surface, so design_brief.md's "colours only via tokens" rule does not apply to them,
 * same as slideSvg() above; scripts/lint-text.mjs only forbids colour *literals*
 * (hex/rgb/hsl), not named CSS colours. Deterministic in `seed` (no Math.random()), so
 * output -- and therefore every screenshot -- is stable across renders and test runs.
 */
function librarySlideSvg(seed: number, titleText: string, width = 320, height = 180): string {
  const pad = Math.round(width * 0.08);
  const titleY = pad + Math.round(height * 0.14);
  const barCount = 3 + (seed % 3); // 3-5
  const barHeight = Math.max(4, Math.round(height * 0.06));
  const barGap = Math.round(height * 0.055);
  const bodyTop = titleY + Math.round(height * 0.12);
  const hasBox = seed % 3 === 1; // "sometimes a box"
  const boxW = Math.round(width * 0.3);
  const boxH = Math.round(height * 0.34);
  const barAreaWidth = hasBox ? width - pad * 2 - boxW - Math.round(width * 0.04) : width - pad * 2;

  const bars = Array.from({ length: barCount }, (_, i) => {
    const frac = 0.5 + (((seed + i) * 7) % 5) * 0.09; // cosmetic per-bar width variation
    const y = bodyTop + i * (barHeight + barGap);
    return `<rect x="${pad}" y="${y}" width="${Math.round(barAreaWidth * Math.min(1, frac))}" height="${barHeight}" rx="${Math.round(barHeight / 3)}" fill="lightgray" />`;
  }).join("");

  const box = hasBox
    ? `<rect x="${width - pad - boxW}" y="${bodyTop}" width="${boxW}" height="${boxH}" fill="none" stroke="darkgray" stroke-width="2" />`
    : "";

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
    <rect width="${width}" height="${height}" fill="white" />
    <text x="${pad}" y="${titleY}" font-family="sans-serif" font-size="${Math.max(11, Math.round(height * 0.1))}" font-weight="bold" fill="dimgray">${escapeXml(titleText)}</text>
    ${bars}
    ${box}
  </svg>`;
  return svgDataUrl(svg);
}

// ---- sample entries ---------------------------------------------------

const SAMPLE_WINDOWS: WindowEntry[] = [
  { id: 1, app: "zoom.us", title: "Zoom Meeting", width: 1280, height: 720, thumb: windowThumb("Zoom Meeting", 0) },
  {
    id: 2,
    app: "Microsoft Edge",
    title: "Lecture video - SharePoint",
    width: 1440,
    height: 900,
    thumb: windowThumb("Lecture video", 1),
  },
  {
    id: 3,
    app: "QuickTime Player",
    title: "lecture_2026-10-06.mov",
    width: 1920,
    height: 1080,
    thumb: windowThumb("lecture_2026-10-06.mov", 2),
  },
];

function sampleSnapshot(windowId: number): Snapshot {
  const w = SAMPLE_WINDOWS.find((entry) => entry.id === windowId) ?? SAMPLE_WINDOWS[0];
  return {
    windowId: w.id,
    app: w.app,
    title: w.title,
    width: w.width,
    height: w.height,
    image: snapshotImage(w.width, w.height, w.title),
  };
}

// ---- platform-dependent defaults --------------------------------------

function defaultShortcuts(): Shortcuts {
  return isMac()
    ? { saveNow: "Control+Alt+Super+S", undo: "Control+Alt+Super+Z", pauseResume: "Control+Alt+Super+P" }
    : { saveNow: "Control+Alt+Shift+S", undo: "Control+Alt+Shift+Z", pauseResume: "Control+Alt+Shift+P" };
}

function defaultPlatform(): PermissionView["platform"] {
  if (isMac()) return "macos";
  const ua = typeof navigator === "undefined" ? "" : navigator.platform || navigator.userAgent || "";
  return /win/i.test(ua) ? "windows" : "linux";
}

function detectOsIsJapanese(): boolean {
  if (typeof navigator === "undefined") return false;
  return navigator.language?.toLowerCase().startsWith("ja") ?? false;
}

function effectiveLanguageFor(language: LangSetting): Lang {
  if (language === "ja" || language === "en") return language;
  return detectOsIsJapanese() ? "ja" : "en";
}

// ---- presets (selected once, via ?preset=<name>, for manual testing and
// scripts/ui-check.sh -- see tests/ui/*.spec.ts) -------------------------

function baseAppState(): AppState {
  return {
    session: null,
    lastSession: null,
    permission: { granted: true, platform: defaultPlatform() },
    videoProgress: null,
  };
}

function sampleSession(state: SessionState): SessionView {
  return {
    state,
    count: 12,
    dir: "~/Pictures/ZoomSlidesPicker/2026-10-06_1900",
    lastThumb: captureThumb(12),
    elapsedS: 754,
    blank: false,
    windowLost: false,
    sourceKind: "window",
    sourceTitle: "Zoom Meeting",
    stopping: false,
  };
}

const PRESETS: Record<string, () => AppState> = {
  stopped: () => baseAppState(),
  "stopped-last": () => ({
    ...baseAppState(),
    lastSession: { dir: "~/Pictures/ZoomSlidesPicker/2026-10-05_1400", count: 18, hasPdf: true },
  }),
  running: () => ({ ...baseAppState(), session: sampleSession("running") }),
  paused: () => ({ ...baseAppState(), session: sampleSession("paused") }),
  stopping: () => ({ ...baseAppState(), session: { ...sampleSession("running"), stopping: true } }),
  "window-lost": () => ({ ...baseAppState(), session: { ...sampleSession("paused"), windowLost: true } }),
  blank: () => ({ ...baseAppState(), session: { ...sampleSession("running"), blank: true } }),
  "no-permission": () => ({ ...baseAppState(), permission: { granted: false, platform: "macos" } }),
  "video-progress": () => ({ ...baseAppState(), videoProgress: { done: 42, total: 120 } }),
};

function presetFromLocation(): string {
  if (typeof location === "undefined") return "stopped";
  return new URLSearchParams(location.search).get("preset") ?? "stopped";
}

function sessionFolderName(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}_${pad(d.getHours())}${pad(d.getMinutes())}`;
}

// ---- module state and pub/sub ------------------------------------------

let appState: AppState = (PRESETS[presetFromLocation()] ?? PRESETS.stopped)();
// A long, realistic save root (a synced Pictures folder) so the settings row's truncation
// is exercised by the UI checks.
let settings: Settings = {
  language: "auto",
  saveRoot: "/Users/student/Library/CloudStorage/OneDrive-University/Pictures/ZoomSlidesPicker",
  intervalS: 0.5,
  shortcuts: defaultShortcuts(),
};
let lastSnapshotWindowId: number = SAMPLE_WINDOWS[0].id;

const appStateListeners = new Set<(s: AppState) => void>();
const settingsListeners = new Set<(s: SettingsView) => void>();
const libraryChangedListeners = new Set<() => void>();

function settingsView(): SettingsView {
  return { ...settings, effectiveLanguage: effectiveLanguageFor(settings.language) };
}

function emitAppState(): void {
  for (const listener of appStateListeners) listener(appState);
}

function emitSettings(): void {
  const view = settingsView();
  for (const listener of settingsListeners) listener(view);
}

function emitLibraryChanged(): void {
  for (const listener of libraryChangedListeners) listener();
}

/**
 * Error/status text the library commands below need that already has a home in
 * src/i18n -- reused via the raw ja/en dictionaries (not the "../i18n" barrel, which
 * pulls in useLang() and therefore ../ipc itself: importing that here would be a real
 * module cycle). Keeps the Japanese/English wording in exactly one place rather than
 * duplicating it as inline literals the way this file's other error strings
 * (setSettings, rebuildPdf below) do, since these particular ones are already spelled
 * out verbatim in docs/impl/library.md's own strings table.
 */
function libT(key: "library.notFound" | "library.noPdfError" | "library.recordingLocked"): string {
  return effectiveLanguageFor(settings.language) === "ja" ? ja[key] : en[key];
}

function delay<T>(value: T, ms: number): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), ms));
}

// A session "in progress" ticks its elapsed time, for realism in `pnpm dev`. Vitest
// component tests never import this module (they vi.mock("../ipc") instead), so this
// timer never runs under the test suite.
if (typeof window !== "undefined") {
  setInterval(() => {
    if (appState.session?.state === "running") {
      appState = { ...appState, session: { ...appState.session, elapsedS: appState.session.elapsedS + 1 } };
      emitAppState();
    }
  }, 1000);
}

// ---- commands (docs/impl/ipc.md) ---------------------------------------

export async function getAppState(): Promise<AppState> {
  return appState;
}

export async function getSettings(): Promise<SettingsView> {
  return settingsView();
}

export async function setSettings(next: Settings): Promise<SettingsView> {
  const stepsFromZero = next.intervalS * 2;
  if (next.intervalS < 0.5 || next.intervalS > 5 || Math.round(stepsFromZero) !== stepsFromZero) {
    throw new Error(
      effectiveLanguageFor(settings.language) === "ja"
        ? "比較の周期は 0.5〜5 秒、0.5 刻みで指定してください。"
        : "The check interval must be between 0.5 and 5 seconds, in steps of 0.5."
    );
  }
  settings = next;
  emitSettings();
  return settingsView();
}

export async function applyLanguage(): Promise<void> {
  // Rebuilds tray menu / tooltip / window titles outside the webview. Nothing to
  // simulate in a plain browser; kept as a real async function so callers can await it.
}

export async function chooseSaveRoot(): Promise<string | null> {
  // No native dialog outside Tauri. The mock "picks" a fixed sample path so the
  // choose -> set_settings flow can still be exercised end to end.
  return "~/Pictures/ZoomSlidesPicker";
}

export async function listWindows(): Promise<WindowEntry[]> {
  return delay(SAMPLE_WINDOWS.map((w) => ({ ...w })), 350);
}

export async function beginRegion(windowId: number): Promise<void> {
  lastSnapshotWindowId = windowId;
}

export async function getRegionSnapshot(): Promise<Snapshot> {
  return sampleSnapshot(lastSnapshotWindowId);
}

export async function startSession(windowId: number, region: RegionSel): Promise<void> {
  void region; // the mock does not simulate cropping; the real backend crops captures to it
  const w = SAMPLE_WINDOWS.find((entry) => entry.id === windowId) ?? SAMPLE_WINDOWS[0];
  appState = {
    ...appState,
    session: {
      state: "running",
      count: 0,
      dir: `~/Pictures/ZoomSlidesPicker/${sessionFolderName()}`,
      lastThumb: null,
      elapsedS: 0,
      blank: false,
      windowLost: false,
      sourceKind: "window",
      sourceTitle: w.title,
      stopping: false,
    },
  };
  emitAppState();
}

export async function cancelRegion(): Promise<void> {
  // The mock keeps no separate "region window open" flag to clear.
}

export async function pauseSession(): Promise<void> {
  if (appState.session?.state === "running") {
    appState = { ...appState, session: { ...appState.session, state: "paused" } };
    emitAppState();
  }
}

export async function resumeSession(): Promise<void> {
  if (appState.session?.state === "paused") {
    appState = { ...appState, session: { ...appState.session, state: "running", windowLost: false } };
    emitAppState();
  }
}

export async function saveNow(): Promise<void> {
  if (appState.session) {
    const count = appState.session.count + 1;
    appState = { ...appState, session: { ...appState.session, count, lastThumb: captureThumb(count) } };
    emitAppState();
  }
}

export async function undoLast(): Promise<void> {
  if (appState.session && appState.session.count > 0) {
    const count = appState.session.count - 1;
    appState = {
      ...appState,
      session: { ...appState.session, count, lastThumb: count > 0 ? captureThumb(count) : null },
    };
    emitAppState();
  }
}

export async function stopSession(): Promise<void> {
  if (appState.session && !appState.session.stopping) {
    const { count, dir } = appState.session;
    appState = { ...appState, session: { ...appState.session, stopping: true } };
    emitAppState();
    // The real backend writes the PDF before it reports the session as finished.
    setTimeout(() => {
      if (!appState.session?.stopping) return;
      appState = { ...appState, session: null, lastSession: { dir, count, hasPdf: count > 0 } };
      emitAppState();
    }, 1500);
  }
}

export async function openFolder(): Promise<void> {
  // Nothing to open in a plain browser; the real command opens the OS file manager.
}

export async function rebuildPdf(): Promise<number> {
  const lastSession = appState.lastSession;
  if (!lastSession || lastSession.count === 0) {
    throw new Error(
      effectiveLanguageFor(settings.language) === "ja"
        ? "直前のセッションの画像がありません。"
        : "There are no images from the last session."
    );
  }
  appState = { ...appState, lastSession: { ...lastSession, hasPdf: true } };
  emitAppState();
  return lastSession.count;
}

export async function extractVideo(): Promise<void> {
  throw new Error(
    effectiveLanguageFor(settings.language) === "ja"
      ? "このモックでは動画ファイルからの抽出を再現していません。"
      : "This mock does not simulate extracting from a video file."
  );
}

export async function requestPermission(): Promise<boolean> {
  appState = { ...appState, permission: { ...appState.permission, granted: true } };
  emitAppState();
  return true;
}

export async function openPermissionSettings(): Promise<void> {}

export async function openSettings(): Promise<void> {}

export async function closeWindow(): Promise<void> {}

// No real panel window to resize outside Tauri (the real backend clamps and resizes it,
// docs/impl/ipc.md > fit_panel); the last height is left on <html> for the UI checks.
export async function fitPanel(height: number): Promise<void> {
  document.documentElement.dataset.fitPanel = String(height);
}

// ---- Library (history) window: docs/impl/library.md > Types/Commands. Six invented
// sessions; no real lecture content. Independent of the appState/session above -- the
// library toolbar reuses that same mock state (see Library.tsx), but this list is its
// own, separate piece of mock data.

interface SessionSeed {
  folder: string;
  customTitle: string | null;
  memo: string;
  sourceApp: string | null;
  sourceTitle: string;
  sourceKind: "window" | "mp4";
  startedAt: string; // RFC 3339, Asia/Tokyo (+09:00) -- see lib/format.ts's comment on why
  endedAt: string | null;
  count: number;
  hasPdf: boolean;
  recording: boolean;
}

// A fixed "now" (rather than `new Date()`) for the one still-recording session's
// duration, so its mock meta line -- and every screenshot that shows it -- is the same
// on every run, not just whenever the suite happens to execute.
const MOCK_NOW = "2026-10-07T15:00:00+09:00";

// A plausible absolute save root for this mock's sessions (never a real user path).
const MOCK_SAVE_ROOT = "/Users/lecturer/Pictures/ZoomSlidesPicker";

const SESSION_SEEDS: SessionSeed[] = [
  {
    folder: "2026-10-07_1410",
    customTitle: null,
    memo: "",
    sourceApp: "Safari",
    sourceTitle: "第2回授業スライド.mp4",
    sourceKind: "window",
    startedAt: "2026-10-07T14:10:00+09:00",
    endedAt: null,
    count: 12,
    hasPdf: false,
    recording: true,
  },
  {
    folder: "2026-10-07_1242",
    customTitle: null,
    memo: "出席確認のあと、前回の復習から開始。レポートの締め切りを案内した。",
    sourceApp: "Safari",
    sourceTitle: "第1回授業スライド.mp4",
    sourceKind: "window",
    startedAt: "2026-10-07T12:42:00+09:00",
    endedAt: "2026-10-07T13:15:00+09:00",
    count: 36,
    hasPdf: true,
    recording: false,
  },
  // SPEC-QUESTION: docs/impl/library.md calls this one's name a "custom name"
  // explicitly; the next entry gives a second name the same way but without that
  // label. Read both as the same shape (a custom name over a generic "Zoom
  // ミーティング" source title) -- see this file's header comment and the report.
  {
    folder: "2026-10-03_1030",
    customTitle: "デジタル人文学 第1回",
    memo: "初回ガイダンス。参考文献リストを配布。",
    sourceApp: "zoom.us",
    sourceTitle: "Zoom ミーティング",
    sourceKind: "window",
    startedAt: "2026-10-03T10:30:00+09:00",
    endedAt: "2026-10-03T12:00:00+09:00",
    count: 58,
    hasPdf: true,
    recording: false,
  },
  {
    folder: "2026-10-02_1300",
    customTitle: "統計学特論",
    memo: "",
    sourceApp: "zoom.us",
    sourceTitle: "Zoom ミーティング",
    sourceKind: "window",
    startedAt: "2026-10-02T13:00:00+09:00",
    endedAt: "2026-10-02T14:30:00+09:00",
    count: 41,
    hasPdf: true,
    recording: false,
  },
  // SPEC-QUESTION: the spec gives a date but no time of day for this entry and the
  // next one; invented a plausible start/end (see the report).
  {
    folder: "2026-10-01_0900",
    customTitle: null,
    memo: "動画からの抽出。画質はやや低め。",
    sourceApp: null, // mp4 mode: no source app (types.ts)
    sourceTitle: "lecture_2026-10-01.mp4",
    sourceKind: "mp4",
    startedAt: "2026-10-01T09:00:00+09:00",
    endedAt: "2026-10-01T10:15:00+09:00",
    count: 22,
    hasPdf: true,
    recording: false,
  },
  {
    folder: "2026-09-30_1000",
    customTitle: null,
    memo: "",
    sourceApp: "Microsoft Edge",
    sourceTitle: "情報処理演習 - Microsoft Stream",
    sourceKind: "window",
    startedAt: "2026-09-30T10:00:00+09:00",
    endedAt: "2026-09-30T10:02:00+09:00",
    count: 0,
    hasPdf: false,
    recording: false,
  },
];

function seedDurationSeconds(seed: SessionSeed): number {
  const start = new Date(seed.startedAt).getTime();
  const end = new Date(seed.endedAt ?? MOCK_NOW).getTime();
  return Math.max(0, Math.round((end - start) / 1000));
}

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

// path -> generated SVG data URL (docs/impl/library.md > Types: "the mock returns
// generated SVG data URLs"), filled in as buildCaptures() below seeds each session.
const THUMB_IMAGES = new Map<string, string>();

/** Captures spaced evenly across the session, from t=0 to t=totalSeconds. */
function buildCaptures(dir: string, count: number, totalSeconds: number): CaptureView[] {
  return Array.from({ length: count }, (_, i) => {
    const index = i + 1;
    const tContent = count > 1 ? Math.round((i * totalSeconds) / (count - 1)) : 0;
    const hh = Math.floor(tContent / 3600);
    const mm = Math.floor((tContent % 3600) / 60);
    const ss = tContent % 60;
    // ACCEPTANCE B2 / docs/session.schema.json's capture file pattern: NNN_hh-mm-ss.png.
    const file = `${String(index).padStart(3, "0")}_${pad2(hh)}-${pad2(mm)}-${pad2(ss)}.png`;
    const path = `${dir}/${file}`;
    THUMB_IMAGES.set(path, librarySlideSvg(index, `Slide ${index}`));
    return { index, file, path, tContent };
  });
}

interface SessionRecord {
  summary: SessionSummary;
  captures: CaptureView[];
}

// Mutable (library commands below trash/rename/re-tag sessions and captures), unlike
// the rest of this mock's seed-derived data.
let sessionRecords: SessionRecord[] = SESSION_SEEDS.map((seed) => {
  const dir = `${MOCK_SAVE_ROOT}/${seed.folder}`;
  const captures = seed.count > 0 ? buildCaptures(dir, seed.count, seedDurationSeconds(seed)) : [];
  const summary: SessionSummary = {
    dir,
    folder: seed.folder,
    title: seed.customTitle ?? seed.sourceTitle,
    customTitle: seed.customTitle,
    memo: seed.memo,
    sourceApp: seed.sourceApp,
    sourceTitle: seed.sourceTitle,
    sourceKind: seed.sourceKind,
    startedAt: seed.startedAt,
    endedAt: seed.endedAt,
    count: seed.count,
    hasPdf: seed.hasPdf,
    recording: seed.recording,
    thumbPath: captures[0]?.path ?? null,
  };
  return { summary, captures };
});

// fold_session's fixed, plausible result for this one sample session (docs/impl/
// library.md > mock: "fold_session returns a fixed plausible list for one sample
// session and [] for others") -- indices zsp_detect::fold_captures would not keep.
const DEDUPE_DIR = `${MOCK_SAVE_ROOT}/2026-10-02_1300`;
const DEDUPE_INDICES = [5, 6, 17, 18, 29];

function findRecord(dir: string): SessionRecord | undefined {
  return sessionRecords.find((r) => r.summary.dir === dir);
}

function notFoundError(): Error {
  return new Error(libT("library.notFound"));
}

function noPdfError(): Error {
  return new Error(libT("library.noPdfError"));
}

function recordingLockedError(): Error {
  return new Error(libT("library.recordingLocked"));
}

export async function openLibrary(): Promise<void> {
  // Nothing to open in a plain browser; the real command opens or focuses the window.
}

export async function listSessions(): Promise<SessionSummary[]> {
  return [...sessionRecords]
    .sort((a, b) => new Date(b.summary.startedAt).getTime() - new Date(a.summary.startedAt).getTime())
    .map((r) => ({ ...r.summary }));
}

export async function getSession(dir: string): Promise<SessionDetail> {
  const record = findRecord(dir);
  if (!record) throw notFoundError();
  return { ...record.summary, captures: record.captures.map((c) => ({ ...c })) };
}

export async function openSessionFolder(dir: string): Promise<void> {
  if (!findRecord(dir)) throw notFoundError();
  // Nothing to open in a plain browser; the real command opens the OS file manager.
}

export async function openSessionPdf(dir: string): Promise<void> {
  const record = findRecord(dir);
  if (!record) throw notFoundError();
  if (!record.summary.hasPdf) throw noPdfError();
  // Nothing to open in a plain browser; the real command opens slides.pdf.
}

const MB = 1024 * 1024;
// Mock bytes per page, from the contract's measured ratios.
const MOCK_BYTES_PER_PAGE: Record<PdfQuality, number> = {
  lossless: 1.4 * MB,
  standard: 0.4 * MB,
  light: 0.19 * MB,
};
const MOCK_QUALITIES: PdfQuality[] = ["lossless", "standard", "light"];
const MOCK_DELAY_MS = 600;

function exportPages(record: SessionRecord, indices: number[] | null): number {
  if (indices === null) return record.captures.length;
  const wanted = new Set(indices);
  return record.captures.filter((c) => wanted.has(c.index)).length;
}

export async function estimatePdf(dir: string, indices: number[] | null): Promise<PdfEstimate[]> {
  const record = findRecord(dir);
  if (!record) throw notFoundError();
  if (record.summary.recording) throw recordingLockedError();
  const pages = exportPages(record, indices);
  await delay(undefined, MOCK_DELAY_MS);
  return MOCK_QUALITIES.map((quality) => ({ quality, bytes: Math.round(pages * MOCK_BYTES_PER_PAGE[quality]) }));
}

export async function exportPdf(dir: string, quality: PdfQuality, indices: number[] | null): Promise<PdfExport> {
  const record = findRecord(dir);
  if (!record) throw notFoundError();
  if (record.summary.recording) throw recordingLockedError();
  const pages = exportPages(record, indices);
  if (pages === 0) throw noPdfError();
  await delay(undefined, MOCK_DELAY_MS);
  let path = `${dir}/slides.pdf`;
  if (indices === null) {
    record.summary.hasPdf = true;
  } else {
    const ja = effectiveLanguageFor(settings.language) === "ja";
    const date = record.summary.startedAt.slice(0, 10);
    path = `${dir}/${ja ? "選んだ画像" : "Selected images"} ${date}.pdf`;
  }
  emitLibraryChanged();
  return { path, pages, bytes: Math.round(pages * MOCK_BYTES_PER_PAGE[quality]) };
}

export async function renameSession(dir: string, title: string | null): Promise<void> {
  const record = findRecord(dir);
  if (!record) throw notFoundError();
  const trimmed = (title ?? "").trim();
  record.summary.customTitle = trimmed === "" ? null : trimmed;
  record.summary.title = record.summary.customTitle ?? record.summary.sourceTitle;
  emitLibraryChanged();
}

export async function setSessionMemo(dir: string, memo: string): Promise<void> {
  const record = findRecord(dir);
  if (!record) throw notFoundError();
  record.summary.memo = memo;
  emitLibraryChanged();
}

export async function trashSession(dir: string): Promise<void> {
  const record = findRecord(dir);
  if (!record) throw notFoundError();
  if (record.summary.recording) throw recordingLockedError();
  sessionRecords = sessionRecords.filter((r) => r !== record);
  if (appState.lastSession?.dir === dir) {
    appState = { ...appState, lastSession: null };
    emitAppState();
  }
  emitLibraryChanged();
}

export async function trashCaptures(dir: string, indices: number[]): Promise<number> {
  const record = findRecord(dir);
  if (!record) throw notFoundError();
  if (record.summary.recording) throw recordingLockedError();
  const toTrash = new Set(indices);
  const before = record.captures.length;
  record.captures = record.captures.filter((c) => !toTrash.has(c.index));
  const moved = before - record.captures.length;
  record.summary.count = record.captures.length;
  record.summary.thumbPath = record.captures[0]?.path ?? null;
  record.summary.hasPdf = record.summary.hasPdf && record.captures.length > 0;
  emitLibraryChanged();
  return moved;
}

export async function foldSession(dir: string): Promise<number[]> {
  const record = findRecord(dir);
  if (!record) throw notFoundError();
  if (record.summary.recording) throw recordingLockedError();
  return dir === DEDUPE_DIR ? [...DEDUPE_INDICES] : [];
}

export async function openCapture(path: string): Promise<void> {
  if (!THUMB_IMAGES.has(path)) throw notFoundError();
  // Nothing to open in a plain browser; the real command opens the OS default viewer.
}

export function thumbUrl(path: string, width: number): string {
  void width; // the mock's generated SVGs are fixed-size, never actually resized
  return THUMB_IMAGES.get(path) ?? librarySlideSvg(0, "?");
}

export async function onLibraryChanged(cb: () => void): Promise<UnlistenFn> {
  libraryChangedListeners.add(cb);
  return () => {
    libraryChangedListeners.delete(cb);
  };
}

export async function onAppState(cb: (state: AppState) => void): Promise<UnlistenFn> {
  appStateListeners.add(cb);
  return () => {
    appStateListeners.delete(cb);
  };
}

export async function onSettings(cb: (settings: SettingsView) => void): Promise<UnlistenFn> {
  settingsListeners.add(cb);
  return () => {
    settingsListeners.delete(cb);
  };
}

// ---- In-app updates (docs/impl/updates.md) --------------------------------
// Scenario via the URL query, like `preset` above (all optional):
//   update=offer          the launch check found 0.2.0 (get_update_offer returns it; default: none)
//   check=uptodate|available|skipped|error   what the manual check returns (default: uptodate)
//   install=stall|fail    install_update downloads to 40% and then stays there (default, so a
//                         screenshot can be taken), or fails after the progress (message below)
// Combine with `preset=running` for the busy state. Module-level setters below let a
// script change the scenario at runtime too.

const MOCK_CURRENT_VERSION = "0.1.6";
const MOCK_NEW_VERSION = "0.2.0";

function queryParam(name: string): string | null {
  if (typeof location === "undefined") return null;
  return new URLSearchParams(location.search).get(name);
}

type MockCheckResult = "uptodate" | "available" | "skipped" | "error";
type MockInstallResult = "stall" | "fail";

let updateOffer: UpdateInfo | null =
  queryParam("update") === "offer" ? { version: MOCK_NEW_VERSION, current: MOCK_CURRENT_VERSION } : null;
let checkResult: MockCheckResult = (queryParam("check") as MockCheckResult | null) ?? "uptodate";
let installResult: MockInstallResult = (queryParam("install") as MockInstallResult | null) ?? "stall";

const updateOfferListeners = new Set<(i: UpdateInfo) => void>();
const updateProgressListeners = new Set<(p: UpdateProgress) => void>();

/** Sets the unanswered launch offer and emits `update-offer` when one is given. */
export function setMockUpdateOffer(info: UpdateInfo | null): void {
  updateOffer = info;
  if (info) for (const listener of updateOfferListeners) listener(info);
}

export function setMockCheckResult(result: MockCheckResult): void {
  checkResult = result;
}

export function setMockInstallResult(result: MockInstallResult): void {
  installResult = result;
}

function emitUpdateProgress(p: UpdateProgress): void {
  for (const listener of updateProgressListeners) listener(p);
}

export async function getAppVersion(): Promise<string> {
  return MOCK_CURRENT_VERSION;
}

export async function getUpdateOffer(): Promise<UpdateInfo | null> {
  return updateOffer;
}

export async function checkUpdate(): Promise<UpdateCheck> {
  const found: UpdateInfo = { version: MOCK_NEW_VERSION, current: MOCK_CURRENT_VERSION };
  if (checkResult === "error") {
    throw new Error(effectiveLanguageFor(settings.language) === "ja" ? "ネットワークに接続できません" : "Network is unreachable");
  }
  return {
    current: MOCK_CURRENT_VERSION,
    available: checkResult === "uptodate" ? null : found,
    skipped: checkResult === "skipped",
  };
}

export async function answerUpdate(answer: UpdateAnswer): Promise<void> {
  void answer;
  updateOffer = null;
}

export async function installUpdate(): Promise<void> {
  if (appState.session !== null || appState.videoProgress !== null) {
    throw new Error("A session is active");
  }
  const total = 50_000_000;
  emitUpdateProgress({ phase: "downloading", downloaded: 0, total });
  await delay(null, 150);
  emitUpdateProgress({ phase: "downloading", downloaded: 20_000_000, total });
  if (installResult === "fail") {
    await delay(null, 150);
    throw new Error("The signature does not match");
  }
  // Like the real app on macOS: never settles (the app would restart here).
  return new Promise<void>(() => {});
}

export async function onUpdateOffer(cb: (info: UpdateInfo) => void): Promise<UnlistenFn> {
  updateOfferListeners.add(cb);
  return () => {
    updateOfferListeners.delete(cb);
  };
}

export async function onUpdateProgress(cb: (progress: UpdateProgress) => void): Promise<UnlistenFn> {
  updateProgressListeners.add(cb);
  return () => {
    updateProgressListeners.delete(cb);
  };
}
