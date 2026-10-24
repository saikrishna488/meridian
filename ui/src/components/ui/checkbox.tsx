import type { ComponentProps } from "react";
/** Retains browser keyboard, form and high-contrast behavior. Supply a label. */
export function Checkbox({ className = "", ...props }: Omit<ComponentProps<"input">, "type">) {
  return <input {...props} type="checkbox" className={`ui-checkbox ${className}`} />;
}
