import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { expectNoAxeViolations } from "../test/axe";
import { makeAppState, makeSettingsView } from "../test/fixtures";
import { createIpcMock } from "../test/mockIpc";

vi.mock("../ipc", () => createIpcMock());

import * as ipc from "../ipc";
import type { SessionDetail, SessionSummary } from "../ipc/types";
import Library from "./Library";

function makeSessions(): SessionSummary[] {
  return [
    {
      id: "s1",
      title: "第1回授業",
      customTitle: "第1回授業",
      sourceApp: "Safari",
      sourceTitle: "video.mp4",
      sourceKind: "window",
      startedAt: "2026-10-07T12:42:00+09:00",
      endedAt: "2026-10-07T13:15:00+09:00",
      count: 10,
      hasPdf: true,
      recording: false,
      thumb: null,
    },
    {
      id: "s2",
      title: "Zoom ミーティング",
      customTitle: null,
      sourceApp: "zoom.us",
      sourceTitle: "Zoom ミーティング",
      sourceKind: "window",
      startedAt: "2026-10-03T10:30:00+09:00",
      endedAt: "2026-10-03T12:00:00+09:00",
      count: 5,
      hasPdf: false,
      recording: false,
      thumb: null,
    },
    {
      id: "s3",
      title: "lecture.mp4",
      customTitle: null,
      sourceApp: null,
      sourceTitle: "lecture.mp4",
      sourceKind: "mp4",
      startedAt: "2026-10-01T09:00:00+09:00",
      endedAt: "2026-10-01T10:00:00+09:00",
      count: 0,
      hasPdf: false,
      recording: false,
      thumb: null,
    },
  ];
}

function setup(sessions: SessionSummary[]) {
  vi.mocked(ipc.getAppState).mockResolvedValue(makeAppState());
  vi.mocked(ipc.getSettings).mockResolvedValue(makeSettingsView());
  vi.mocked(ipc.applyLanguage).mockResolvedValue(undefined);
  vi.mocked(ipc.onAppState).mockResolvedValue(() => {});
  vi.mocked(ipc.onSettings).mockResolvedValue(() => {});
  vi.mocked(ipc.listSessions).mockResolvedValue(sessions);
  vi.mocked(ipc.getSession).mockImplementation(async (id: string): Promise<SessionDetail> => {
    const s = sessions.find((x) => x.id === id);
    if (!s) throw new Error(`unknown session: ${id}`);
    return { ...s, captures: [] };
  });
}

describe("Library", () => {
  it("filters by custom name", async () => {
    setup(makeSessions());
    render(<Library />);

    const search = await screen.findByPlaceholderText("題名・日付・アプリで検索");
    fireEvent.change(search, { target: { value: "第1回" } });

    expect(await screen.findByRole("option", { name: /第1回授業/ })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /Zoom ミーティング/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /lecture\.mp4/ })).not.toBeInTheDocument();
  });

  it("filters by source title", async () => {
    setup(makeSessions());
    render(<Library />);

    const search = await screen.findByPlaceholderText("題名・日付・アプリで検索");
    fireEvent.change(search, { target: { value: "video" } });

    expect(await screen.findByRole("option", { name: /第1回授業/ })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /Zoom ミーティング/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /lecture\.mp4/ })).not.toBeInTheDocument();
  });

  it("filters by source app, case-insensitively", async () => {
    setup(makeSessions());
    render(<Library />);

    const search = await screen.findByPlaceholderText("題名・日付・アプリで検索");
    fireEvent.change(search, { target: { value: "ZOOM.US" } });

    expect(await screen.findByRole("option", { name: /Zoom ミーティング/ })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /第1回授業/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /lecture\.mp4/ })).not.toBeInTheDocument();
  });

  it("filters by date in each of the three search forms (M/D, M月D日, YYYY-MM-DD)", async () => {
    setup(makeSessions());
    render(<Library />);
    const search = await screen.findByPlaceholderText("題名・日付・アプリで検索");

    fireEvent.change(search, { target: { value: "10/7" } });
    expect(await screen.findByRole("option", { name: /第1回授業/ })).toBeInTheDocument();
    expect(screen.getAllByRole("option")).toHaveLength(1);

    fireEvent.change(search, { target: { value: "10月3日" } });
    expect(await screen.findByRole("option", { name: /Zoom ミーティング/ })).toBeInTheDocument();
    expect(screen.getAllByRole("option")).toHaveLength(1);

    fireEvent.change(search, { target: { value: "2026-10-01" } });
    expect(await screen.findByRole("option", { name: /lecture\.mp4/ })).toBeInTheDocument();
    expect(screen.getAllByRole("option")).toHaveLength(1);
  });

  it("matches only when every space-separated term matches (AND)", async () => {
    setup(makeSessions());
    render(<Library />);
    const search = await screen.findByPlaceholderText("題名・日付・アプリで検索");

    fireEvent.change(search, { target: { value: "zoom 10/3" } });
    expect(await screen.findByRole("option", { name: /Zoom ミーティング/ })).toBeInTheDocument();
    expect(screen.getAllByRole("option")).toHaveLength(1);

    fireEvent.change(search, { target: { value: "zoom 10/1" } });
    expect(await screen.findByText("見つかりませんでした。")).toBeInTheDocument();
    expect(screen.queryByRole("option")).not.toBeInTheDocument();
  });

  it("shows a no-hits message when nothing matches", async () => {
    setup(makeSessions());
    render(<Library />);
    const search = await screen.findByPlaceholderText("題名・日付・アプリで検索");

    fireEvent.change(search, { target: { value: "nonexistent" } });

    expect(await screen.findByText("見つかりませんでした。")).toBeInTheDocument();
  });

  it("shows an empty-library message and a Start button when there are no sessions at all", async () => {
    setup([]);
    render(<Library />);

    expect(await screen.findByText("まだ記録はありません。授業の窓を選んで開始すると、ここに並びます。")).toBeInTheDocument();
    // Two: the toolbar's own (stopped, independent of the list) and the empty state's.
    expect(screen.getAllByRole("button", { name: "開始" }).length).toBeGreaterThanOrEqual(1);
  });

  it("moves the listbox selection with ArrowDown/ArrowUp/Home/End in layout A", async () => {
    setup(makeSessions());
    render(<Library />);

    // The first session is selected by default, showing its detail.
    expect(await screen.findByRole("heading", { name: "第1回授業" })).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "ArrowDown" });
    expect(await screen.findByRole("heading", { name: "Zoom ミーティング" })).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "ArrowDown" });
    expect(await screen.findByRole("heading", { name: "lecture.mp4" })).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "ArrowUp" });
    expect(await screen.findByRole("heading", { name: "Zoom ミーティング" })).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "Home" });
    expect(await screen.findByRole("heading", { name: "第1回授業" })).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "End" });
    expect(await screen.findByRole("heading", { name: "lecture.mp4" })).toBeInTheDocument();
  });

  it("has no axe violations", async () => {
    setup(makeSessions());
    const { container } = render(<Library />);
    await screen.findByRole("heading", { name: "第1回授業" });
    await expectNoAxeViolations(container);
  });
});
