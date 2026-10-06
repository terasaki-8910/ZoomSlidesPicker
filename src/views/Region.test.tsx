import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { expectNoAxeViolations } from "../test/axe";
import { makeSettingsView } from "../test/fixtures";
import { createIpcMock } from "../test/mockIpc";

vi.mock("../ipc", () => createIpcMock());

import * as ipc from "../ipc";
import Region from "./Region";

const SNAPSHOT = {
  windowId: 1,
  app: "zoom.us",
  title: "Zoom Meeting",
  width: 1280,
  height: 720,
  image: "data:image/svg+xml,snapshot",
};

function setup() {
  vi.mocked(ipc.getRegionSnapshot).mockResolvedValue(SNAPSHOT);
  vi.mocked(ipc.startSession).mockResolvedValue(undefined);
  vi.mocked(ipc.cancelRegion).mockResolvedValue(undefined);
  // Region also calls useLang() internally, which needs these two regardless of this
  // view having no language-switching UI of its own.
  vi.mocked(ipc.getSettings).mockResolvedValue(makeSettingsView());
  vi.mocked(ipc.onSettings).mockResolvedValue(() => {});
  vi.mocked(ipc.applyLanguage).mockResolvedValue(undefined);
}

describe("Region", () => {
  it("loads and shows the snapshot image", async () => {
    setup();
    render(<Region />);
    expect(await screen.findByAltText("Zoom Meeting の静止画")).toBeInTheDocument();
  });

  it("Enter starts the session with the whole window when nothing was dragged (C2)", async () => {
    setup();
    render(<Region />);
    await screen.findByAltText("Zoom Meeting の静止画");

    fireEvent.keyDown(window, { key: "Enter" });

    expect(ipc.startSession).toHaveBeenCalledWith(1, { x: 0, y: 0, w: 1, h: 1 });
  });

  it("Esc cancels the region (C2)", async () => {
    setup();
    render(<Region />);
    await screen.findByAltText("Zoom Meeting の静止画");

    fireEvent.keyDown(window, { key: "Escape" });

    expect(ipc.cancelRegion).toHaveBeenCalledTimes(1);
  });

  it("the Start and Cancel buttons call the same commands as the keyboard", async () => {
    setup();
    render(<Region />);
    await screen.findByAltText("Zoom Meeting の静止画");

    screen.getByRole("button", { name: "取り消す" }).click();
    expect(ipc.cancelRegion).toHaveBeenCalledTimes(1);

    screen.getByRole("button", { name: "開始" }).click();
    expect(ipc.startSession).toHaveBeenCalledWith(1, { x: 0, y: 0, w: 1, h: 1 });
  });

  it("has no axe violations (C5)", async () => {
    setup();
    const { container } = render(<Region />);
    await screen.findByAltText("Zoom Meeting の静止画");
    await expectNoAxeViolations(container);
  });
});
