import type { AppState, SessionDetail, SessionSummary, SettingsView } from "../ipc/types";

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

/** docs/impl/library.md > Types -- `dir` is the identifier. */
export function makeSessionSummary(overrides: Partial<SessionSummary> = {}): SessionSummary {
  return {
    dir: "/mock/ZoomSlidesPicker/2026-10-07_1242",
    folder: "2026-10-07_1242",
    title: "第1回授業",
    customTitle: "第1回授業",
    memo: "",
    sourceApp: "Safari",
    sourceTitle: "video.mp4",
    sourceKind: "window",
    startedAt: "2026-10-07T12:42:00+09:00",
    endedAt: "2026-10-07T13:15:00+09:00",
    count: 2,
    hasPdf: true,
    recording: false,
    thumbPath: null,
    ...overrides,
  };
}

export function makeSessionDetail(overrides: Partial<SessionDetail> = {}): SessionDetail {
  const summary = makeSessionSummary(overrides);
  return {
    ...summary,
    captures: [
      { index: 1, file: "001_00-00-00.png", path: `${summary.dir}/001_00-00-00.png`, tContent: 0 },
      { index: 2, file: "002_00-05-00.png", path: `${summary.dir}/002_00-05-00.png`, tContent: 300 },
    ],
    ...overrides,
  };
}
