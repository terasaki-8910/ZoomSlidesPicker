import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoAxeViolations } from "../test/axe";
import { makeAppState, makeSettingsView } from "../test/fixtures";
import { createIpcMock } from "../test/mockIpc";

vi.mock("../ipc", () => createIpcMock());

import * as ipc from "../ipc";
import Panel from "./Panel";

function setup(appState = makeAppState(), settings = makeSettingsView()) {
  vi.mocked(ipc.getAppState).mockResolvedValue(appState);
  vi.mocked(ipc.getSettings).mockResolvedValue(settings);
  vi.mocked(ipc.applyLanguage).mockResolvedValue(undefined);
  vi.mocked(ipc.onAppState).mockResolvedValue(() => {});
  vi.mocked(ipc.onSettings).mockResolvedValue(() => {});
}

describe("Panel", () => {
  it("stopped with no last session shows only Start (C3)", async () => {
    setup(makeAppState());
    render(<Panel />);

    expect(await screen.findByText("停止中")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "開始" })).toBeEnabled();
    expect(screen.queryByRole("button", { name: "保存先を開く" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "PDF を作り直す" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "停止" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "一時停止" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "今すぐ保存" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "直前の保存を取り消す" })).not.toBeInTheDocument();
  });

  it("stopped with a last session shows open-folder and rebuild, which call their commands (C3/C8)", async () => {
    setup(makeAppState({ lastSession: { dir: "d", count: 5, hasPdf: true } }));
    vi.mocked(ipc.rebuildPdf).mockResolvedValue(3);
    const user = userEvent.setup();
    render(<Panel />);

    await user.click(await screen.findByRole("button", { name: "保存先を開く" }));
    expect(ipc.openFolder).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: "PDF を作り直す" }));
    expect(ipc.rebuildPdf).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("3 ページで作り直しました。")).toBeInTheDocument();
  });

  it("shows the rebuild error inline when it fails (C8)", async () => {
    setup(makeAppState({ lastSession: { dir: "d", count: 0, hasPdf: false } }));
    vi.mocked(ipc.rebuildPdf).mockRejectedValue(new Error("no images"));
    const user = userEvent.setup();
    render(<Panel />);

    await user.click(await screen.findByRole("button", { name: "PDF を作り直す" }));
    expect(await screen.findByText("no images")).toBeInTheDocument();
  });

  it("does not show last-session actions without one (C8)", async () => {
    setup(makeAppState({ lastSession: null }));
    render(<Panel />);

    await screen.findByRole("button", { name: "開始" });
    expect(screen.queryByRole("button", { name: "保存先を開く" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "PDF を作り直す" })).not.toBeInTheDocument();
  });

  it("running shows the count, elapsed time, thumbnail, pause, save-now, undo and stop (C3)", async () => {
    setup(
      makeAppState({
        session: {
          state: "running",
          count: 12,
          dir: "d",
          lastThumb: "data:image/svg+xml,thumb",
          elapsedS: 754,
          blank: false,
          windowLost: false,
          sourceKind: "window",
          sourceTitle: "Zoom Meeting",
        },
      })
    );
    render(<Panel />);

    expect(await screen.findByText("保存中：12 枚")).toBeInTheDocument();
    expect(screen.getByText("0:12:34")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "保存先を開く" })).toBeInTheDocument(); // the thumbnail button
    expect(screen.getByRole("button", { name: "停止" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "一時停止" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "今すぐ保存" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "直前の保存を取り消す" })).toBeInTheDocument();
  });

  it("paused is shown distinctly, with Resume instead of Pause (C3)", async () => {
    setup(
      makeAppState({
        session: {
          state: "paused",
          count: 12,
          dir: "d",
          lastThumb: null,
          elapsedS: 10,
          blank: false,
          windowLost: false,
          sourceKind: "window",
          sourceTitle: "Zoom Meeting",
        },
      })
    );
    render(<Panel />);

    expect(await screen.findByText("一時停止中：12 枚")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "再開" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "一時停止" })).not.toBeInTheDocument();
  });

  it("shows the permission notice and disables Start when permission is not granted (C7)", async () => {
    setup(makeAppState({ permission: { granted: false, platform: "macos" } }));
    render(<Panel />);

    expect(await screen.findByText(/画面収録の許可が必要です/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "開始" })).toBeDisabled();
  });

  it("picks a window with arrow keys + Enter and begins the region (C2)", async () => {
    setup(makeAppState());
    vi.mocked(ipc.listWindows).mockResolvedValue([
      { id: 1, app: "zoom.us", title: "Zoom Meeting", width: 1280, height: 720, thumb: "data:image/svg+xml,a" },
      { id: 2, app: "Microsoft Edge", title: "Lecture video", width: 1440, height: 900, thumb: "data:image/svg+xml,b" },
    ]);
    const user = userEvent.setup();
    render(<Panel />);

    await user.click(await screen.findByRole("button", { name: "開始" }));
    const listbox = await screen.findByRole("listbox");
    await waitFor(() => expect(within(listbox).getAllByRole("option")).toHaveLength(2));

    fireEvent.keyDown(window, { key: "ArrowDown" });
    fireEvent.keyDown(window, { key: "Enter" });

    expect(ipc.beginRegion).toHaveBeenCalledWith(2);
  });

  it("Esc inside the picker goes back instead of closing the window (C2/C9)", async () => {
    setup(makeAppState());
    vi.mocked(ipc.listWindows).mockResolvedValue([
      { id: 1, app: "zoom.us", title: "Zoom Meeting", width: 1280, height: 720, thumb: "data:image/svg+xml,a" },
    ]);
    const user = userEvent.setup();
    render(<Panel />);

    await user.click(await screen.findByRole("button", { name: "開始" }));
    await screen.findByRole("listbox");

    fireEvent.keyDown(window, { key: "Escape" });

    expect(await screen.findByRole("button", { name: "開始" })).toBeInTheDocument();
    expect(ipc.closeWindow).not.toHaveBeenCalled();
  });

  it("Esc closes the window when the picker is not open (C9)", async () => {
    setup(makeAppState());
    render(<Panel />);

    await screen.findByRole("button", { name: "開始" });
    fireEvent.keyDown(window, { key: "Escape" });

    expect(ipc.closeWindow).toHaveBeenCalledTimes(1);
  });

  it("Cmd+, opens settings on macOS (C4)", async () => {
    vi.spyOn(window.navigator, "platform", "get").mockReturnValue("MacIntel");
    setup(makeAppState());
    render(<Panel />);

    await screen.findByRole("button", { name: "開始" });
    fireEvent.keyDown(window, { key: ",", metaKey: true });

    expect(ipc.openSettings).toHaveBeenCalledTimes(1);
  });

  it("Ctrl+, opens settings on Windows/Linux (C4)", async () => {
    vi.spyOn(window.navigator, "platform", "get").mockReturnValue("Win32");
    setup(makeAppState());
    render(<Panel />);

    await screen.findByRole("button", { name: "開始" });
    fireEvent.keyDown(window, { key: ",", ctrlKey: true });

    expect(ipc.openSettings).toHaveBeenCalledTimes(1);
  });

  it("has no axe violations when stopped (C5)", async () => {
    setup(makeAppState());
    const { container } = render(<Panel />);
    await screen.findByRole("button", { name: "開始" });
    await expectNoAxeViolations(container);
  });

  it("has no axe violations when running (C5)", async () => {
    setup(
      makeAppState({
        session: {
          state: "running",
          count: 1,
          dir: "d",
          lastThumb: "data:image/svg+xml,thumb",
          elapsedS: 1,
          blank: false,
          windowLost: false,
          sourceKind: "window",
          sourceTitle: "Zoom Meeting",
        },
      })
    );
    const { container } = render(<Panel />);
    await screen.findByRole("button", { name: "停止" });
    await expectNoAxeViolations(container);
  });

  it("renders in English when effectiveLanguage is en, and calls apply_language (C1)", async () => {
    setup(makeAppState(), makeSettingsView({ effectiveLanguage: "en" }));
    render(<Panel />);

    expect(await screen.findByText("Stopped")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument();
    await waitFor(() => expect(ipc.applyLanguage).toHaveBeenCalled());
  });
});
