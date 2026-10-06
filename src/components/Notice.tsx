import type { ReactNode } from "react";
import "./Notice.css";

interface NoticeProps {
  /** Red is reserved for failures and warnings (design_brief.md); "neutral" is everything else. */
  tone?: "warning" | "neutral";
  children: ReactNode;
  id?: string;
}

export default function Notice({ tone = "warning", children, id }: NoticeProps) {
  // role="status" implies aria-live="polite": announced without interrupting, and without
  // the alarm connotation role="alert" would add to a routine, expected state banner.
  return (
    <p id={id} className={`notice notice-${tone}`} role="status">
      {children}
    </p>
  );
}
