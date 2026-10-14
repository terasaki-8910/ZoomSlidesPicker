// Typed wrappers over @tauri-apps/api invoke/listen, matching docs/impl/ipc.md exactly.
// The webview never touches files, dialogs, the shell or the OS directly -- everything
// goes through these functions.
//
// Component tests replace this whole module with `vi.mock("../../ipc", ...)` and supply
// their own fakes per test, so they do not depend on src/ipc/mock.ts at all. The mock
// below is used instead at runtime (pnpm dev, pnpm build:mock/preview:mock, and
// scripts/ui-check.sh) when VITE_IPC_MOCK=1.
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import * as mock from "./mock";
import type { AppState, PdfEstimate, PdfExport, PdfQuality, RegionSel, SessionDetail, SessionSummary, Settings, SettingsView, Snapshot, UpdateAnswer, UpdateCheck, UpdateInfo, UpdateProgress, WindowEntry } from "./types";

const useMock = import.meta.env.VITE_IPC_MOCK === "1";

export function getAppState(): Promise<AppState> {
  return useMock ? mock.getAppState() : invoke("get_app_state");
}

export function getSettings(): Promise<SettingsView> {
  return useMock ? mock.getSettings() : invoke("get_settings");
}

export function setSettings(settings: Settings): Promise<SettingsView> {
  return useMock ? mock.setSettings(settings) : invoke("set_settings", { settings });
}

export function applyLanguage(): Promise<void> {
  return useMock ? mock.applyLanguage() : invoke("apply_language");
}

export function chooseSaveRoot(): Promise<string | null> {
  return useMock ? mock.chooseSaveRoot() : invoke("choose_save_root");
}

export function listWindows(): Promise<WindowEntry[]> {
  return useMock ? mock.listWindows() : invoke("list_windows");
}

export function beginRegion(windowId: number): Promise<void> {
  return useMock ? mock.beginRegion(windowId) : invoke("begin_region", { windowId });
}

export function getRegionSnapshot(): Promise<Snapshot> {
  return useMock ? mock.getRegionSnapshot() : invoke("get_region_snapshot");
}

export function startSession(windowId: number, region: RegionSel): Promise<void> {
  return useMock ? mock.startSession(windowId, region) : invoke("start_session", { windowId, region });
}

export function cancelRegion(): Promise<void> {
  return useMock ? mock.cancelRegion() : invoke("cancel_region");
}

export function pauseSession(): Promise<void> {
  return useMock ? mock.pauseSession() : invoke("pause_session");
}

export function resumeSession(): Promise<void> {
  return useMock ? mock.resumeSession() : invoke("resume_session");
}

export function saveNow(): Promise<void> {
  return useMock ? mock.saveNow() : invoke("save_now");
}

export function undoLast(): Promise<void> {
  return useMock ? mock.undoLast() : invoke("undo_last");
}

export function stopSession(): Promise<void> {
  return useMock ? mock.stopSession() : invoke("stop_session");
}

export function openFolder(): Promise<void> {
  return useMock ? mock.openFolder() : invoke("open_folder");
}

export function rebuildPdf(): Promise<number> {
  return useMock ? mock.rebuildPdf() : invoke("rebuild_pdf");
}

export function extractVideo(): Promise<void> {
  return useMock ? mock.extractVideo() : invoke("extract_video");
}

export function requestPermission(): Promise<boolean> {
  return useMock ? mock.requestPermission() : invoke("request_permission");
}

export function openPermissionSettings(): Promise<void> {
  return useMock ? mock.openPermissionSettings() : invoke("open_permission_settings");
}

export function openSettings(): Promise<void> {
  return useMock ? mock.openSettings() : invoke("open_settings");
}

export function closeWindow(): Promise<void> {
  return useMock ? mock.closeWindow() : invoke("close_window");
}

export function fitPanel(height: number): Promise<void> {
  return useMock ? mock.fitPanel(height) : invoke("fit_panel", { height });
}

export function onAppState(cb: (state: AppState) => void): Promise<UnlistenFn> {
  return useMock ? mock.onAppState(cb) : listen<AppState>("app-state", (e) => cb(e.payload));
}

export function onSettings(cb: (settings: SettingsView) => void): Promise<UnlistenFn> {
  return useMock ? mock.onSettings(cb) : listen<SettingsView>("settings", (e) => cb(e.payload));
}

