// In-memory implementation of docs/impl/ipc.md, used when VITE_IPC_MOCK=1 (pnpm dev,
// pnpm build:mock / preview:mock, and scripts/ui-check.sh). Keeps state and emits
// app-state/settings to listeners so flows work end to end in a plain browser: start a
// session, pause, save now, undo, stop, and so on.
//
// All sample data below is invented for this mock -- window titles, thumbnails and the
// snapshot image are generated SVGs with neutral shapes and placeholder labels, never
// real screen content.
import { isMac } from "../lib/platform";
import type {
  AppState,
  Lang,
  LangSetting,
  PermissionView,
  RegionSel,
  SessionState,
  SessionView,
  Settings,
  SettingsView,
  Shortcuts,
  Snapshot,
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
  if (appState.session) {
    const { count, dir } = appState.session;
    appState = { ...appState, session: null, lastSession: { dir, count, hasPdf: count > 0 } };
    emitAppState();
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
