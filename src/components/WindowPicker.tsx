import { useCallback, useEffect, useRef, useState } from "react";
import { beginRegion, listWindows } from "../ipc";
import type { WindowEntry } from "../ipc/types";
import { useLang } from "../i18n";
import Button from "./Button";
import "./WindowPicker.css";

interface WindowPickerProps {
  /** Called right after a window is picked (begin_region already invoked) so the
   *  caller can close its own "picker is open" UI state -- the panel swaps its content
   *  back, the library closes its modal. */
  onPicked: () => void;
  /** Escape, the "戻る" button, or (when wrapped in a modal) a backdrop click / close
   *  button -- nothing was picked. */
  onCancel: () => void;
}

/**
 * The "choose the lecture window" list (docs/impl/frontend.md > Panel's picker;
 * docs/impl/library.md > Layout's start button), shared by the panel -- where it
 * replaces the panel's own body content -- and the library window, where the caller
 * wraps it in a modal dialog. Owns its data loading, listbox selection and keyboard
 * handling (Arrow/Enter/Escape) so both call sites behave identically.
 */
export default function WindowPicker({ onPicked, onCancel }: WindowPickerProps) {
  const { t } = useLang();
  const [windows, setWindows] = useState<WindowEntry[] | null>(null);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const listboxRef = useRef<HTMLUListElement>(null);

  useEffect(() => {
    let active = true;
    listWindows().then((list) => {
      if (active) setWindows(list);
    });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (windows && windows.length > 0) listboxRef.current?.focus();
  }, [windows]);

  const pick = useCallback(
    (windowId: number) => {
      void beginRegion(windowId);
      onPicked();
    },
    [onPicked]
  );

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
        return;
      }
      if (!windows || windows.length === 0) return;
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, windows.length - 1));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const w = windows[selectedIndex];
        if (w) pick(w.id);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [windows, selectedIndex, onCancel, pick]);

  return (
    <div className="window-picker">
      <h2 id="window-picker-heading" className="window-picker-heading">
        {t("picker.heading")}
      </h2>
      {windows === null ? (
        <p className="window-picker-status">{t("picker.loading")}</p>
      ) : windows.length === 0 ? (
        <p className="window-picker-status">{t("picker.empty")}</p>
      ) : (
        <ul
          ref={listboxRef}
          role="listbox"
          aria-labelledby="window-picker-heading"
          aria-activedescendant={`window-picker-option-${selectedIndex}`}
          tabIndex={0}
          className="window-picker-list"
        >
          {windows.map((w, i) => (
            // Keyboard interaction for this listbox is handled once, globally, by the
            // keydown effect above (Arrow keys + Enter) using aria-activedescendant --
            // the standard ARIA listbox pattern, where individual options are not
            // themselves tab-stops. The click handler below is the mouse-only
            // equivalent of that same Enter action, so it needs no keyboard listener
            // of its own.
            // eslint-disable-next-line jsx-a11y/click-events-have-key-events
            <li
              key={w.id}
              id={`window-picker-option-${i}`}
              role="option"
              aria-selected={i === selectedIndex}
              className={i === selectedIndex ? "window-picker-option is-selected" : "window-picker-option"}
              onPointerEnter={() => setSelectedIndex(i)}
              onClick={() => pick(w.id)}
            >
              <img src={w.thumb} alt="" className="window-picker-thumb" />
              <span className="window-picker-meta">
                <span className="window-picker-app">{w.app}</span>
                <span className="window-picker-window-title">{w.title}</span>
              </span>
            </li>
          ))}
        </ul>
      )}
      <Button kind="secondary" onClick={onCancel}>
        {t("action.back")}
      </Button>
    </div>
  );
}