// ---- Library (history) window: docs/impl/library.md > Commands. The Rust commands
// below do not exist yet (src-tauri/src/library.rs lands separately); this branch is
// written for when they do, against the mock in the meantime.
export function openLibrary(): Promise<void> {
  return useMock ? mock.openLibrary() : invoke("open_library");
}

export function listSessions(): Promise<SessionSummary[]> {
  return useMock ? mock.listSessions() : invoke("list_sessions");
}

export function getSession(dir: string): Promise<SessionDetail> {
  return useMock ? mock.getSession(dir) : invoke("get_session", { dir });
}

export function openSessionFolder(dir: string): Promise<void> {
  return useMock ? mock.openSessionFolder(dir) : invoke("open_session_folder", { dir });
}

export function openSessionPdf(dir: string): Promise<void> {
  return useMock ? mock.openSessionPdf(dir) : invoke("open_session_pdf", { dir });
}

export function estimatePdf(dir: string, indices: number[] | null): Promise<PdfEstimate[]> {
  return useMock ? mock.estimatePdf(dir, indices) : invoke("estimate_pdf", { dir, indices });
}

export function exportPdf(dir: string, quality: PdfQuality, indices: number[] | null): Promise<PdfExport> {
  return useMock ? mock.exportPdf(dir, quality, indices) : invoke("export_pdf", { dir, quality, indices });
}

export function renameSession(dir: string, title: string | null): Promise<void> {
  return useMock ? mock.renameSession(dir, title) : invoke("rename_session", { dir, title });
}

export function setSessionMemo(dir: string, memo: string): Promise<void> {
  return useMock ? mock.setSessionMemo(dir, memo) : invoke("set_session_memo", { dir, memo });
}

export function trashSession(dir: string): Promise<void> {
  return useMock ? mock.trashSession(dir) : invoke("trash_session", { dir });
}

export function trashCaptures(dir: string, indices: number[]): Promise<number> {
  return useMock ? mock.trashCaptures(dir, indices) : invoke("trash_captures", { dir, indices });
}

export function foldSession(dir: string): Promise<number[]> {
  return useMock ? mock.foldSession(dir) : invoke("fold_session", { dir });
}

export function openCapture(path: string): Promise<void> {
  return useMock ? mock.openCapture(path) : invoke("open_capture", { path });
}

export function onLibraryChanged(cb: () => void): Promise<UnlistenFn> {
  return useMock ? mock.onLibraryChanged(cb) : listen("library-changed", () => cb());
}

/**
 * Never fetches image bytes through a command -- the webview loads this URL directly.
 * Tauri builds `zspthumb://localhost/...` on macOS and `http://zspthumb.localhost/...`
 * on Windows (docs/impl/library.md > Types); the mock instead returns one of the
 * generated SVG data URLs it keeps keyed by path.
 */
export function thumbUrl(path: string, width: number): string {
  return useMock ? mock.thumbUrl(path, width) : `${convertFileSrc(path, "zspthumb")}?w=${width}`;
}

// ---- In-app updates: docs/impl/updates.md > Commands and Events.
export function getAppVersion(): Promise<string> {
  return useMock ? mock.getAppVersion() : invoke("get_app_version");
}

export function getUpdateOffer(): Promise<UpdateInfo | null> {
  return useMock ? mock.getUpdateOffer() : invoke("get_update_offer");
}

export function checkUpdate(): Promise<UpdateCheck> {
  return useMock ? mock.checkUpdate() : invoke("check_update");
}

export function answerUpdate(answer: UpdateAnswer): Promise<void> {
  return useMock ? mock.answerUpdate(answer) : invoke("answer_update", { answer });
}

/** On macOS the app restarts before this settles; it rejects with the error message on failure. */
export function installUpdate(): Promise<void> {
  return useMock ? mock.installUpdate() : invoke("install_update");
}

export function onUpdateOffer(cb: (info: UpdateInfo) => void): Promise<UnlistenFn> {
  return useMock ? mock.onUpdateOffer(cb) : listen<UpdateInfo>("update-offer", (e) => cb(e.payload));
}

export function onUpdateProgress(cb: (progress: UpdateProgress) => void): Promise<UnlistenFn> {
  return useMock ? mock.onUpdateProgress(cb) : listen<UpdateProgress>("update-progress", (e) => cb(e.payload));
}
