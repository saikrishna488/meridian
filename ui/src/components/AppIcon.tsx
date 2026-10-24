import { useState, useSyncExternalStore, type CSSProperties } from "react";
import { on } from "../lib/bridge";

let iconRevision = 0;
const iconListeners = new Set<() => void>();
let stopIconEvents: (() => void) | undefined;
function subscribeIconRevision(listener: () => void) {
  iconListeners.add(listener);
  stopIconEvents ??= on("search.invalidated", () => {
    iconRevision++;
    iconListeners.forEach(notify => notify());
  });
  return () => {
    iconListeners.delete(listener);
    if (!iconListeners.size) { stopIconEvents?.(); stopIconEvents = undefined; }
  };
}
const getIconRevision = () => iconRevision;
const builtinIcons: Record<string, string> = {
  "meridian-drift.desktop": "browser",
  "meridian-calendar.desktop": "calendar",
  "meridian-clock.desktop": "clock",
  "meridian-pulse.desktop": "pulse",
  "meridian-settings.desktop": "settings",
  "meridian-finder.desktop": "finder",
  "meridian-terminal.desktop": "terminal",
  "meridian-control-center.desktop": "control-center",
};
export function AppIcon({ name, icon, id }: { name: string; icon?: string | null; id?: string }) {
  const revision = useSyncExternalStore(subscribeIconRevision, getIconRevision, getIconRevision);
  const builtin = id ? builtinIcons[id] : icon === "view-app-grid" ? "applications" : icon === "user-trash" ? "trash" : null;
  const fixed = builtin ? `meridian://ui/desktop-icons/meridian/${builtin}.svg` : null;
  const identity = JSON.stringify([fixed, icon, id, revision]);
  const [fallback, setFallback] = useState({ identity, attempt: 0 });
  const attempt = fallback.identity === identity ? fallback.attempt : 0;
  const dynamic = (uri: string | null) => uri ? `${uri}?rev=${revision}` : null;
  // Retry installed app artwork on catalog refresh, then show a legible monogram.
  const candidates = [fixed, id ? dynamic(`meridian://app-icons/${encodeURIComponent(id)}`) : null,
    icon && !icon.startsWith("/") ? dynamic(`meridian://icons/${encodeURIComponent(icon)}`) : null].filter((src): src is string => !!src);
  const src = candidates[attempt] ?? null;
  const generic = !!src?.includes("/generic.svg");
  const hue = Array.from(name).reduce((n, c) => (n * 31 + c.charCodeAt(0)) % 360, 0);
  return <span className={`dock-icon app-icon${src ? " has-image" : ""}${src && src !== fixed ? " system-artwork" : ""}${generic ? " generic-image" : ""}`} aria-hidden="true" style={{ "--app-hue": hue } as CSSProperties}>
    {src ? <img key={src} src={src} alt="" onError={() => setFallback({ identity, attempt: attempt + 1 })} /> : name.trim()[0]?.toLocaleUpperCase() || "?"}
  </span>;
}
