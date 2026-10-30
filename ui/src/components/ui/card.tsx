import { Glass } from "glisse";
import type { ComponentProps } from "react";
export function Card({ className = "", ...props }: ComponentProps<"div">) {
  return <Glass {...props} className={`ui-card ${className}`} />;
}
