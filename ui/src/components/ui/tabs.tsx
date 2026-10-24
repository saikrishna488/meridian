import { useId, type ReactNode } from "react";
import { Button } from "./button";

type Item<T extends string> = { value: T; label: ReactNode; disabled?: boolean };
export function Tabs<T extends string>({ value, onValueChange, items, label, className = "", children }: {
  value: T; onValueChange: (value: T) => void; items: readonly Item<T>[];
  label: string; className?: string; children: ReactNode;
}) {
  const id = useId();
  const enabled = items.filter(item => !item.disabled);
  return <>
    <div className={`ui-tabs ${className}`} role="tablist" aria-label={label} onKeyDown={event => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
      event.preventDefault(); event.stopPropagation();
      const current = enabled.findIndex(item => item.value === value);
      const rtl = getComputedStyle(event.currentTarget).direction === "rtl";
      const forward = (event.key === "ArrowRight") !== rtl;
      const index = event.key === "Home" ? 0 : event.key === "End" ? enabled.length - 1 : (current + (forward ? 1 : enabled.length - 1)) % enabled.length;
      const next = enabled[index];
      if (next) { onValueChange(next.value); document.getElementById(`${id}-tab-${next.value}`)?.focus(); }
    }}>
      {items.map(item => <Button key={item.value} id={`${id}-tab-${item.value}`} role="tab"
        aria-selected={value === item.value} aria-controls={`${id}-panel`}
        tabIndex={value === item.value ? 0 : -1} disabled={item.disabled}
        onClick={() => onValueChange(item.value)}>{item.label}</Button>)}
    </div>
    <div key={value} id={`${id}-panel`} className="ui-tab-panel" role="tabpanel" aria-labelledby={`${id}-tab-${value}`} tabIndex={0}>{children}</div>
  </>;
}
