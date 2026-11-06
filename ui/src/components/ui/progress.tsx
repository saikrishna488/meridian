type Props = { value?: number; max?: number; className?: string; level?: "normal" | "warning" | "critical"; "aria-label"?: string };
export function Progress({ className = "", value, max = 1, level = "normal", ...props }: Props) {
  const percent = value == null ? undefined : Math.max(0, Math.min(100, value / (max > 0 ? max : 1) * 100));
  return <progress {...props} data-level={level} max={100} value={percent} className={`ui-progress ${className}`} />;
}
