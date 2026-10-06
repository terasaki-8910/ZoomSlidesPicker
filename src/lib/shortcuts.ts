// Conversion between tauri-plugin-global-shortcut accelerator strings (docs/impl/ipc.md)
// and what Settings shows/records for the three shortcut rows.

const MAC_SYMBOLS: Record<string, string> = {
  Control: "⌃",
  Alt: "⌥",
  Shift: "⇧",
  Super: "⌘",
};

const MODIFIER_ORDER = ["Control", "Alt", "Shift", "Super"] as const;

/** Accelerator string -> the platform's own notation (SPEC.md's own examples: macOS
 * "⌃⌥⌘S", Windows/Linux "Ctrl+Alt+Shift+S"). */
export function formatAccelerator(accel: string, mac: boolean): string {
  const parts = accel.split("+").filter(Boolean);
  if (parts.length === 0) return accel;
  const key = parts[parts.length - 1];
  const mods = parts.slice(0, -1);
  const ordered = MODIFIER_ORDER.filter((m) => mods.includes(m));
  if (mac) {
    return [...ordered.map((m) => MAC_SYMBOLS[m]), key.toUpperCase()].join("");
  }
  return [...ordered, key.toUpperCase()].join("+");
}

const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

interface KeyDownLike {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
  key: string;
}

/** The accelerator for a keydown event, or null while only modifier keys have been
 * pressed so far (the recorder should keep waiting for a non-modifier key). */
export function acceleratorFromKeyDown(e: KeyDownLike): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");
  const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  return [...mods, key].join("+");
}
