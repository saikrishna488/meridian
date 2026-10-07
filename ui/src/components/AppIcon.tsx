import { useEffect, useState, type CSSProperties } from "react";
export function AppIcon({ name, icon, id }: { name: string; icon?: string | null; id?: string }) {
  const builtin = id === "meridian-settings.desktop" ? "settings" : id === "meridian-finder.desktop" ? "finder" : id === "meridian-terminal.desktop" ? "terminal" : id === "meridian-control-center.desktop" ? "control-center" : !id && icon === "view-app-grid" ? "applications" : !id && icon === "user-trash" ? "trash" : null;
  const fixed = builtin ? `meridian://ui/desktop-icons/meridian/${builtin}.svg` : null;
  const [failedSystem, setFailedSystem] = useState(false);
  useEffect(() => { setFailedSystem(false); }, [fixed, icon, id]);
  const hue = Array.from(name).reduce((n, c) => (n * 31 + c.charCodeAt(0)) % 360, 0);
  const src = fixed ?? (!failedSystem && (id || icon) ? id ? `meridian://app-icons/${encodeURIComponent(id)}` : `meridian://icons/${encodeURIComponent(icon!)}` : null);
  const generic = !!src?.endsWith("/generic.svg");
  return <span className={`dock-icon app-icon${src ? " has-image" : ""}${src && !fixed ? " system-artwork" : ""}${generic ? " generic-image" : ""}`} aria-hidden="true" style={{ "--app-hue": hue } as CSSProperties}>
    {generic && <span className="generic-monogram">{name.trim()[0]?.toLocaleUpperCase() || "?"}</span>}
    {src ? <img key={src} src={src} alt="" onError={() => setFailedSystem(true)} /> : name.trim()[0]?.toLocaleUpperCase() || "?"}
  </span>;
}
