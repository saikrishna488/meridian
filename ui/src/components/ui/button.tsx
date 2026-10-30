import { Button as GlisseButton } from "glisse";
import type { ComponentProps } from "react";

export type ButtonProps = ComponentProps<"button"> & {
  variant?: "default" | "primary" | "danger" | "ghost";
  size?: "default" | "small" | "icon";
};

/** Preserve Meridian's existing button API while using Glisse's interaction and material. */
export function Button({ type = "button", variant = "default", size = "default", className = "", ...props }: ButtonProps) {
  return <GlisseButton {...props} type={type} variant={variant === "primary" ? "filled" : "glass"}
    destructive={variant === "danger"} data-size={size} className={`ui-button ui-button-${variant} ${className}`} />;
}
