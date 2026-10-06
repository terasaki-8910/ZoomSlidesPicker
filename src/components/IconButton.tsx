import type { ButtonHTMLAttributes, ReactNode } from "react";
import "./IconButton.css";

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: ReactNode;
  /** Required: this button has no visible text, so its accessible name comes from here. */
  label: string;
}

export default function IconButton({ icon, label, type = "button", className, ...rest }: IconButtonProps) {
  const cls = ["icon-btn", className].filter(Boolean).join(" ");
  return (
    <button type={type} className={cls} aria-label={label} title={label} {...rest}>
      {icon}
    </button>
  );
}
