import { cloneElement, useId, useState, type ReactElement, type ReactNode } from "react";
import { Icon, type IconName } from "../components/Icon";
import { Card } from "../components/ui/card";
export type Tone = "green" | "blue" | "orange" | "red" | "white";
export function IconTile({ icon, tone = "blue", className = "" }: { icon: IconName; tone?: Tone; className?: string }) {
  return <span className={`muk-icon-tile ${className}`} data-tone={tone} aria-hidden="true"><Icon name={icon} /></span>;
}
export function SettingRow({ title, description, children, status }: { title: ReactNode; description?: ReactNode; children: ReactNode; status?: ReactNode }) {
  return <div className="settings-row muk-setting-row"><div className="muk-setting-copy"><strong>{title}</strong>{description && <p>{description}</p>}{status && <small role="status">{status}</small>}</div><div className="muk-setting-control">{children}</div></div>;
}
export function Section({ title, description, children, className = "" }: { title: string; description?: ReactNode; children: ReactNode; className?: string }) {
  const id = useId();
  return <section className={`muk-section ${className}`} aria-labelledby={id}><header><h3 id={id}>{title}</h3>{description && <p>{description}</p>}</header><Card>{children}</Card></section>;
}
export function Badge({ children, tone = "green" }: { children: ReactNode; tone?: Tone }) {
  return <span className="muk-badge" data-tone={tone}>{children}</span>;
}
/** Optional hints only: the wrapped control still needs its own accessible name. */
export function HoverTip({ label, children }: { label: string; children: ReactElement }) {
  const id = useId();
  const [dismissed, setDismissed] = useState(false);
  const child = children as ReactElement<{ "aria-describedby"?: string }>;
  return <span className="muk-tip-anchor" data-dismissed={dismissed || undefined} onMouseLeave={() => setDismissed(false)} onFocusCapture={() => setDismissed(false)} onKeyDown={event => { if (event.key === "Escape" && !dismissed) { setDismissed(true); event.stopPropagation(); } }}>{cloneElement(child, { "aria-describedby": [child.props["aria-describedby"], id].filter(Boolean).join(" ") })}<span id={id} role="tooltip" className="muk-tooltip">{label}</span></span>;
}
export function EmptyState({ title, description, action }: { title: string; description?: ReactNode; action?: ReactNode }) {
  return <div className="muk-empty" role="status"><strong>{title}</strong>{description && <p>{description}</p>}{action}</div>;
}
