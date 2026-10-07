import type { CSSProperties } from "react";
export function Toggle({ checked, onChange, label, disabled = false, id, labelledBy, style }: {
 checked: boolean; onChange: (checked: boolean) => void; label?: string; disabled?: boolean; id?: string; labelledBy?: string; style?: CSSProperties;
}) {
 return <button type="button" id={id} className="meridian-toggle" role="switch" aria-checked={checked} aria-label={label} aria-labelledby={labelledBy} disabled={disabled} style={style} onClick={() => onChange(!checked)}><span className="toggle-thumb" aria-hidden="true" /></button>;
}
