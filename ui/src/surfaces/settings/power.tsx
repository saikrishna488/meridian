import { useEffect, useRef, useState } from "react";
import { usePowerStatus } from "../../lib/power";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import { BatteryIcon } from "../../components/BatteryIcon";
import { Icon } from "../../components/Icon";
import { Select } from "../../components/Select";
import type { IdleSettings } from "../../lib/generated/IdleSettings";

export function Power() {
  const power = usePowerStatus();
  const [saved, setSaved] = useState<IdleSettings | null>(null), [draft, setDraft] = useState<IdleSettings | null>(null);
  const [busy, setBusy] = useState(false), [error, setError] = useState(""), [message, setMessage] = useState("");
  const pending = useRef(false);
  useEffect(() => { let alive = true; void request({ method: "settings.idle" }).then(settings => { if (alive) { setSaved(settings); setDraft(settings); } }).catch(e => { if (alive) setError(errorMessage(e)); }); return () => { alive = false; }; }, []);
  async function save() {
    if (!draft || pending.current) return;
    pending.current = true; setBusy(true); setError(""); setMessage("");
    try { const result = await request({ method: "settings.idle_set", params: draft }); setSaved(result); setDraft(result); setMessage("Screen-lock timing is saved. It takes effect the next time you sign in."); }
    catch (e) { setError(errorMessage(e)); } finally { pending.current = false; setBusy(false); }
  }
  const changed = !!draft && (saved?.lock_after !== draft.lock_after || saved?.blank_after !== draft.blank_after);
  return <>
    <div className="power-source settings-card"><span className="power-source-icon"><Icon name="power" width="24" height="24" /></span><div><h3>{power?.on_ac === true ? "Connected to power" : power?.on_ac === false ? "Running on battery" : "Power & battery"}</h3><p>{power?.on_ac === true ? "Your computer is using an external power source." : power?.on_ac === false ? "Your computer is using battery power." : "Battery readings appear when your hardware provides them."}</p></div></div>
    {power?.batteries.map(battery => <section className="settings-card battery-card" key={battery.name}><header><span><BatteryIcon percent={battery.percent} charging={battery.status === "Charging"} /></span><div><h3>{power.batteries.length === 1 ? "Battery" : battery.name}</h3><p>{battery.status}</p></div><strong>{battery.percent === null ? "—" : `${battery.percent}%`}</strong></header><div className="battery-meter" role={battery.percent === null ? "img" : "meter"} aria-label={`${battery.name} charge${battery.percent === null ? " unavailable" : ""}`} aria-valuemin={battery.percent === null ? undefined : 0} aria-valuemax={battery.percent === null ? undefined : 100} aria-valuenow={battery.percent ?? undefined} aria-valuetext={battery.percent === null ? undefined : `${battery.percent}%`}><span style={{ width: `${battery.percent ?? 0}%` }} /></div>{battery.health !== null && <p className="battery-health">Full charge capacity · {battery.health}% of design capacity</p>}</section>)}
    {power && !power.batteries.length && <p className="section-caption">No system battery detected. Screen-lock settings work on desktop computers too.</p>}
    <h3 className="power-section-title">Screen & lock</h3><div className="settings-card"><div className="settings-row"><div><strong>Automatically lock</strong><small className="row-detail">After this long without keyboard or mouse activity.</small></div><Select aria-label="Automatically lock after" disabled={!draft || busy} value={draft?.lock_after ?? 300} onChange={e => { const seconds = Number(e.target.value); setDraft(old => old && { lock_after: seconds, blank_after: seconds + old.blank_after - old.lock_after }); setMessage(""); }}>{[60,120,300,600,900,1800].map(seconds => <option key={seconds} value={seconds}>{seconds / 60} {seconds === 60 ? "minute" : "minutes"}</option>)}</Select></div><div className="settings-row"><div><strong>Turn off the display</strong><small className="row-detail">Wait after locking, then put the screen in standby.</small></div><Select aria-label="Turn off display after lock" disabled={!draft || busy} value={draft ? draft.blank_after - draft.lock_after : 30} onChange={e => { const seconds = Number(e.target.value); setDraft(old => old && { ...old, blank_after: old.lock_after + seconds }); setMessage(""); }}>{[30,60,120,300].map(seconds => <option key={seconds} value={seconds}>{seconds < 60 ? "30 seconds" : `${seconds / 60} ${seconds === 60 ? "minute" : "minutes"}`}</option>)}</Select></div><div className="settings-row power-save"><span>Timing changes apply at your next sign-in.</span><button className="primary" disabled={!changed || busy} onClick={() => void save()}>{busy ? "Saving…" : "Save changes"}</button></div></div>
    <div className="power-lock-note"><Icon name="lock" width="18" height="18" /><p>Lock anytime with <kbd>Super + L</kbd>. Meridian also locks before sleep and confirms the lock before turning off your display.</p></div>
    {message && <p className="startup-message" role="status">{message}</p>}{error && <p className="settings-error" role="alert">{error}</p>}
  </>;
}
