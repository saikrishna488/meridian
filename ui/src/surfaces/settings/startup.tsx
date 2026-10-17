import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import { errorMessage, useDialogFocus } from "../../lib/react";
import { Icon } from "../../components/Icon";
import { AppIcon } from "../../components/AppIcon";
import { Toggle } from "../../components/Toggle";
import type { StartupEntry } from "../../lib/generated/StartupEntry";
import type { SearchItem } from "../../lib/generated/SearchItem";

export function Startup() {
  const [entries, setEntries] = useState<StartupEntry[]>([]), [apps, setApps] = useState<SearchItem[]>([]);
  const [busy, setBusy] = useState(false), [loaded, setLoaded] = useState(false), [adding, setAdding] = useState(false);
  const [query, setQuery] = useState(""), [error, setError] = useState(""), [message, setMessage] = useState("");
  const pending = useRef(false), dialog = useRef<HTMLDivElement>(null);
  useDialogFocus(dialog, adding, () => { if (!pending.current) setAdding(false); });
  async function load() {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError("");
    try { setEntries(await request({ method: "settings.startup" })); setLoaded(true); }
    catch (e) { setError(errorMessage(e)); } finally { pending.current = false; setBusy(false); }
  }
  useEffect(() => { void load(); }, []);
  async function toggle(entry: StartupEntry, enabled: boolean) {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError(""); setMessage("");
    try {
      setEntries(await request({ method: "settings.startup_set", params: { id: entry.id, enabled } }));
      setMessage(`${entry.name} will ${enabled ? "start" : "stay closed"} the next time you sign in.`);
    } catch (e) { setError(errorMessage(e)); } finally { pending.current = false; setBusy(false); }
  }
  async function choose() {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError(""); setQuery("");
    try { setApps(await request({ method: "settings.startup_apps" })); setAdding(true); }
    catch (e) { setError(errorMessage(e)); } finally { pending.current = false; setBusy(false); }
  }
  async function add(app: SearchItem) {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError("");
    try { setEntries(await request({ method: "settings.startup_add", params: { id: app.id } })); setAdding(false); setMessage(`${app.title} will start the next time you sign in.`); }
    catch (e) { setError(errorMessage(e)); } finally { pending.current = false; setBusy(false); }
  }
  const matching = apps.filter(app => !entries.some(e => e.id === app.id) && `${app.title} ${app.subtitle ?? ""}`.toLowerCase().includes(query.toLowerCase()));
  return <>
    <div className="startup-summary settings-card"><span className="startup-summary-icon"><Icon name="grid" width="25" height="25" /></span><div><h3>Your apps, ready at sign-in</h3><p>Choose what opens when you log in to Meridian.</p></div><button className="primary" disabled={busy} onClick={() => void choose()}><Icon name="plus" width="15" height="15" />Add application</button></div>
    <div className="startup-list-heading"><h3>Login applications{loaded && <span>{entries.filter(e => e.enabled && e.available).length} enabled</span>}</h3><button aria-label="Refresh startup applications" disabled={busy} onClick={() => void load()}><Icon name="refresh" width="16" height="16" />Refresh</button></div>
    {!loaded && !error && <p role="status" className="device-empty">Reading startup applications…</p>}
    {loaded && !entries.length && <div className="settings-card startup-empty"><Icon name="grid" width="32" height="32" /><h3>A fresh start</h3><p>No login applications yet. Add an app to have it ready when you sign in.</p></div>}
    {entries.length > 0 && <div className="settings-card">{entries.map(entry => <div className="settings-row startup-row" key={entry.id}><AppIcon id={entry.id} name={entry.name} /><div className="startup-details"><strong>{entry.name}</strong><small>{entry.description || entry.command || "Login application"}</small><span className="startup-origin">{entry.user ? "Your account" : "System default"}{!entry.available && ` · ${entry.reason}`}</span></div><Toggle label={`Start ${entry.name} at sign-in`} checked={entry.enabled} disabled={busy || (!entry.enabled && !entry.available)} onChange={value => void toggle(entry, value)} /></div>)}</div>}
    <p className="section-caption">Changes apply at your next sign-in. Turning an app off here keeps it installed and does not close it now. System defaults are overridden only for your account.</p>
    {message && <p className="startup-message" role="status">{message}</p>}
    {error && !adding && <p className="settings-error" role="alert">{error}</p>}
    {adding && <div className="startup-dialog-scrim"><div ref={dialog} role="dialog" aria-modal="true" aria-labelledby="startup-dialog-title" className="startup-dialog"><header><div><h2 id="startup-dialog-title">Add a login application</h2><p>It will open automatically the next time you sign in.</p></div><button aria-label="Close application picker" disabled={busy} onClick={() => setAdding(false)}><Icon name="close" width="18" height="18" /></button></header><label className="startup-picker-search"><Icon name="search" width="17" height="17" /><input data-autofocus aria-label="Search installed applications" placeholder="Find an application" value={query} disabled={busy} onChange={e => setQuery(e.target.value)} /></label><div className="startup-app-picker">{matching.map(app => <button key={app.id} disabled={busy} onClick={() => void add(app)}><AppIcon id={app.id} name={app.title} icon={app.icon} /><span><strong>{app.title}</strong><small>{app.subtitle || app.id}</small></span><Icon name="plus" width="17" height="17" /></button>)}{!matching.length && <p className="device-empty">{query ? "No matching applications." : "All installed applications are already listed."}</p>}</div>{error && <p className="settings-error" role="alert">{error}</p>}<footer><button disabled={busy} onClick={() => setAdding(false)}>Cancel</button></footer></div></div>}
  </>;
}
