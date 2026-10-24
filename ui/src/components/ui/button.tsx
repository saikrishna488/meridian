import type { ComponentProps } from "react";

export type ButtonProps = ComponentProps<"button"> & {
  variant?: "default" | "primary" | "danger" | "ghost";
  size?: "default" | "small" | "icon";
};

/** Native button semantics and refs; variants use Meridian's semantic tokens. */
export function Button({ type = "button", variant = "default", size = "default", className = "", ...props }: ButtonProps) {
  return <button {...props} type={type} data-variant={variant} data-size={size} className={`ui-button ${className}`} />;
}
