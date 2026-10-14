import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoAxeViolations } from "../test/axe";
import { makeSettingsView } from "../test/fixtures";
import { createIpcMock, createListenerMock } from "../test/mockIpc";

vi.mock("../ipc", () => createIpcMock());

import * as ipc from "../ipc";
import type { SettingsView } from "../ipc/types";
import Settings from "./Settings";

function setup(initial = makeSettingsView()) {
  const settingsListener = createListenerMock<SettingsView>();
  vi.mocked(ipc.getSettings).mockResolvedValue(initial);
  vi.mocked(ipc.applyLanguage).mockResolvedValue(undefined);
  vi.mocked(ipc.onSettings).mockImplementation(settingsListener.fn);
  return { settingsListener };
}

describe("Settings", () => {
  it("renders the three sections in Japanese by default", async () => {
    setup();
    render(<Settings />);

    expect(await screen.findByRole("heading", { name: "設定" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "一般" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "キャプチャ" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "ショートカット" })).toBeInTheDocument();
  });

  it("switching the language calls set_settings, then apply_language once the settings event arrives (C1)", async () => {
    const { settingsListener } = setup(makeSettingsView({ language: "auto", effectiveLanguage: "ja" }));
    const updated = makeSettingsView({ language: "en", effectiveLanguage: "en" });
    vi.mocked(ipc.setSettings).mockResolvedValue(updated);
    const user = userEvent.setup();
    render(<Settings />);

    const select = await screen.findByLabelText("言語");
    await user.selectOptions(select, "English");

    expect(ipc.setSettings).toHaveBeenCalledWith(expect.objectContaining({ language: "en" }));

    const applyCallsBefore = vi.mocked(ipc.applyLanguage).mock.calls.length;
    settingsListener.emit(updated);

    expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
    await waitFor(() => expect(ipc.applyLanguage).toHaveBeenCalledTimes(applyCallsBefore + 1));
  });

  it("choosing a save folder calls choose_save_root then set_settings, and confirms", async () => {
    setup();
    vi.mocked(ipc.chooseSaveRoot).mockResolvedValue("/tmp/sample-save-root");
    vi.mocked(ipc.setSettings).mockImplementation(async (s) => ({ ...s, effectiveLanguage: "ja" }));
    const user = userEvent.setup();
    render(<Settings />);

    await user.click(await screen.findByRole("button", { name: "変更…" }));

    expect(ipc.chooseSaveRoot).toHaveBeenCalledTimes(1);
    expect(ipc.setSettings).toHaveBeenCalledWith(expect.objectContaining({ saveRoot: "/tmp/sample-save-root" }));
    expect(await screen.findByText("保存しました")).toBeInTheDocument();
  });

  it("rejects an out-of-range check interval without saving, and accepts a valid one", async () => {
    setup();
    vi.mocked(ipc.setSettings).mockImplementation(async (s) => ({ ...s, effectiveLanguage: "ja" }));
    const user = userEvent.setup();
    render(<Settings />);

    const input = await screen.findByLabelText("比較の周期（秒）");
    await user.clear(input);
    await user.type(input, "9");
    await user.tab();

    expect(await screen.findByText(/0\.5〜5 秒/)).toBeInTheDocument();
    expect(ipc.setSettings).not.toHaveBeenCalled();

    await user.clear(input);
    await user.type(input, "2.5");
    await user.tab();

    await waitFor(() => expect(ipc.setSettings).toHaveBeenCalledWith(expect.objectContaining({ intervalS: 2.5 })));
  });

  it("records a new shortcut from the next key combination, and Esc cancels without saving", async () => {
    setup();
    vi.mocked(ipc.setSettings).mockImplementation(async (s) => ({ ...s, effectiveLanguage: "ja" }));
    const user = userEvent.setup();
    render(<Settings />);

    await user.click((await screen.findAllByRole("button", { name: "変更" }))[0]); // 今すぐ保存 row
    expect(await screen.findByText("キーを押してください（Esc で取り消し）")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "Escape" });
    expect(ipc.setSettings).not.toHaveBeenCalled();

    await user.click(screen.getAllByRole("button", { name: "変更" })[0]);
    fireEvent.keyDown(window, { key: "t", ctrlKey: true, altKey: true });

    await waitFor(() =>
      expect(ipc.setSettings).toHaveBeenCalledWith(
        expect.objectContaining({ shortcuts: expect.objectContaining({ saveNow: "Control+Alt+T" }) })
      )
    );
  });

  it("has no axe violations (C5)", async () => {
    setup();
    const { container } = render(<Settings />);
    await screen.findByRole("heading", { name: "設定" });
    await expectNoAxeViolations(container);
  });
});

