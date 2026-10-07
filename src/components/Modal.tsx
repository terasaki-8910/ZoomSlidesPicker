import { useEffect, useRef, type ReactNode } from "react";
import "./Modal.css";

interface ModalProps {
  /** id of an element inside `children` that names the dialog (aria-labelledby). */
  titleId: string;
  /** Escape or a backdrop click -- light dismiss (~/.claude/rules/ui.md). The caller's
   *  own content still needs an explicit close/cancel control of its own; this only
   *  wires up the two implicit ways out. */
  onClose: () => void;
  children: ReactNode;
}

/**
 * A light-dismissing modal shell: a dimmed backdrop (click closes it), Escape closes
 * it, and the dialog itself is `role="dialog"`/`aria-modal`. Used for the library
 * window's start-session picker and its two confirm dialogs (docs/impl/library.md >
 * Layout) -- each supplies its own heading/buttons as children.
 */
export default function Modal({ titleId, onClose, children }: ModalProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    panelRef.current?.focus();
  }, []);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    // role="presentation" marks this backdrop as a plain click-catcher, not a
    // semantically meaningful element of its own -- jsx-a11y's interactive-element
    // rules exempt it accordingly; keyboard users close the dialog via Escape instead
    // (handled above), matching ui.md's "light dismiss + explicit button" (the content
    // below supplies the explicit button).
    <div
      className="modal-backdrop"
      role="presentation"
      onClick={(e) => {
        // Only a click on the backdrop itself (not one that bubbled up from inside the
        // dialog panel) should close it -- checking the target here, rather than
        // stopPropagation() on the panel, keeps the panel itself free of its own click
        // handler (which jsx-a11y flags on a non-interactive role="dialog" element).
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div ref={panelRef} role="dialog" aria-modal="true" aria-labelledby={titleId} tabIndex={-1} className="modal-panel">
        {children}
      </div>
    </div>
  );
}
