import type { ComponentProps, CSSProperties } from "react";

type Props = Omit<ComponentProps<"input">, "type">;
/** Presentation only. Hardware write scheduling belongs to the consumer. */
export function Slider({ className = "", min = 0, max = 100, value, style, ...props }: Props) {
  const number = Number(value), lower = Number(min), upper = Number(max);
  const fill = Number.isFinite(number) && upper > lower ? Math.min(100, Math.max(0, (number - lower) / (upper - lower) * 100)) : 0;
  return <input {...props} type="range" min={min} max={max} value={value}
    className={`ui-slider ${className}`} style={{ "--fill": `${fill}%`, ...style } as CSSProperties} />;
}
