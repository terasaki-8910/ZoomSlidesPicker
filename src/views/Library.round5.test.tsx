// Round 5 (docs/impl/library.md > Round 5): range selection, selection actions, the
// export dialog, the session-row context menu, and the stopping state.
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoAxeViolations } from "../test/axe";
import { makeAppState, makeSessionDetail, makeSessionSummary, makeSettingsView } from "../test/fixtures";
import { createIpcMock } from "../test/mockIpc";

vi.mock("../ipc", () => createIpcMock());

import * as ipc from "../ipc";
import type { SessionDetail, SessionSummary } from "../ipc/types";
import Library from "./Library";

const MB = 1024 * 1024;

function captures(dir: string, n: number): SessionDetail["captures"] {
  return Array.from({ length: n }, (_, i) => ({
    index: i + 1,
    file: `${String(i + 1).padStart(3, "0")}_00-00-0${i}.png`,
    path: `${dir}/${i + 1}.png`,
    tContent: i,
  }));
}

function setup(sessions: SessionSummary[], details: Record<string, SessionDetail> = {}) {
  vi.mocked(ipc.getAppState).mockResolvedValue(makeAppState());
  vi.mocked(ipc.onAppState).mockResolvedValue(() => {});
  vi.mocked(ipc.getSettings).mockResolvedValue(makeSettingsView());
  vi.mocked(ipc.applyLanguage).mockResolvedValue(undefined);
  vi.mocked(ipc.onSettings).mockResolvedValue(() => {});
  vi.mocked(ipc.onLibraryChanged).mockResolvedValue(() => {});
  vi.mocked(ipc.listSessions).mockResolvedValue(sessions);
  vi.mocked(ipc.listWindows).mockResolvedValue([]);
  vi.mocked(ipc.thumbUrl).mockImplementation((path: string) => `data:,${path}`);
  vi.mocked(ipc.getSession).mockImplementation(async (dir: string) => {
    if (details[dir]) return details[dir];
    const s = sessions.find((x) => x.dir === dir);
    if (!s) throw new Error(`unknown session: ${dir}`);
    return { ...s, captures: [] };
  });
  vi.mocked(ipc.estimatePdf).mockResolvedValue([
    { quality: "lossless", bytes: 73 * MB },
    { quality: "standard", bytes: 21 * MB },
    { quality: "light", bytes: 9.5 * MB },
  ]);
  vi.mocked(ipc.exportPdf).mockResolvedValue({ path: "/d/x.pdf", pages: 181, bytes: 72.8 * MB });
  vi.mocked(ipc.openSessionFolder).mockResolvedValue(undefined);
  vi.mocked(ipc.trashCaptures).mockResolvedValue(0);
}

function withFive() {
  const s = makeSessionSummary({ dir: "/d/s1", count: 5, startedAt: "2026-10-13T10:00:00+09:00" });
  const detail: SessionDetail = { ...makeSessionDetail(s), captures: captures(s.dir, 5) };
  setup([s], { [s.dir]: detail });
  return { s, detail };
}

async function enterSelectMode(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole("button", { name: "画像を選ぶ" }));
  return screen.getAllByRole("checkbox");
}

function checkedIndices(): number[] {
  return screen
    .getAllByRole("checkbox")
    .map((c, i) => ((c as HTMLInputElement).checked ? i + 1 : 0))
    .filter(Boolean);
}

describe("selection", () => {
  it("click toggles, Shift+click adds the inclusive range from the last click, Cmd/Ctrl+click toggles", async () => {
    withFive();
    const user = userEvent.setup();
    render(<Library />);
    const boxes = await enterSelectMode(user);

    await user.click(boxes[1]);
    expect(checkedIndices()).toEqual([2]);
    await user.keyboard("{Shift>}");
    await user.click(boxes[4]);
    await user.keyboard("{/Shift}");
    expect(checkedIndices()).toEqual([2, 3, 4, 5]);

    // Cmd+click toggles one without touching the others; so does Ctrl+click.
    await user.keyboard("{Meta>}");
    await user.click(boxes[2]);
    await user.keyboard("{/Meta}");
    expect(checkedIndices()).toEqual([2, 4, 5]);
    await user.keyboard("{Control>}");
    await user.click(boxes[0]);
    await user.keyboard("{/Control}");
    expect(checkedIndices()).toEqual([1, 2, 4, 5]);
  });

  it("Shift+click also works upwards from the last click", async () => {
    withFive();
    const user = userEvent.setup();
    render(<Library />);
    const boxes = await enterSelectMode(user);

    await user.click(boxes[3]);
    await user.keyboard("{Shift>}");
    await user.click(boxes[1]);
    await user.keyboard("{/Shift}");
    expect(checkedIndices()).toEqual([2, 3, 4]);
  });

  it("すべて選ぶ selects everything and then becomes 選択を解除", async () => {
    withFive();
    const user = userEvent.setup();
    render(<Library />);
    await enterSelectMode(user);

    await user.click(screen.getByRole("button", { name: "すべて選ぶ" }));
    expect(checkedIndices()).toEqual([1, 2, 3, 4, 5]);
    await user.click(screen.getByRole("button", { name: "選択を解除" }));
    expect(checkedIndices()).toEqual([]);
    expect(screen.getByRole("button", { name: "すべて選ぶ" })).toBeInTheDocument();
  });

  it("選んだ画像以外をごみ箱へ confirms with the unselected count and trashes the complement", async () => {
    const { s } = withFive();
    const user = userEvent.setup();
    render(<Library />);
    const boxes = await enterSelectMode(user);

    const others = screen.getByRole("button", { name: /選んだ画像以外をごみ箱へ/ });
    expect(others).toBeDisabled();
    await user.click(boxes[1]);
    await user.click(boxes[3]);
    expect(screen.getByRole("button", { name: "選んだ画像以外をごみ箱へ（3 枚）" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "選んだ画像をごみ箱へ（2 枚）" })).toBeEnabled();

    await user.click(others);
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("選んでいない 3 枚をごみ箱に移して、PDF を作り直しますか？")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "ごみ箱に移す" }));

    expect(ipc.trashCaptures).toHaveBeenCalledTimes(1);
    expect(ipc.trashCaptures).toHaveBeenCalledWith(s.dir, [1, 3, 5]);
  });
});