// ACCEPTANCE C22 -- docs/impl/updates.md > UI > Settings.
describe("Settings update check", () => {
  function setupUpdate() {
    setup();
    vi.mocked(ipc.getAppVersion).mockResolvedValue("0.1.6");
    vi.mocked(ipc.getAppState).mockResolvedValue({
      session: null,
      lastSession: null,
      permission: { granted: true, platform: "macos" },
      videoProgress: null,
    });
    vi.mocked(ipc.onAppState).mockResolvedValue(() => {});
    vi.mocked(ipc.answerUpdate).mockResolvedValue(undefined);
  }

  it("shows the running version", async () => {
    setupUpdate();
    render(<Settings />);
    expect(await screen.findByText("バージョン 0.1.6")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "アップデートを確認" })).toBeEnabled();
  });

  it("disables the button while checking", async () => {
    setupUpdate();
    let resolve!: (v: { current: string; available: null; skipped: boolean }) => void;
    vi.mocked(ipc.checkUpdate).mockReturnValue(new Promise((r) => (resolve = r)));
    const user = userEvent.setup();
    render(<Settings />);
    const button = await screen.findByRole("button", { name: "アップデートを確認" });
    await user.click(button);
    expect(button).toBeDisabled();
    expect(screen.getByText("確認しています…")).toBeInTheDocument();
    resolve({ current: "0.1.6", available: null, skipped: false });
    expect(await screen.findByText("最新の版です。")).toBeInTheDocument();
    expect(button).toBeEnabled();
  });

  it("an available version opens the dialog and says so; Later just closes it", async () => {
    setupUpdate();
    vi.mocked(ipc.checkUpdate).mockResolvedValue({
      current: "0.1.6",
      available: { version: "0.2.0", current: "0.1.6" },
      skipped: false,
    });
    const user = userEvent.setup();
    render(<Settings />);
    await user.click(await screen.findByRole("button", { name: "アップデートを確認" }));
    expect(await screen.findByRole("heading", { name: "新しい版があります" })).toBeInTheDocument();
    expect(screen.getAllByText("0.2.0 があります。")).toHaveLength(1);
    await user.click(screen.getByRole("button", { name: "後で" }));
    expect(ipc.answerUpdate).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByText("0.2.0 があります。")).toBeInTheDocument();
  });

  it("a skipped version still opens the dialog, the line says it is skipped, and Skip answers skip", async () => {
    setupUpdate();
    vi.mocked(ipc.checkUpdate).mockResolvedValue({
      current: "0.1.6",
      available: { version: "0.2.0", current: "0.1.6" },
      skipped: true,
    });
    const user = userEvent.setup();
    render(<Settings />);
    await user.click(await screen.findByRole("button", { name: "アップデートを確認" }));
    await screen.findByRole("dialog");
    expect(screen.getByText("0.2.0 はスキップしています。")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "この版をスキップ" }));
    expect(ipc.answerUpdate).toHaveBeenCalledWith("skip");
  });

  it("an error shows the reason", async () => {
    setupUpdate();
    vi.mocked(ipc.checkUpdate).mockRejectedValue(new Error("offline"));
    const user = userEvent.setup();
    render(<Settings />);
    await user.click(await screen.findByRole("button", { name: "アップデートを確認" }));
    expect(await screen.findByText("確認できませんでした：offline")).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
