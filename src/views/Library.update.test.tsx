// ACCEPTANCE C21: the library window's launch offer (docs/impl/updates.md > UI > Where it shows).
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { makeAppState, makeSettingsView } from "../test/fixtures";
import { createIpcMock, createListenerMock } from "../test/mockIpc";

vi.mock("../ipc", () => createIpcMock());

import * as ipc from "../ipc";
import type { UpdateInfo } from "../ipc/types";
import Library from "./Library";

const info: UpdateInfo = { version: "0.2.0", current: "0.1.6" };

function setup(offer: UpdateInfo | null) {
  const offerListener = createListenerMock<UpdateInfo>();
  vi.mocked(ipc.getAppState).mockResolvedValue(makeAppState());
  vi.mocked(ipc.onAppState).mockResolvedValue(() => {});
  vi.mocked(ipc.getSettings).mockResolvedValue(makeSettingsView());
  vi.mocked(ipc.applyLanguage).mockResolvedValue(undefined);
  vi.mocked(ipc.onSettings).mockResolvedValue(() => {});
  vi.mocked(ipc.onLibraryChanged).mockResolvedValue(() => {});
  vi.mocked(ipc.listSessions).mockResolvedValue([]);
  vi.mocked(ipc.listWindows).mockResolvedValue([]);
  vi.mocked(ipc.getUpdateOffer).mockResolvedValue(offer);
  vi.mocked(ipc.onUpdateOffer).mockImplementation(offerListener.fn);
  vi.mocked(ipc.answerUpdate).mockResolvedValue(undefined);
  return { offerListener };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("Library update offer", () => {
  it("opens the dialog from get_update_offer on mount", async () => {
    setup(info);
    render(<Library />);
    expect(await screen.findByRole("heading", { name: "新しい版があります" })).toBeInTheDocument();
    expect(screen.getByText("ZoomSlidesPicker 0.2.0 が出ています。")).toBeInTheDocument();
  });

  it("opens no dialog when there is no offer", async () => {
    setup(null);
    render(<Library />);
    await waitFor(() => expect(ipc.getUpdateOffer).toHaveBeenCalled());
    expect(screen.queryByRole("heading", { name: "新しい版があります" })).not.toBeInTheDocument();
  });

  it("opens the dialog when update-offer arrives, and Later answers later and closes it", async () => {
    const { offerListener } = setup(null);
    const user = userEvent.setup();
    render(<Library />);
    await waitFor(() => expect(ipc.onUpdateOffer).toHaveBeenCalled());
    act(() => offerListener.emit(info));
    expect(await screen.findByRole("heading", { name: "新しい版があります" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "後で" }));
    expect(ipc.answerUpdate).toHaveBeenCalledWith("later");
    expect(screen.queryByRole("heading", { name: "新しい版があります" })).not.toBeInTheDocument();
  });

  it("Skip answers skip; Escape answers later", async () => {
    setup(info);
    const user = userEvent.setup();
    const first = render(<Library />);
    await user.click(await screen.findByRole("button", { name: "この版をスキップ" }));
    expect(ipc.answerUpdate).toHaveBeenCalledWith("skip");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    first.unmount();

    vi.clearAllMocks();
    setup(info);
    render(<Library />);
    await screen.findByRole("heading", { name: "新しい版があります" });
    await user.keyboard("{Escape}");
    expect(ipc.answerUpdate).toHaveBeenCalledWith("later");
  });

  it("Update now calls install_update; a rejection shows the failed state", async () => {
    setup(info);
    vi.mocked(ipc.installUpdate).mockRejectedValue(new Error("boom"));
    const user = userEvent.setup();
    render(<Library />);
    const button = await screen.findByRole("button", { name: "今すぐ更新" });
    await waitFor(() => expect(button).toBeEnabled());
    await user.click(button);
    expect(ipc.installUpdate).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("更新できませんでした：boom")).toBeInTheDocument();
  });
});
