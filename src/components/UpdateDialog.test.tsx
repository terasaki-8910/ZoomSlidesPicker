// ACCEPTANCE C21 (dialog states and calls) -- docs/impl/updates.md > UI.
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { expectNoAxeViolations } from "../test/axe";
import { makeAppState, makeSettingsView } from "../test/fixtures";
import { createIpcMock, createListenerMock } from "../test/mockIpc";

vi.mock("../ipc", () => createIpcMock());

import * as ipc from "../ipc";
import type { AppState, UpdateProgress } from "../ipc/types";
import UpdateDialog from "./UpdateDialog";

const info = { version: "0.2.0", current: "0.1.6" };

function setup(state: AppState = makeAppState()) {
  const appListener = createListenerMock<AppState>();
  const progressListener = createListenerMock<UpdateProgress>();
  vi.mocked(ipc.getAppState).mockResolvedValue(state);
  vi.mocked(ipc.onAppState).mockImplementation(appListener.fn);
  vi.mocked(ipc.onUpdateProgress).mockImplementation(progressListener.fn);
  vi.mocked(ipc.getSettings).mockResolvedValue(makeSettingsView());
  vi.mocked(ipc.onSettings).mockResolvedValue(() => {});
  vi.mocked(ipc.applyLanguage).mockResolvedValue(undefined);
  vi.mocked(ipc.answerUpdate).mockResolvedValue(undefined);
  return { appListener, progressListener };
}