describe("export dialog", () => {
  it("shows the three estimates with 標準 preselected, and exports the whole session at the chosen quality", async () => {
    const { s } = withFive();
    let release: () => void = () => {};
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const estimates = await vi.mocked(ipc.estimatePdf)(s.dir, null);
    vi.mocked(ipc.estimatePdf).mockImplementation(async () => {
      await gate;
      return estimates;
    });
    const user = userEvent.setup();
    render(<Library />);

    await user.click(await screen.findByRole("button", { name: "PDF を書き出す…" }));
    const dialog = await screen.findByRole("dialog", { name: "PDF を書き出す" });
    expect(within(dialog).getAllByText("見積もっています…")).toHaveLength(3);
    release();
    expect(await within(dialog).findByText("約 73 MB")).toBeInTheDocument();
    expect(within(dialog).getByText("約 21 MB")).toBeInTheDocument();
    expect(within(dialog).getByText("約 9.5 MB")).toBeInTheDocument();
    expect(within(dialog).getByRole("radio", { name: /標準/ })).toBeChecked();
    expect(within(dialog).getByText("slides.pdf を作り直します。")).toBeInTheDocument();
    expect(ipc.estimatePdf).toHaveBeenCalledWith(s.dir, null);

    await user.click(within(dialog).getByRole("radio", { name: /軽量（幅 1920px）/ }));
    await user.click(within(dialog).getByRole("button", { name: "書き出す" }));

    expect(ipc.exportPdf).toHaveBeenCalledWith(s.dir, "light", null);
    expect(await screen.findByText(/書き出しました（181 ページ、72\.8 MB）。/)).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: "フォルダで表示" }));
    expect(ipc.openSessionFolder).toHaveBeenCalledWith(s.dir);
  });

  it("exports only the selected images, in index order, and names the file", async () => {
    const { s } = withFive();
    const user = userEvent.setup();
    render(<Library />);
    const boxes = await enterSelectMode(user);
    await user.click(boxes[3]);
    await user.click(boxes[1]);

    await user.click(screen.getByRole("button", { name: "選んだ画像で PDF を書き出す…" }));
    const dialog = await screen.findByRole("dialog");
    expect(ipc.estimatePdf).toHaveBeenCalledWith(s.dir, [2, 4]);
    expect(within(dialog).getByText("「選んだ画像 2026-10-13.pdf」としてフォルダに保存します。")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("radio", { name: /高画質/ }));
    await user.click(within(dialog).getByRole("button", { name: "書き出す" }));

    expect(ipc.exportPdf).toHaveBeenCalledWith(s.dir, "lossless", [2, 4]);
  });

  it("closes on Escape and on the cancel button without exporting", async () => {
    withFive();
    const user = userEvent.setup();
    render(<Library />);

    await user.click(await screen.findByRole("button", { name: "PDF を書き出す…" }));
    await screen.findByRole("dialog");
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    await user.click(screen.getByRole("button", { name: "PDF を書き出す…" }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "やめる" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(ipc.exportPdf).not.toHaveBeenCalled();
  });

  it("has no axe violations with the dialog open", async () => {
    withFive();
    const user = userEvent.setup();
    const { baseElement } = render(<Library />);
    await user.click(await screen.findByRole("button", { name: "PDF を書き出す…" }));
    await screen.findByText("約 73 MB");
    await expectNoAxeViolations(baseElement);
  });
});

