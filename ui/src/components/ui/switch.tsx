import { Switch as GlisseSwitch } from "glisse";
import type { CSSProperties } from "react";

export function Switch({ checked, onChange, label, disabled = false, id, labelledBy, style }: {
  checked: boolean; onChange: (checked: boolean) => void; label?: string;
  disabled?: boolean; id?: string; labelledBy?: string; style?: CSSProperties;
}) {
  return <GlisseSwitch className="meridian-toggle" id={id} checked={checked} onCheckedChange={onChange}
    aria-label={label} aria-labelledby={labelledBy} disabled={disabled} style={style} />;
}
