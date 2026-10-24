import type { ComponentProps } from "react";
/** Omit value for an indeterminate native progress indicator. */
export function Progress({ className = "", ...props }: ComponentProps<"progress">) {
  return <progress {...props} className={`ui-progress ${className}`} />;
}
