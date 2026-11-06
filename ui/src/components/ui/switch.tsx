import type { CSSProperties } from "react";
export function Switch({ checked, onChange, label, disabled = false, id, labelledBy, describedBy, style }: {
  checked: boolean; onChange: (checked: boolean) => void; label?: string; disabled?: boolean; id?: string; labelledBy?: string; describedBy?: string; style?: CSSProperties;
}) {
  return <span className="meridian-toggle" data-checked={checked} data-disabled={disabled} style={style}>
    <input type="checkbox" role="switch" className="muk-switch-input" id={id} checked={checked}
      aria-label={label} aria-labelledby={labelledBy} aria-describedby={describedBy} disabled={disabled}
      onChange={event => onChange(event.currentTarget.checked)} />
    <span className="muk-switch-thumb" aria-hidden="true" />
  </span>;
}
