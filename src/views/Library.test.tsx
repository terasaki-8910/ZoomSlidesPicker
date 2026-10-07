import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { expectNoAxeViolations } from "../test/axe";
import { makeAppState, makeSessionDetail, makeSessionSummary, makeSettingsView } from "../test/fixtures";
import { createIpcMock } from "../test/mockIpc";

vi.mock("../ipc", () => createIpcMock());

import * as ipc from "../ipc";
import type { SessionDetail, SessionSummary } from "../ipc/types";
import Library from "./Library";

function setup(sessions: SessionSummary[], detailsByDir: Record<string, SessionDetail> = {}) {
  vi.mocked(ipc.getAppState).mockResolvedValue(makeAppState());
  vi.mocked(ipc.onAppState).mockResolvedValue(() => {});
  vi.mocked(ipc.getSettings).mockResolvedValue(makeSettingsView());
  vi.mocked(ipc.applyLanguage).mockResolvedValue(undefined);
  vi.mocked(ipc.onSettings).mockResolvedValue(() => {});
  vi.mocked(ipc.onLibraryChanged).mockResolvedValue(() => {});
  vi.mocked(ipc.listSessions).mockResolvedValue(sessions);
  vi.mocked(ipc.listWindows).mockResolvedValue([]);
  vi.mocked(ipc.thumbUrl).mockImplementation((path: string) => `data:,${path}`);
  vi.mocked(ipc.getSession).mockImplementation(async (dir: string): Promise<SessionDetail> => {
    if (detailsByDir[dir]) return detailsByDir[dir];
    const s = sessions.find((x) => x.dir === dir);
    if (!s) throw new Error(`unknown session: ${dir}`);
    return { ...s, captures: [] };
  });
}

function session(overrides: Partial<SessionSummary> = {}): SessionSummary {
  return makeSessionSummary(overrides);
}

afterEach(() => {
  vi.useRealTimers();
});

