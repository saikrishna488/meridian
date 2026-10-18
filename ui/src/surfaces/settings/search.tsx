import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import { Toggle } from "../../components/Toggle";
import type { SearchPreferences } from "../../lib/generated/SearchPreferences";
export function SearchSettings() {
  const [value, setValue] = useState<SearchPreferences | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const pending = useRef(false);
  useEffect(() => { let alive = true; void request({ method: "settings.search" }).then(value => { if (alive) setValue(value); }).catch(e => { if (alive) setError(errorMessage(e)); }); return () => { alive = false; }; }, []);
  async function save(key: keyof SearchPreferences, enabled: boolean) {
    if (!value || pending.current) return;
    pending.current = true; setBusy(true); setError("");
    try { setValue(await request({ method: "settings.search_set", params: { ...value, [key]: enabled } })); }
    catch (e) { setError(errorMessage(e)); } finally { pending.current = false; setBusy(false); }
  }
  return <><p className="section-caption">Installed applications are always searchable. Searches stay on this computer.</p><div className="settings-card">{([['settings','System settings','Find settings such as Wi-Fi, displays and sound.'],['calculator','Calculator','Calculate arithmetic expressions. Select a result to copy it.']] as const).map(([key,label,description]) => <div className="settings-row" key={key}><div><strong>{label}</strong><small className="row-detail">{description}</small></div><Toggle label={`Search ${label.toLowerCase()}`} checked={value?.[key] === true} disabled={!value || busy} onChange={enabled => void save(key, enabled)} /></div>)}</div>{error && <p role="alert" className="settings-error">{error}</p>}</>;
}
