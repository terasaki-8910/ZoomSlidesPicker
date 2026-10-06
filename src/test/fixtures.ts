import type { AppState, SettingsView } from "../ipc/types";

export function makeSettingsView(overrides: Partial<SettingsView> = {}): SettingsView {
  return {
    language: "auto",
    effectiveLanguage: "ja",
    saveRoot: "~/Pictures/ZoomSlidesPicker",
    intervalS: 0.5,
    shortcuts: {
      saveNow: "Control+Alt+Super+S",
      undo: "Control+Alt+Super+Z",
      pauseResume: "Control+Alt+Super+P",
    },
    ...overrides,
  };
}

export function makeAppState(overrides: Partial<AppState> = {}): AppState {
  return {
    session: null,
    lastSession: null,
    permission: { granted: true, platform: "macos" },
    videoProgress: null,
    ...overrides,
  };
}
