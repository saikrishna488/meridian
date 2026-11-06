import type { ComponentProps } from "react";
export type ButtonProps = ComponentProps<"button"> & { variant?: "default" | "primary" | "secondary" | "danger" | "ghost"; size?: "default" | "small" | "icon" };
export function Button({ type = "button", variant = "default", size = "default", className = "", ...props }: ButtonProps) {
  return <button {...props} type={type} data-size={size} className={`ui-button ui-button-${variant} ${className}`} />;
}
export function SurfaceButton({ type = "button", className = "", ...props }: ComponentProps<"button">) {
  return <button {...props} type={type} className={`muk-hover-option ${className}`} />;
}
