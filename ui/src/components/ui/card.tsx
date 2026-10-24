import type { ComponentProps } from "react";
export function Card({ className = "", ...props }: ComponentProps<"div">) {
  return <div {...props} className={`ui-card ${className}`} />;
}
