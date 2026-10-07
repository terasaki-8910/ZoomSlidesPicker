// Types from docs/impl/ipc.md, kept in exact sync with it (serde rename_all = "camelCase"
// on the Rust side; there is no code generation, so change both sides together).

export type Lang = "ja" | "en";
export type LangSetting = "auto" | "ja" | "en";

/** Accelerator strings in tauri-plugin-global-shortcut format. */
export interface Shortcuts {
  saveNow: string; // default macOS "Control+Alt+Super+S", Windows "Control+Alt+Shift+S"
  undo: string; // default macOS "Control+Alt+Super+Z", Windows "Control+Alt+Shift+Z"
  pauseResume: string; // default macOS "Control+Alt+Super+P", Windows "Control+Alt+Shift+P"
}

export interface Settings {
  language: LangSetting; // default "auto"
  saveRoot: string; // default: <OS Pictures folder>/ZoomSlidesPicker
  intervalS: number; // comparison period N, 0.5-5 in steps of 0.5, default 0.5
  shortcuts: Shortcuts;
}

export interface SettingsView extends Settings {
  /** "ja" when language is "ja", or "auto" and the OS locale starts with "ja"; else "en". */
  effectiveLanguage: Lang;
}

export type SessionState = "running" | "paused" | "stopped";

export interface SessionView {
  state: SessionState;
  count: number; // captures in the session
  dir: string; // session folder
  lastThumb: string | null; // data URL (PNG, at most 280 px wide) of the latest capture
  elapsedS: number;
  blank: boolean; // single-colour screen for 10 s or more
  windowLost: boolean; // the window is gone; the session is paused
  sourceKind: "window" | "mp4";
  sourceTitle: string; // window title, or video file name
}

/** The most recent finished session. */
export interface LastSession {
  dir: string;
  count: number;
  hasPdf: boolean;
}

export interface PermissionView {
  granted: boolean; // always true outside macOS
  platform: "macos" | "windows" | "linux";
}

export interface VideoProgress {
  done: number; // frames processed
  total: number | null; // when known
}

export interface AppState {
  session: SessionView | null; // null when no session is running or paused
  lastSession: LastSession | null;
  permission: PermissionView;
  videoProgress: VideoProgress | null; // non-null while a video is being processed
}

export interface WindowEntry {
  id: number;
  app: string;
  title: string;
  width: number; // points
  height: number;
  thumb: string; // data URL (PNG, at most 240 px wide)
}

/** For the region window. */
export interface Snapshot {
  windowId: number;
  app: string;
  title: string;
  width: number; // pixels of the captured window
  height: number;
  image: string; // data URL (PNG, at most 1600 px wide)
}

/** Fractions 0-1 of the window. */
export interface RegionSel {
  x: number;
  y: number;
  w: number;
  h: number;
}

// ---- Library (history) window: docs/impl/library.md > Data ----------------

export interface SessionSummary {
  id: string; // folder name, e.g. "2026-10-07_1242"
  title: string; // custom name, else source title
  customTitle: string | null;
  sourceApp: string | null; // e.g. "Safari"; null for mp4
  sourceTitle: string; // window title or video file name
  sourceKind: "window" | "mp4";
  startedAt: string; // RFC 3339
  endedAt: string | null; // null while recording
  count: number;
  hasPdf: boolean;
  recording: boolean;
  thumb: string | null; // data URL of the first slide (small), null when count is 0
}

export interface SessionDetail extends SessionSummary {
  captures: { index: number; file: string; tContent: number; thumb: string }[];
}