describe("session row context menu", () => {
  function two() {
    const a = makeSessionSummary({ dir: "/d/a", title: "A 授業", customTitle: "A 授業", count: 5, hasPdf: true, startedAt: "2026-10-13T10:00:00+09:00" });
    const b = makeSessionSummary({ dir: "/d/b", title: "B 録画中", customTitle: "B 録画中", count: 2, hasPdf: false, recording: true, startedAt: "2026-10-12T10:00:00+09:00" });
    setup([a, b], { [a.dir]: { ...makeSessionDetail(a), captures: captures(a.dir, 5) } });
    return { a, b };
  }

  it("opens on right click with the full item list, and Escape closes it", async () => {
    two();
    const user = userEvent.setup();
    render(<Library />);
    const row = await screen.findByRole("option", { name: /A 授業/ });

    fireEvent.contextMenu(row, { clientX: 100, clientY: 120 });
    const menu = await screen.findByRole("menu");
    expect(within(menu).getAllByRole("menuitem").map((i) => i.textContent)).toEqual([
      "フォルダを開く",
      "PDF を開く",
      "PDF を書き出す…",
      "名前を変更",
      "ごみ箱へ…",
    ]);
    expect(within(menu).getByRole("menuitem", { name: "フォルダを開く" })).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(within(menu).getByRole("menuitem", { name: "PDF を開く" })).toHaveFocus();
    await user.keyboard("{ArrowUp}{ArrowUp}");
    expect(within(menu).getByRole("menuitem", { name: "ごみ箱へ…" })).toHaveFocus();

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("opens on Shift+F10 and the ContextMenu key for the focused list", async () => {
    two();
    const user = userEvent.setup();
    render(<Library />);
    const list = await screen.findByRole("listbox");
    list.focus();

    await user.keyboard("{Shift>}{F10}{/Shift}");
    expect(await screen.findByRole("menu")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();

    list.focus();
    fireEvent.keyDown(list, { key: "ContextMenu" });
    expect(await screen.findByRole("menu")).toBeInTheDocument();
  });

  it("offers only the folder and rename for a recording session", async () => {
    two();
    render(<Library />);
    fireEvent.contextMenu(await screen.findByRole("option", { name: /B 録画中/ }), { clientX: 50, clientY: 50 });
    const menu = await screen.findByRole("menu");
    expect(within(menu).getAllByRole("menuitem").map((i) => i.textContent)).toEqual(["フォルダを開く", "名前を変更"]);
  });

  it("closes on a click outside, and Enter on an item runs it (PDF を書き出す… opens the dialog for that row)", async () => {
    const { a } = two();
    const user = userEvent.setup();
    render(<Library />);
    const row = await screen.findByRole("option", { name: /A 授業/ });

    fireEvent.contextMenu(row, { clientX: 10, clientY: 10 });
    await screen.findByRole("menu");
    fireEvent.mouseDown(document.body);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();

    fireEvent.contextMenu(row, { clientX: 10, clientY: 10 });
    await screen.findByRole("menu");
    await user.keyboard("{ArrowDown}{ArrowDown}{Enter}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(await screen.findByRole("dialog", { name: "PDF を書き出す" })).toBeInTheDocument();
    expect(ipc.estimatePdf).toHaveBeenCalledWith(a.dir, null);
  });

  it("名前を変更 on another row selects it and starts the rename", async () => {
    const { a } = two();
    const user = userEvent.setup();
    render(<Library />);
    await screen.findByRole("heading", { name: a.title });

    fireEvent.contextMenu(await screen.findByRole("option", { name: /B 録画中/ }), { clientX: 10, clientY: 10 });
    await user.click(await screen.findByRole("menuitem", { name: "名前を変更" }));
    const input = await screen.findByRole("textbox", { name: "名前を変更" });
    expect(input).toHaveValue("B 録画中");
  });

  it("ごみ箱へ… opens the existing confirm", async () => {
    two();
    const user = userEvent.setup();
    render(<Library />);
    fireEvent.contextMenu(await screen.findByRole("option", { name: /A 授業/ }), { clientX: 10, clientY: 10 });
    await user.click(await screen.findByRole("menuitem", { name: "ごみ箱へ…" }));
    expect(await screen.findByText(/「A 授業」をごみ箱に移しますか/)).toBeInTheDocument();
  });

  it("has no axe violations with the menu open", async () => {
    two();
    const { baseElement } = render(<Library />);
    fireEvent.contextMenu(await screen.findByRole("option", { name: /A 授業/ }), { clientX: 10, clientY: 10 });
    await screen.findByRole("menu");
    await expectNoAxeViolations(baseElement);
  });
});

describe("stopping", () => {
  it("the top bar shows 'PDF を作成中…' and no session controls", async () => {
    setup([makeSessionSummary()]);
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
          stopping: true,
          sourceKind: "window",
          sourceTitle: "Zoom Meeting",
        },
      })
    );
    render(<Library />);

    expect(await screen.findByText("PDF を作成中…")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "停止" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "一時停止" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "開始" })).not.toBeInTheDocument();
  });
});
