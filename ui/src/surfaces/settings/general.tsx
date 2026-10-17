import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import type { SystemInfo } from "../../lib/generated/SystemInfo";
import type { UpdateInfo } from "../../lib/generated/UpdateInfo";
export function General() {
  const [info, setInfo] = useState<SystemInfo | null>(null), [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [busy, setBusy] = useState(false), [error, setError] = useState(""), [checked, setChecked] = useState("");
  const pending = useRef(false);
  useEffect(() => { void request({ method: "settings.system_info" }).then(setInfo).catch(e => setError(errorMessage(e))); }, []);
  async function check() {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError(""); setUpdate(null); setChecked("");
    try { setUpdate(await request({ method: "settings.check_updates" })); setChecked(new Date().toLocaleString()); }
    catch (e) { setError(errorMessage(e)); } finally { pending.current = false; setBusy(false); }
  }
  const rows = info ? [["Meridian version", info.version], ["Operating system", info.os], ["Kernel", info.kernel], ["Architecture", info.architecture], ["Computer name", info.hostname], ["Manufacturer", info.manufacturer], ["Model", info.model], ["Processor", info.processor], ["Memory", info.memory], ["Desktop", "Meridian"]] : [];
  return <>
    <div className="settings-card">{!info && <p className="device-empty" role="status">Reading system information…</p>}{rows.map(([label, value]) => <div className="settings-row general-info-row" key={label}><span>{label}</span><strong>{value || "Unavailable"}</strong></div>)}</div>
    <h3>Meridian updates</h3><div className="settings-card"><div className="settings-row"><div className="management-details"><strong>Software update</strong><small className="row-detail">Downloads and installation are manual.</small></div><button className="primary" disabled={busy} onClick={() => void check()}>{busy ? "Checking…" : "Check for updates"}</button></div>
      <div className="update-status" role="status" aria-live="polite">{busy ? "Contacting GitHub…" : update ? <><p>{update.message}</p>{update.latest && <p>Installed: {update.installed} · Latest release: {update.latest}</p>}{update.published && <p>Published: {new Date(update.published).toLocaleDateString()}</p>}<small>Last checked: {checked}</small></> : null}</div>
      <div className="settings-actions"><button onClick={() => void request({ method: "settings.open_releases" }).catch(e => setError(errorMessage(e)))}>View releases</button></div>
    </div><p className="section-caption">This checks Meridian releases only. Kernel, driver, and Linux distribution updates are managed by your system package manager.</p>
    {error && <p className="settings-error" role="alert">{error}</p>}
  </>;
}
