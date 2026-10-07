// Typed wrappers over @tauri-apps/api invoke/listen, matching docs/impl/ipc.md exactly.
// The webview never touches files, dialogs, the shell or the OS directly -- everything
// goes through these functions.
//
// Component tests replace this whole module with `vi.mock("../../ipc", ...)` and supply
// their own fakes per test, so they do not depend on src/ipc/mock.ts at all. The mock
// below is used instead at runtime (pnpm dev, pnpm build:mock/preview:mock, and
// scripts/ui-check.sh) when VITE_IPC_MOCK=1.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import * as mock from "./mock";
import type { AppState, RegionSel, SessionDetail, SessionSummary, Settings, SettingsView, Snapshot, WindowEntry } from "./types";

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

// ---- Library (history) window: docs/impl/library.md > Data. Prototype only -- the
// Rust commands below do not exist yet (the real branch is written for when they do).
export function listSessions(): Promise<SessionSummary[]> {
  return useMock ? mock.listSessions() : invoke("list_sessions");
}

export function getSession(id: string): Promise<SessionDetail> {
  return useMock ? mock.getSession(id) : invoke("get_session", { id });
}
