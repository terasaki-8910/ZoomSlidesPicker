import { vi } from "vitest";

/**
 * A self-contained factory for `vi.mock("../../ipc", () => createIpcMock())`.
 * Calling an *imported* function from inside a vi.mock factory is fine (only
 * referencing a same-file local declared below the call is the usual hoisting trap),
 * so every test file can share this instead of repeating the full command list.
 */
export function createIpcMock() {
  return {
    getAppState: vi.fn(),
    getSettings: vi.fn(),
    setSettings: vi.fn(),
    applyLanguage: vi.fn(),
    chooseSaveRoot: vi.fn(),
    listWindows: vi.fn(),
    beginRegion: vi.fn(),
    getRegionSnapshot: vi.fn(),
    startSession: vi.fn(),
    cancelRegion: vi.fn(),
    pauseSession: vi.fn(),
    resumeSession: vi.fn(),
    saveNow: vi.fn(),
    undoLast: vi.fn(),
    stopSession: vi.fn(),
    openFolder: vi.fn(),
    rebuildPdf: vi.fn(),
    extractVideo: vi.fn(),
    requestPermission: vi.fn(),
    openPermissionSettings: vi.fn(),
    openSettings: vi.fn(),
    closeWindow: vi.fn(),
    fitPanel: vi.fn(),
    onAppState: vi.fn(),
    onSettings: vi.fn(),
    openLibrary: vi.fn(),
    listSessions: vi.fn(),
    getSession: vi.fn(),
    openSessionFolder: vi.fn(),
    openSessionPdf: vi.fn(),
    rebuildSessionPdf: vi.fn(),
    renameSession: vi.fn(),
    setSessionMemo: vi.fn(),
    trashSession: vi.fn(),
    trashCaptures: vi.fn(),
    foldSession: vi.fn(),
    openCapture: vi.fn(),
    onLibraryChanged: vi.fn(),
    thumbUrl: vi.fn(),
  };
}

/**
 * A mocked onAppState/onSettings implementation that captures every subscriber so a
 * test can simulate an event arriving later with `.emit(payload)`. More than one part
 * of a view can subscribe independently -- e.g. Settings.tsx itself and the useLang()
 * hook it also calls both subscribe to onSettings -- so this must notify all of them,
 * not just the most recent one.
 */
export function createListenerMock<T>() {
  const listeners = new Set<(payload: T) => void>();
  const fn = vi.fn((cb: (payload: T) => void) => {
    listeners.add(cb);
    return Promise.resolve(() => {
      listeners.delete(cb);
    });
  });
  return {
    fn,
    emit: (payload: T) => {
      for (const listener of listeners) listener(payload);
    },
  };
}
