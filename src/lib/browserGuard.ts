// The webview is a browser underneath, and its own shortcuts and context menu (print,
// find, reload, devtools, zoom, history, "Back"/"Reload" menu, ...) must not reach the
// user: they open dialogs or navigate away from the app (docs/impl/library.md > Round 5 >
// Browser behaviour off in every window). Windows also turns the accelerator keys off in
// WebView2 itself; this is the in-page half, for every view and platform.

/** The parts of a KeyboardEvent the guard reads (so tests need no real events). */
export interface KeyLike {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  target?: EventTarget | null;
}

export function isTextField(target: EventTarget | null | undefined): boolean {
  if (typeof HTMLElement === "undefined" || !(target instanceof HTMLElement)) return false;
  if (target instanceof HTMLTextAreaElement) return true;
  if (target instanceof HTMLInputElement) {
    return ["text", "search", "url", "tel", "email", "password", "number"].includes(target.type);
  }
  return target.isContentEditable;
}

// Cmd/Ctrl + letter that the browser claims: find, find next, downloads, print, reload,
// view source, save, open, bookmark, history, new window / tab, address bar, search.
const MOD_LETTERS = new Set(["f", "g", "j", "p", "r", "u", "s", "o", "d", "h", "n", "t", "l", "e", "k"]);
// Cmd/Ctrl + Shift + letter: devtools (I, J, C), hard reload (R), print dialog (P),
// private window (N), reopen tab (T), bookmarks (B, O), clear browsing data (Delete).
const MOD_SHIFT_LETTERS = new Set(["i", "j", "c", "r", "p", "n", "t", "b", "o", "Delete"]);
const FUNCTION_KEYS = new Set(["F3", "F5", "F7", "F12"]);
// Dedicated keys on some keyboards.
const BROWSER_KEYS = new Set(["BrowserBack", "BrowserForward", "BrowserRefresh", "BrowserHome", "BrowserSearch"]);

/**
 * True when the browser's own handling of this key press must be cancelled. Text editing
 * keys (Cmd/Ctrl + A, C, V, X, Z, Y), Shift+F10 / ContextMenu (the app's own menu),
 * Escape, arrows and the app's shortcuts (Cmd/Ctrl + ,) are never listed. The history
 * keys (Alt/Cmd + Left/Right) are left alone inside text fields, where they move the caret.
 */
export function shouldCancelKey(e: KeyLike): boolean {
  const mod = e.ctrlKey || e.metaKey;
  const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;

  if (FUNCTION_KEYS.has(key) || BROWSER_KEYS.has(key)) return true;

  if ((e.altKey || e.metaKey) && (key === "ArrowLeft" || key === "ArrowRight") && !e.ctrlKey && !e.shiftKey) {
    return !isTextField(e.target);
  }

  if (mod && !e.altKey) {
    if (key === "Tab") return true;
    if (key === "+" || key === "=" || key === "-" || key === "_" || key === "0") return true; // zoom
    if (e.shiftKey) return MOD_SHIFT_LETTERS.has(key);
    return MOD_LETTERS.has(key);
  }
  if (mod && e.altKey) {
    return key === "i"; // devtools on macOS (Cmd+Alt+I)
  }

  return false;
}

/** Installs the capturing keydown / contextmenu / ctrl+wheel guards on `target`. */
export function installBrowserGuard(target: Window = window): () => void {
  function onKeyDown(e: KeyboardEvent) {
    if (shouldCancelKey(e)) e.preventDefault();
  }
  function onContextMenu(e: MouseEvent) {
    // The page's own handlers (the library's row menu) still run: capturing + preventDefault
    // cancels only the browser's menu and does not stop propagation.
    if (!isTextField(e.target)) e.preventDefault();
  }
  function onWheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) e.preventDefault(); // pinch / ctrl+wheel zoom
  }
  // The mouse's back and forward buttons navigate the webview's history.
  function onMouseButton(e: MouseEvent) {
    if (e.button === 3 || e.button === 4) e.preventDefault();
  }
  target.addEventListener("keydown", onKeyDown, true);
  target.addEventListener("contextmenu", onContextMenu, true);
  target.addEventListener("wheel", onWheel, { capture: true, passive: false });
  target.addEventListener("mouseup", onMouseButton, true);
  target.addEventListener("auxclick", onMouseButton, true);
  return () => {
    target.removeEventListener("mouseup", onMouseButton, true);
    target.removeEventListener("auxclick", onMouseButton, true);
    target.removeEventListener("keydown", onKeyDown, true);
    target.removeEventListener("contextmenu", onContextMenu, true);
    target.removeEventListener("wheel", onWheel, true);
  };
}
