import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import "./ContextMenu.css";

export interface ContextMenuItem {
  id: string;
  label: string;
  icon: ReactNode;
  onSelect: () => void;
}

interface ContextMenuProps {
  /** Where the menu opens (viewport px); it is clamped to stay inside the window. */
  x: number;
  y: number;
  label: string;
  items: ContextMenuItem[];
  onClose: () => void;
}

const EDGE = 4;

/**
 * An in-page menu at the pointer. role=menu / menuitem; Up/Down/Home/End move, Enter or
 * Space picks, Escape, Tab, a click outside or the window losing focus closes it.
 * (~/.claude/rules/ui.md: every menu closes on Escape and outside click and has a
 * keyboard path.)
 */
export default function ContextMenu({ x, y, label, items, onClose }: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });

  useLayoutEffect(() => {
    const el = menuRef.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    setPos({
      left: Math.max(EDGE, Math.min(x, window.innerWidth - width - EDGE)),
      top: Math.max(EDGE, Math.min(y, window.innerHeight - height - EDGE)),
    });
  }, [x, y, items.length]);

  useEffect(() => {
    menuRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
  }, []);

  useEffect(() => {
    function onPointerDown(e: MouseEvent) {
      if (e.target instanceof Node && menuRef.current?.contains(e.target)) return;
      onClose();
    }
    function onKeyDown(e: KeyboardEvent) {
      const buttons = Array.from(menuRef.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? []);
      const idx = buttons.findIndex((b) => b === document.activeElement);
      let next: number | null = null;
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
        return;
      }
      if (e.key === "Tab") {
        onClose();
        return;
      }
      if (e.key === "ArrowDown") next = idx === -1 ? 0 : (idx + 1) % buttons.length;
      else if (e.key === "ArrowUp") next = idx === -1 ? buttons.length - 1 : (idx - 1 + buttons.length) % buttons.length;
      else if (e.key === "Home") next = 0;
      else if (e.key === "End") next = buttons.length - 1;
      if (next !== null) {
        e.preventDefault();
        e.stopPropagation();
        buttons[next]?.focus();
      }
    }
    document.addEventListener("mousedown", onPointerDown, true);
    document.addEventListener("contextmenu", onPointerDown, true);
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("blur", onClose);
    return () => {
      document.removeEventListener("mousedown", onPointerDown, true);
      document.removeEventListener("contextmenu", onPointerDown, true);
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("blur", onClose);
    };
  }, [onClose]);

  return (
    <div ref={menuRef} role="menu" aria-label={label} className="context-menu" style={{ left: pos.left, top: pos.top }}>
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          role="menuitem"
          className="context-menu-item"
          onClick={() => {
            onClose();
            item.onSelect();
          }}
        >
          <span className="context-menu-icon" aria-hidden="true">
            {item.icon}
          </span>
          {item.label}
        </button>
      ))}
    </div>
  );
}