function runningState(): AppState {
  return makeAppState({
    session: {
      state: "running",
      count: 3,
      dir: "/d/s",
      lastThumb: null,
      elapsedS: 10,
      blank: false,
      windowLost: false,
      sourceKind: "window",
      sourceTitle: "Zoom",
      stopping: false,
    },
  });
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("UpdateDialog offer", () => {
  it("shows both versions and the three buttons in order: skip, later, update now", async () => {
    setup();
    render(<UpdateDialog info={info} answerLater onClose={() => {}} />);
    expect(await screen.findByRole("heading", { name: "新しい版があります" })).toBeInTheDocument();
    expect(screen.getByText("ZoomSlidesPicker 0.2.0 が出ています。")).toBeInTheDocument();
    expect(screen.getByText("いまの版は 0.1.6 です。")).toBeInTheDocument();
    const names = screen.getAllByRole("button").map((b) => b.textContent);
    expect(names).toEqual(["この版をスキップ", "後で", "今すぐ更新"]);
    await waitFor(() => expect(screen.getByRole("button", { name: "今すぐ更新" })).toBeEnabled());
  });

  it("Later, Escape and a backdrop click answer later; Skip answers skip", async () => {
    const user = userEvent.setup();
    for (const how of ["button", "escape", "backdrop"] as const) {
      setup();
      const onClose = vi.fn();
      const { unmount } = render(<UpdateDialog info={info} answerLater onClose={onClose} />);
      await screen.findByRole("dialog");
      if (how === "button") await user.click(screen.getByRole("button", { name: "後で" }));
      else if (how === "escape") await user.keyboard("{Escape}");
      else await user.click(document.querySelector(".modal-backdrop") as Element);
      expect(ipc.answerUpdate).toHaveBeenCalledWith("later");
      expect(onClose).toHaveBeenCalledTimes(1);
      unmount();
      vi.clearAllMocks();
    }
    setup();
    const onClose = vi.fn();
    render(<UpdateDialog info={info} answerLater onClose={onClose} />);
    await user.click(await screen.findByRole("button", { name: "この版をスキップ" }));
    expect(ipc.answerUpdate).toHaveBeenCalledWith("skip");
    expect(ipc.answerUpdate).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("without answerLater (Settings), Later only closes", async () => {
    setup();
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(<UpdateDialog info={info} answerLater={false} onClose={onClose} />);
    await user.click(await screen.findByRole("button", { name: "後で" }));
    expect(ipc.answerUpdate).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("Update now calls install_update", async () => {
    setup();
    vi.mocked(ipc.installUpdate).mockReturnValue(new Promise(() => {}));
    const user = userEvent.setup();
    render(<UpdateDialog info={info} answerLater onClose={() => {}} />);
    const button = await screen.findByRole("button", { name: "今すぐ更新" });
    await waitFor(() => expect(button).toBeEnabled());
    await user.click(button);
    expect(ipc.installUpdate).toHaveBeenCalledTimes(1);
    expect(await screen.findByRole("heading", { name: "更新しています" })).toBeInTheDocument();
  });

  it("is English when the language is English", async () => {
    setup();
    vi.mocked(ipc.getSettings).mockResolvedValue(makeSettingsView({ language: "en", effectiveLanguage: "en" }));
    render(<UpdateDialog info={info} answerLater onClose={() => {}} />);
    expect(await screen.findByRole("heading", { name: "A new version is available" })).toBeInTheDocument();
    expect(screen.getByText("You have 0.1.6.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Skip this version" })).toBeInTheDocument();
  });
});

describe("UpdateDialog busy", () => {
  it("disables Update now with the reason while a session is active, and re-enables when it ends", async () => {
    const { appListener } = setup(runningState());
    render(<UpdateDialog info={info} answerLater onClose={() => {}} />);
    const button = await screen.findByRole("button", { name: "今すぐ更新" });
    await waitFor(() => expect(button).toBeDisabled());
    expect(screen.getByText("記録中は更新できません。停止すると更新できます。")).toBeInTheDocument();

    act(() => appListener.emit(makeAppState()));
    await waitFor(() => expect(button).toBeEnabled());
    expect(screen.queryByText("記録中は更新できません。停止すると更新できます。")).not.toBeInTheDocument();

    act(() => appListener.emit(makeAppState({ videoProgress: { done: 1, total: 10 } })));
    await waitFor(() => expect(button).toBeDisabled());
  });
});

describe("UpdateDialog installing", () => {
  async function startInstall() {
    const ctx = setup();
    vi.mocked(ipc.installUpdate).mockReturnValue(new Promise(() => {}));
    const onClose = vi.fn();
    const user = userEvent.setup();
    const view = render(<UpdateDialog info={info} answerLater onClose={onClose} />);
    const button = await screen.findByRole("button", { name: "今すぐ更新" });
    await waitFor(() => expect(button).toBeEnabled());
    await user.click(button);
    await screen.findByRole("heading", { name: "更新しています" });
    return { ...ctx, onClose, user, view };
  }

  it("shows download percent, then installing, with the restart note and no buttons", async () => {
    const { progressListener } = await startInstall();
    expect(screen.getByText("ダウンロード中…")).toBeInTheDocument();
    expect(screen.getByText("終わるとアプリが再起動します。")).toBeInTheDocument();
    expect(screen.queryAllByRole("button")).toHaveLength(0);

    act(() => progressListener.emit({ phase: "downloading", downloaded: 25, total: 100 }));
    expect(screen.getByText("ダウンロード中… 25%")).toBeInTheDocument();

    act(() => progressListener.emit({ phase: "downloading", downloaded: 25, total: null }));
    expect(screen.getByText("ダウンロード中…")).toBeInTheDocument();

    act(() => progressListener.emit({ phase: "installing", downloaded: 100, total: 100 }));
    expect(screen.getByText("インストール中…")).toBeInTheDocument();
  });

  it("is not dismissed by Escape or a backdrop click", async () => {
    const { onClose, user } = await startInstall();
    await user.keyboard("{Escape}");
    await user.click(document.querySelector(".modal-backdrop") as Element);
    expect(onClose).not.toHaveBeenCalled();
    expect(ipc.answerUpdate).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "更新しています" })).toBeInTheDocument();
  });
});

describe("UpdateDialog failed", () => {
  it("shows the message and a Close button", async () => {
    setup();
    vi.mocked(ipc.installUpdate).mockRejectedValue(new Error("署名が合いません"));
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(<UpdateDialog info={info} answerLater onClose={onClose} />);
    const button = await screen.findByRole("button", { name: "今すぐ更新" });
    await waitFor(() => expect(button).toBeEnabled());
    await user.click(button);
    expect(await screen.findByText("更新できませんでした：署名が合いません")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "閉じる" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

describe("UpdateDialog accessibility", () => {
  it("has no axe violations in the offer, busy and installing states", async () => {
    setup();
    const offer = render(<UpdateDialog info={info} answerLater onClose={() => {}} />);
    await screen.findByRole("dialog");
    await expectNoAxeViolations(offer.container.ownerDocument.body);
    offer.unmount();

    setup(runningState());
    const busy = render(<UpdateDialog info={info} answerLater onClose={() => {}} />);
    await screen.findByText("記録中は更新できません。停止すると更新できます。");
    await expectNoAxeViolations(busy.container.ownerDocument.body);
    busy.unmount();

    setup();
    vi.mocked(ipc.installUpdate).mockReturnValue(new Promise(() => {}));
    const user = userEvent.setup();
    const installing = render(<UpdateDialog info={info} answerLater onClose={() => {}} />);
    const button = await screen.findByRole("button", { name: "今すぐ更新" });
    await waitFor(() => expect(button).toBeEnabled());
    await user.click(button);
    await screen.findByRole("heading", { name: "更新しています" });
    await expectNoAxeViolations(installing.container.ownerDocument.body);
  });
});