describe("Library", () => {
  // ---- C10: search + keyboard list navigation -----------------------------
  describe("search and list navigation (C10)", () => {
    function threeSessions(): SessionSummary[] {
      return [
        session({
          dir: "/d/s1",
          folder: "s1",
          title: "第1回授業",
          customTitle: "第1回授業",
          memo: "",
          sourceApp: "Safari",
          sourceTitle: "video.mp4",
          startedAt: "2026-10-07T12:42:00+09:00",
        }),
        session({
          dir: "/d/s2",
          folder: "s2",
          title: "Zoom ミーティング",
          customTitle: null,
          memo: "宿題の相談をした",
          sourceApp: "zoom.us",
          sourceTitle: "Zoom ミーティング",
          startedAt: "2026-10-03T10:30:00+09:00",
        }),
        session({
          dir: "/d/s3",
          folder: "s3",
          title: "lecture.mp4",
          customTitle: null,
          memo: "",
          sourceApp: null,
          sourceTitle: "lecture.mp4",
          sourceKind: "mp4",
          startedAt: "2026-10-01T09:00:00+09:00",
        }),
      ];
    }

    it("filters by custom title", async () => {
      setup(threeSessions());
      render(<Library />);
      const search = await screen.findByPlaceholderText("題名・日付・アプリ・メモで検索");
      fireEvent.change(search, { target: { value: "第1回" } });

      expect(await screen.findByRole("option", { name: /第1回授業/ })).toBeInTheDocument();
      expect(screen.queryByRole("option", { name: /Zoom ミーティング/ })).not.toBeInTheDocument();
      expect(screen.queryByRole("option", { name: /lecture\.mp4/ })).not.toBeInTheDocument();
    });

    it("filters by source title", async () => {
      setup(threeSessions());
      render(<Library />);
      const search = await screen.findByPlaceholderText("題名・日付・アプリ・メモで検索");
      fireEvent.change(search, { target: { value: "video" } });

      expect(await screen.findByRole("option", { name: /第1回授業/ })).toBeInTheDocument();
      expect(screen.getAllByRole("option")).toHaveLength(1);
    });

    it("filters by source app, case-insensitively", async () => {
      setup(threeSessions());
      render(<Library />);
      const search = await screen.findByPlaceholderText("題名・日付・アプリ・メモで検索");
      fireEvent.change(search, { target: { value: "ZOOM.US" } });

      expect(await screen.findByRole("option", { name: /Zoom ミーティング/ })).toBeInTheDocument();
      expect(screen.getAllByRole("option")).toHaveLength(1);
    });

    it("filters by memo", async () => {
      setup(threeSessions());
      render(<Library />);
      const search = await screen.findByPlaceholderText("題名・日付・アプリ・メモで検索");
      fireEvent.change(search, { target: { value: "宿題" } });

      expect(await screen.findByRole("option", { name: /Zoom ミーティング/ })).toBeInTheDocument();
      expect(screen.getAllByRole("option")).toHaveLength(1);
    });

    it("filters by date in each of the three search forms (M/D, M月D日, YYYY-MM-DD)", async () => {
      setup(threeSessions());
      render(<Library />);
      const search = await screen.findByPlaceholderText("題名・日付・アプリ・メモで検索");

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
      setup(threeSessions());
      render(<Library />);
      const search = await screen.findByPlaceholderText("題名・日付・アプリ・メモで検索");

      fireEvent.change(search, { target: { value: "zoom 10/3" } });
      expect(await screen.findByRole("option", { name: /Zoom ミーティング/ })).toBeInTheDocument();
      expect(screen.getAllByRole("option")).toHaveLength(1);

      fireEvent.change(search, { target: { value: "zoom 10/1" } });
      expect(await screen.findByText("見つかりませんでした。")).toBeInTheDocument();
      expect(screen.queryByRole("option")).not.toBeInTheDocument();
    });

    it("shows a no-hits message when nothing matches, and an empty-library message with no sessions at all", async () => {
      setup(threeSessions());
      render(<Library />);
      const search = await screen.findByPlaceholderText("題名・日付・アプリ・メモで検索");
      fireEvent.change(search, { target: { value: "nonexistent" } });
      expect(await screen.findByText("見つかりませんでした。")).toBeInTheDocument();

      setup([]);
      render(<Library />);
      expect(await screen.findByText("まだ記録はありません。")).toBeInTheDocument();
    });

    it("moves the listbox selection with ArrowDown/ArrowUp/Home/End, showing the chosen record on the right", async () => {
      setup(threeSessions());
      render(<Library />);

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
  });

  // ---- C11: rename in place + memo autosave --------------------------------
  describe("rename and memo (C11)", () => {
    it("renames on Enter (trims; empty clears to the source title), exactly once; Escape does not call it", async () => {
      const s = session({ title: "第1回授業", customTitle: "第1回授業", sourceTitle: "video.mp4" });
      setup([s], { [s.dir]: makeSessionDetail(s) });
      const user = userEvent.setup();
      render(<Library />);

      await screen.findByRole("heading", { name: "第1回授業" });
      await user.click(screen.getByRole("button", { name: "名前を変更" }));
      const input = screen.getByRole("textbox", { name: "名前を変更" });

      await user.clear(input);
      await user.type(input, "  改題した授業  {Enter}");
      expect(ipc.renameSession).toHaveBeenCalledTimes(1);
      expect(ipc.renameSession).toHaveBeenCalledWith(s.dir, "改題した授業");

      await user.click(await screen.findByRole("button", { name: "名前を変更" }));
      const input2 = screen.getByRole("textbox", { name: "名前を変更" });
      await user.type(input2, "何か入力{Escape}");
      expect(ipc.renameSession).toHaveBeenCalledTimes(1);

      await user.click(screen.getByRole("button", { name: "名前を変更" }));
      const input3 = screen.getByRole("textbox", { name: "名前を変更" });
      await user.clear(input3);
      await user.type(input3, "{Enter}");
      expect(ipc.renameSession).toHaveBeenCalledTimes(2);
      expect(ipc.renameSession).toHaveBeenLastCalledWith(s.dir, null);
    });

    it("saves the memo on blur, and never twice in a row for the same content", async () => {
      const s = session();
      setup([s], { [s.dir]: makeSessionDetail(s) });
      render(<Library />);

      const memo = await screen.findByLabelText("メモ");
      fireEvent.change(memo, { target: { value: "出席確認をした" } });
      fireEvent.blur(memo);
      await waitFor(() => expect(ipc.setSessionMemo).toHaveBeenCalledTimes(1));
      expect(ipc.setSessionMemo).toHaveBeenCalledWith(s.dir, "出席確認をした");
      expect(await screen.findByText("保存しました")).toBeInTheDocument();

      // Blurring again without changing the content must not call it a second time.
      fireEvent.blur(memo);
      expect(ipc.setSessionMemo).toHaveBeenCalledTimes(1);
    });

    it("saves the memo 600ms after typing stops", async () => {
      const s = session();
      setup([s], { [s.dir]: makeSessionDetail(s) });
      render(<Library />);
      const memo = await screen.findByLabelText("メモ");

      vi.useFakeTimers();
      fireEvent.change(memo, { target: { value: "復習内容を書いた" } });
      expect(ipc.setSessionMemo).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(600);
      expect(ipc.setSessionMemo).toHaveBeenCalledTimes(1);
      expect(ipc.setSessionMemo).toHaveBeenCalledWith(s.dir, "復習内容を書いた");
    });
  });

  // ---- C12: select images -> confirm -> trash_captures ---------------------
  describe("select images and move to Trash (C12)", () => {
    function setupWithCaptures() {
      const s = session({ count: 2 });
      const detail = makeSessionDetail(s);
      setup([s], { [s.dir]: detail });
      return { s, detail };
    }

    it("is disabled at 0 selected, and calls trash_captures exactly once on confirm", async () => {
      const { s, detail } = setupWithCaptures();
      const user = userEvent.setup();
      render(<Library />);

      await user.click(await screen.findByRole("button", { name: "画像を選ぶ" }));
      const trashButton = screen.getByRole("button", { name: /選んだ画像をごみ箱へ/ });
      expect(trashButton).toBeDisabled();

      const checkbox = screen.getAllByRole("checkbox")[0];
      await user.click(checkbox);
      expect(trashButton).toBeEnabled();

      await user.click(trashButton);
      expect(await screen.findByText("1 枚をごみ箱に移して、PDF を作り直しますか？")).toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: "ごみ箱に移す" }));
      expect(ipc.trashCaptures).toHaveBeenCalledTimes(1);
      expect(ipc.trashCaptures).toHaveBeenCalledWith(s.dir, [detail.captures[0].index]);
    });

    it("does not call trash_captures when cancelled, dismissed with Escape, or dismissed by an outside click", async () => {
      setupWithCaptures();
      const user = userEvent.setup();
      render(<Library />);

      await user.click(await screen.findByRole("button", { name: "画像を選ぶ" }));
      await user.click(screen.getAllByRole("checkbox")[0]);
      await user.click(screen.getByRole("button", { name: /選んだ画像をごみ箱へ/ }));
      const dialog1 = await screen.findByRole("dialog");

      // "やめる" is ambiguous here: the select-mode action row's own cancel button
      // (library.cancelSelect) has the same Japanese text as the dialog's (library.cancel).
      await user.click(within(dialog1).getByRole("button", { name: "やめる" }));
      await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
      expect(ipc.trashCaptures).not.toHaveBeenCalled();

      await user.click(screen.getByRole("button", { name: /選んだ画像をごみ箱へ/ }));
      await screen.findByRole("dialog");
      fireEvent.keyDown(window, { key: "Escape" });
      await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
      expect(ipc.trashCaptures).not.toHaveBeenCalled();

      await user.click(screen.getByRole("button", { name: /選んだ画像をごみ箱へ/ }));
      await screen.findByRole("dialog");
      fireEvent.click(document.querySelector(".modal-backdrop")!);
      await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
      expect(ipc.trashCaptures).not.toHaveBeenCalled();
    });
  });

  // ---- C13: merge duplicate slides (fold_session) --------------------------
  describe("merge duplicate slides (C13)", () => {
    it("enters select mode with exactly the returned indices selected, and shows the count", async () => {
      const s = session({ count: 2 });
      const detail = makeSessionDetail(s);
      setup([s], { [s.dir]: detail });
      vi.mocked(ipc.foldSession).mockResolvedValue([detail.captures[1].index]);
      const user = userEvent.setup();
      render(<Library />);

      await user.click(await screen.findByRole("button", { name: "同じスライドをまとめる…" }));
      expect(await screen.findByText("同じスライドとみなした 1 枚を選びました。確かめてから、ごみ箱に移してください。")).toBeInTheDocument();
      const checkboxes = screen.getAllByRole("checkbox");
      expect(checkboxes[0]).not.toBeChecked();
      expect(checkboxes[1]).toBeChecked();
      expect(ipc.trashCaptures).not.toHaveBeenCalled();
      expect(ipc.trashSession).not.toHaveBeenCalled();
    });

    it("shows a not-found note and does not enter select mode when nothing repeats", async () => {
      const s = session({ count: 2 });
      setup([s], { [s.dir]: makeSessionDetail(s) });
      vi.mocked(ipc.foldSession).mockResolvedValue([]);
      const user = userEvent.setup();
      render(<Library />);

      await user.click(await screen.findByRole("button", { name: "同じスライドをまとめる…" }));
      expect(await screen.findByText("同じスライドは見つかりませんでした。")).toBeInTheDocument();
      expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "画像を選ぶ" })).toBeInTheDocument();
    });
  });

  // ---- C14: a recording session's restrictions + the toolbar ---------------
  describe("a recording session is read-only for organizing (C14)", () => {
    it("hides the organizing actions and shows the explanation, while rename/memo/open-folder stay", async () => {
      const s = session({ recording: true, hasPdf: false });
      setup([s], { [s.dir]: makeSessionDetail(s) });
      render(<Library />);

      await screen.findByRole("heading", { name: s.title });
      expect(screen.getByRole("button", { name: "フォルダを開く" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "名前を変更" })).toBeInTheDocument();
      expect(screen.getByLabelText("メモ")).toBeInTheDocument();
      expect(screen.getByText("記録中のため、整理は停止してからできます。")).toBeInTheDocument();

      expect(screen.queryByRole("button", { name: "PDF を開く" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "PDF を作り直す" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "同じスライドをまとめる…" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "画像を選ぶ" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "削除…" })).not.toBeInTheDocument();
    });

    it("the top bar shows the recording state with Pause/Stop, and Paused with Resume", async () => {
      setup([session()]);
      vi.mocked(ipc.getAppState).mockResolvedValue(
        makeAppState({
          session: {
            state: "running",
            count: 3,
            dir: "/d/running",
            lastThumb: null,
            elapsedS: 5,
            blank: false,
            windowLost: false,
            sourceKind: "window",
            sourceTitle: "Zoom Meeting",
          },
        })
      );
      render(<Library />);

      expect(await screen.findByText("保存中：3 枚")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "一時停止" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "停止" })).toBeInTheDocument();
    });

    it("stopped: Start opens the window picker as a modal inside the window, dismissed by Escape and a backdrop click", async () => {
      setup([session()]);
      const user = userEvent.setup();
      render(<Library />);

      await user.click(await screen.findByRole("button", { name: "開始" }));
      await screen.findByRole("dialog");
      expect(screen.getByRole("heading", { name: "授業の窓を選ぶ" })).toBeInTheDocument();

      fireEvent.keyDown(window, { key: "Escape" });
      await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

      await user.click(screen.getByRole("button", { name: "開始" }));
      await screen.findByRole("dialog");
      fireEvent.click(document.querySelector(".modal-backdrop")!);
      await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    });
  });

  it("has no axe violations", async () => {
    const s = session({ count: 2 });
    setup([s], { [s.dir]: makeSessionDetail(s) });
    const { container } = render(<Library />);
    await screen.findByRole("heading", { name: s.title });
    await expectNoAxeViolations(container);
  });
});
