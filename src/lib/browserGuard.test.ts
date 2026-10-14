import { afterEach, describe, expect, it } from "vitest";
import { installBrowserGuard, shouldCancelKey, type KeyLike } from "./browserGuard";

function k(key: string, mods: Partial<Omit<KeyLike, "key">> = {}): KeyLike {
  return { key, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...mods };
}

describe("shouldCancelKey", () => {
  it("cancels the browser's own shortcuts", () => {
    const cancelled: KeyLike[] = [
      k("j", { ctrlKey: true }),
      k("p", { ctrlKey: true }),
      k("f", { ctrlKey: true }),
      k("f", { metaKey: true }),
      k("g", { metaKey: true }),
      k("r", { metaKey: true }),
      k("R", { ctrlKey: true, shiftKey: true }),
      k("F3"),
      k("F5"),
      k("F7"),
      k("F12"),
      k("I", { ctrlKey: true, shiftKey: true }),
      k("i", { metaKey: true, altKey: true }),
      k("+", { ctrlKey: true }),
      k("=", { metaKey: true }),
      k("-", { ctrlKey: true }),
      k("0", { metaKey: true }),
      k("Tab", { ctrlKey: true }),
      k("ArrowLeft", { altKey: true }),
      k("ArrowRight", { metaKey: true }),
      k("h", { ctrlKey: true }),
      k("n", { ctrlKey: true }),
      k("t", { metaKey: true }),
      k("N", { ctrlKey: true, shiftKey: true }),
      k("Delete", { ctrlKey: true, shiftKey: true }),
      k("BrowserBack"),
      k("BrowserRefresh"),
    ];
    for (const e of cancelled) expect(shouldCancelKey(e), JSON.stringify(e)).toBe(true);
  });

  it("lets text editing keys through", () => {
    for (const key of ["a", "c", "v", "x", "z", "y"]) {
      expect(shouldCancelKey(k(key, { ctrlKey: true })), key).toBe(false);
      expect(shouldCancelKey(k(key, { metaKey: true })), key).toBe(false);
    }
    expect(shouldCancelKey(k("z", { metaKey: true, shiftKey: true }))).toBe(false);
  });

  it("lets the app's own keys through", () => {
    const passed: KeyLike[] = [
      k(",", { metaKey: true }),
      k(",", { ctrlKey: true }),
      k("Escape"),
      k("Enter"),
      k("ArrowDown"),
      k("Home"),
      k("F10", { shiftKey: true }),
      k("ContextMenu"),
      k("Tab"),
      k("Tab", { shiftKey: true }),
      k("ArrowLeft"),
    ];
    for (const e of passed) expect(shouldCancelKey(e), JSON.stringify(e)).toBe(false);
  });

  it("keeps Alt/Cmd+Left/Right working inside text fields (caret movement)", () => {
    const input = document.createElement("input");
    input.type = "text";
    expect(shouldCancelKey(k("ArrowLeft", { altKey: true, target: input }))).toBe(false);
    expect(shouldCancelKey(k("ArrowRight", { metaKey: true, target: input }))).toBe(false);
    expect(shouldCancelKey(k("ArrowLeft", { altKey: true, target: document.body }))).toBe(true);
  });
});

describe("installBrowserGuard", () => {
  let uninstall: (() => void) | undefined;
  afterEach(() => uninstall?.());

  it("prevents default on a browser key but not on an editing key", () => {
    uninstall = installBrowserGuard();
    const print = new KeyboardEvent("keydown", { key: "p", ctrlKey: true, cancelable: true, bubbles: true });
    document.body.dispatchEvent(print);
    expect(print.defaultPrevented).toBe(true);
    const paste = new KeyboardEvent("keydown", { key: "v", ctrlKey: true, cancelable: true, bubbles: true });
    document.body.dispatchEvent(paste);
    expect(paste.defaultPrevented).toBe(false);
  });

  it("cancels the context menu except in text inputs and textareas", () => {
    uninstall = installBrowserGuard();
    const input = document.createElement("input");
    const area = document.createElement("textarea");
    const div = document.createElement("div");
    document.body.append(input, area, div);
    for (const [el, expected] of [
      [input, false],
      [area, false],
      [div, true],
    ] as const) {
      const ev = new MouseEvent("contextmenu", { cancelable: true, bubbles: true });
      el.dispatchEvent(ev);
      expect(ev.defaultPrevented).toBe(expected);
    }
    input.remove();
    area.remove();
    div.remove();
  });
});

describe("installBrowserGuard: mouse buttons", () => {
  it("cancels the mouse's back and forward buttons", () => {
    const remove = installBrowserGuard(window);
    try {
      for (const button of [3, 4]) {
        const e = new MouseEvent("mouseup", { button, cancelable: true });
        window.dispatchEvent(e);
        expect(e.defaultPrevented, `button ${button}`).toBe(true);
      }
      const left = new MouseEvent("mouseup", { button: 0, cancelable: true });
      window.dispatchEvent(left);
      expect(left.defaultPrevented).toBe(false);
    } finally {
      remove();
    }
  });
});
