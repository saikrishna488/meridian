import { useState, type CSSProperties } from "react";
const builtinIcons: Record<string, string> = {
  "meridian-drift.desktop": "browser",
  "meridian-vault.desktop": "vault",
  "meridian-nova.desktop": "nova",
  "meridian-calendar.desktop": "calendar",
  "meridian-clock.desktop": "clock",
  "meridian-pulse.desktop": "pulse",
  "meridian-settings.desktop": "settings",
  "meridian-finder.desktop": "finder",
  "meridian-terminal.desktop": "terminal",
  "meridian-control-center.desktop": "control-center",
};
export function AppIcon({ name, icon, id }: { name: string; icon?: string | null; id?: string }) {
  const builtin = id ? builtinIcons[id] : icon === "view-app-grid" ? "applications" : icon === "user-trash" ? "trash" : null;
  const fixed = builtin ? `meridian://ui/desktop-icons/meridian/${builtin}.svg` : null;
  const identity = JSON.stringify([fixed, icon, id]);
  const [fallback, setFallback] = useState({ identity, attempt: 0 });
  const attempt = fallback.identity === identity ? fallback.attempt : 0;
  const hue = Array.from(name).reduce((n, c) => (n * 31 + c.charCodeAt(0)) % 360, 0);
  // Try the desktop-entry artwork, then its theme name, then a legible monogram.
  const candidates = [fixed, id ? `meridian://app-icons/${encodeURIComponent(id)}` : null, icon && !icon.startsWith("/") ? `meridian://icons/${encodeURIComponent(icon)}` : null].filter((src): src is string => !!src);
  const src = candidates[attempt] ?? null;
  const generic = !!src?.endsWith("/generic.svg");
  return <span className={`dock-icon app-icon${src ? " has-image" : ""}${src && src !== fixed ? " system-artwork" : ""}${generic ? " generic-image" : ""}`} aria-hidden="true" style={{ "--app-hue": hue } as CSSProperties}>
    {generic && <span className="generic-monogram">{name.trim()[0]?.toLocaleUpperCase() || "?"}</span>}
    {src ? <img key={src} src={src} alt="" onError={() => setFallback({ identity, attempt: attempt + 1 })} /> : name.trim()[0]?.toLocaleUpperCase() || "?"}
  </span>;
}
