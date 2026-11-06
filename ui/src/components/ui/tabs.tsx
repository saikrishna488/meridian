import { useId, type ReactNode } from "react";
type Item<T extends string> = { value: T; label: ReactNode; disabled?: boolean };
export function Tabs<T extends string>({ value, onValueChange, items, label, className = "", children }: {
  value: T; onValueChange: (value: T) => void; items: readonly Item<T>[]; label: string; className?: string; children: ReactNode;
}) {
  const id = useId();
  return <><div className={`ui-tabs ${className}`} role="tablist" aria-label={label} onKeyDown={event => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    const enabled = items.filter(item => !item.disabled); if (!enabled.length) return;
    event.preventDefault(); event.stopPropagation();
    const index = enabled.findIndex(item => item.value === value);
    const rtl = getComputedStyle(event.currentTarget).direction === "rtl";
    const forward = (event.key === "ArrowRight") !== rtl;
    const next = enabled[event.key === "Home" ? 0 : event.key === "End" ? enabled.length - 1 : (index + (forward ? 1 : enabled.length - 1)) % enabled.length];
    if (next) { onValueChange(next.value); document.getElementById(`${id}-${next.value}`)?.focus(); }
  }}>{items.map(item => <button type="button" className="ui-button ui-tab" key={item.value} id={`${id}-${item.value}`} role="tab" aria-selected={value === item.value} aria-controls={`${id}-panel`} tabIndex={value === item.value ? 0 : -1} disabled={item.disabled} onClick={() => onValueChange(item.value)}>{item.label}</button>)}</div>
    <div key={value} id={`${id}-panel`} className="ui-tab-panel" role="tabpanel" aria-labelledby={`${id}-${value}`} tabIndex={0}>{children}</div></>;
}
