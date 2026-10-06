import type { ButtonHTMLAttributes } from "react";
import "./Button.css";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Exactly one primary action at a time (design_brief.md); everything else is secondary. */
  kind?: "primary" | "secondary";
}

export default function Button({ kind = "secondary", type = "button", className, ...rest }: ButtonProps) {
  const cls = ["btn", kind === "primary" ? "btn-primary" : "btn-secondary", className].filter(Boolean).join(" ");
  return <button type={type} className={cls} {...rest} />;
}
