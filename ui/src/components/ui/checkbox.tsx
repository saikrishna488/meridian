import type { ComponentProps } from "react";
export function Checkbox({ className = "", ...props }: Omit<ComponentProps<"input">, "type">) {
  return <input {...props} type="checkbox" className={`ui-checkbox ${className}`} />;
}
export function Radio({ className = "", ...props }: Omit<ComponentProps<"input">, "type">) {
  return <input {...props} type="radio" className={`ui-radio ${className}`} />;
}
