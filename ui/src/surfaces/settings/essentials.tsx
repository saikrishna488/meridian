import { useEffect, useRef, useState } from "react";
import { Button } from "../../components/ui/button";
import { on, request } from "../../lib/bridge";
import { Switch } from "../../components/ui/switch";
import { errorMessage } from "../../lib/react";
import type { InputPreferences } from "../../lib/generated/InputPreferences";
import type { NotificationState } from "../../lib/generated/NotificationState";

type InputKey = keyof InputPreferences;
function ToggleRow({ title, detail, value, disabled, onChange }: {
  title: string; detail?: string; value: boolean; disabled?: boolean; onChange: (value: boolean) => void;
}) {
  return <div className="settings-row essentials-setting"><span><strong>{title}</strong>{detail && <small className="row-detail">{detail}</small>}</span>
    <Switch checked={value} disabled={disabled} onChange={onChange} label={title} /></div>;
}
function RangeRow({ title, value, min, max, step, unit, onChange }: {
  title: string; value: number; min: number; max: number; step: number; unit: string; onChange: (value: number) => void;
}) {
  return <label className="settings-row essentials-setting"><strong>{title}</strong>
    <span className="essentials-range"><input type="range" min={min} max={max} step={step} value={value}
      onChange={event => onChange(Number(event.target.value))} aria-label={title} />
      <output>{value}{unit}</output></span></label>;
}
export function InputSettings({ kind }: { kind: "keyboard" | "mouse" }) {
  const [draft, setDraft] = useState<InputPreferences | null>(null);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let mounted = true;
    void request({ method: "settings.input" })
      .then(value => { if (mounted) { setDraft(value); } })
      .catch(e => { if (mounted) setError(errorMessage(e)); });
    return () => { mounted = false; };
  }, []);
  function update(key: InputKey, value: number | boolean | string) {
    setDraft(current => current ? { ...current, [key]: value } : current);
    setMessage("");
  }
  async function save() {
    if (!draft || busy) return;
    setBusy(true); setError(""); setMessage("");
    try {
      const result = await request({ method: "settings.input_set", params: draft });
      setDraft(result); setMessage("Input settings applied to the labwc session.");
    } catch(e) { setError(errorMessage(e)); }
    finally { setBusy(false); }
  }
  if (!draft) return <p className="section-caption">{error || "Loading input settings…"}</p>;
  return <div className="essentials-panel">
    <p className="section-caption">Input controls are unsupported in this session: labwc does not expose the readback needed to verify changes. Values below may not reflect connected devices.</p>
    <fieldset disabled className="essentials-disabled">
    {kind === "keyboard" ? <>
      <div className="settings-card">
        <h3>Typing</h3>
        <label className="settings-row essentials-setting"><strong>Keyboard layout</strong>
          <select aria-label="Keyboard layout" value={draft.keyboard_layout}
            onChange={event => update("keyboard_layout", event.target.value)}>
            {!["us", "gb", "in", "de", "fr", "es", "it", "br", "ca", "jp", "ru"].includes(draft.keyboard_layout)
              && <option value={draft.keyboard_layout}>{draft.keyboard_layout}</option>}
            {[["us", "English (US)"], ["gb", "English (UK)"], ["in", "English (India)"], ["de", "German"], ["fr", "French"], ["es", "Spanish"], ["it", "Italian"], ["br", "Portuguese (Brazil)"], ["ca", "Canadian"], ["jp", "Japanese"], ["ru", "Russian"]].map(([code, name]) =>
              <option key={code} value={code}>{name}</option>)}
          </select>
        </label>
        <ToggleRow title="Num Lock on startup" value={draft.num_lock} onChange={value => update("num_lock", value)} />
        <RangeRow title="Key repeat speed" value={draft.repeat_rate} min={1} max={60} step={1} unit=" /sec"
          onChange={value => update("repeat_rate", value)} />
        <RangeRow title="Repeat delay" value={draft.repeat_delay} min={100} max={1200} step={50} unit=" ms"
          onChange={value => update("repeat_delay", value)} />
        <p className="section-caption">These settings control how keys repeat in Meridian's labwc session.</p>
      </div>

    </> : <>
      <div className="settings-card"><h3>Mouse</h3>
        <RangeRow title="Pointer speed" value={draft.mouse_speed} min={-1} max={1} step={0.05} unit=""
          onChange={value => update("mouse_speed", value)} />
        <ToggleRow title="Pointer acceleration" detail="Adaptive movement" value={draft.mouse_acceleration} onChange={value => update("mouse_acceleration", value)} />
        <ToggleRow title="Natural scrolling" value={draft.mouse_natural_scroll} onChange={value => update("mouse_natural_scroll", value)} />
        <ToggleRow title="Left-handed buttons" value={draft.mouse_left_handed} onChange={value => update("mouse_left_handed", value)} />
      </div>
      <div className="settings-card"><h3>Touchpad</h3>
        <RangeRow title="Touchpad speed" value={draft.touchpad_speed} min={-1} max={1} step={0.05} unit=""
          onChange={value => update("touchpad_speed", value)} />
        <ToggleRow title="Tap to click" value={draft.tap_to_click} onChange={value => update("tap_to_click", value)} />
        <ToggleRow title="Tap and drag" value={draft.tap_and_drag} onChange={value => update("tap_and_drag", value)} />
        <ToggleRow title="Natural scrolling" value={draft.touchpad_natural_scroll} onChange={value => update("touchpad_natural_scroll", value)} />
        <ToggleRow title="Disable while typing" value={draft.disable_while_typing} onChange={value => update("disable_while_typing", value)} />
      </div>
    </>}
    </fieldset>
    {error && <p role="alert" className="settings-error">{error}</p>}
    {message && <p role="status" className="section-caption">{message}</p>}
    <div className="settings-actions"><Button disabled onClick={() => void save()}>Unsupported</Button></div>

  </div>;
}

export function ShortcutSettings() {
  return <section className="settings-card"><h3>Keyboard shortcuts</h3>
    <p className="section-caption">Reading and editing the active compositor shortcuts is unsupported. Review your labwc rc.xml for the current bindings.</p>
  </section>;
}
export function NotificationSettings() {
  const [data, setData] = useState<NotificationState | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const revision = useRef(0);
  useEffect(() => { let active = true; const unsubscribe = on("notifications.changed", event => { revision.current++; setData(event.data); });
    const initialRevision = revision.current;
    void request({ method: "notifications.get" })
    .then(value => { if (active && revision.current === initialRevision) setData(value); }).catch(e => { if (active) setError(errorMessage(e)); });
    return () => { active = false; unsubscribe(); }; }, []);
  async function setDnd(enabled: boolean) {
    if (busy) return; setBusy(true); setError("");
    const currentRevision = revision.current;
    try { const state = await request({ method: "notifications.do_not_disturb", params: { enabled } }); if (revision.current === currentRevision) setData(state); }
    catch (e) { setError(errorMessage(e)); } finally { setBusy(false); }
  }
  return <><section className="settings-card"><h3>Notifications</h3>
    <ToggleRow title="Do Not Disturb" detail="Silence notification pop-ups" value={data?.do_not_disturb ?? false}
      disabled={!data?.available || busy} onChange={value => void setDnd(value)} />
    <div className="settings-row"><span>Notification service</span><strong>{!data ? "Loading…" : data.available ? "Available" : "Unavailable"}</strong></div>
    <div className="settings-row"><span>Recent notifications</span><strong>{data?.items.length ?? "—"}</strong></div>
    </section>{error && <p role="alert" className="settings-error">{error}</p>}
    <p className="section-caption">Per-app notification rules are not supported by the current notification server.</p></>;
}
export function DateTimeSettings() {
  const [current, setCurrent] = useState<import("../../lib/generated/TimeState").TimeState | null>(null);
  const [zone, setZone] = useState("");
  const [sync, setSync] = useState(false);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    let alive = true;
    const timer = setInterval(() => setNow(new Date()), 1000);
    void request({ method: "settings.time" }).then(value => {
      if (alive) { setCurrent(value); setZone(value.timezone); setSync(value.ntp); }
    }).catch(e => { if (alive) setError(errorMessage(e)); });
    return () => { alive = false; clearInterval(timer); };
  }, []);
  async function save() {
    if (!current || busy) return;
    setBusy(true); setError("");
    try {
      const changed = await request({ method: "settings.time_set", params: {
        timezone: zone !== current.timezone ? zone : null,
        ntp: sync !== current.ntp ? sync : null,
      } });
      setCurrent(changed); setZone(changed.timezone); setSync(changed.ntp);
    } catch (e) { setError(errorMessage(e)); }
    finally { setBusy(false); }
  }
  const dirty = current && (current.timezone !== zone || current.ntp !== sync);
  return <><section className="settings-card"><h3>Date &amp; time</h3>
    <div className="settings-row"><span>Current time</span><strong>{now.toLocaleString()}</strong></div>
    <div className="settings-row"><span>Synchronization</span><strong>{current ? current.synchronized ? "Synchronized" : "Not synchronized" : "Unavailable"}</strong></div>
    <label className="settings-row essentials-setting"><strong>Time zone</strong>
      <input aria-label="Time zone" className="essentials-timezone" list="meridian-time-zones"
        value={zone} onChange={event => setZone(event.target.value)} disabled={!current || busy} />
      <datalist id="meridian-time-zones">{["UTC", "America/Chicago", "America/New_York", "America/Los_Angeles", "Asia/Kolkata", "Europe/London", "Europe/Berlin", "Asia/Tokyo"].map(value => <option key={value} value={value} />)}</datalist>
    </label>
    <ToggleRow title="Automatic time synchronization" value={sync} disabled={!current || busy} onChange={setSync} />
    </section>
    {error && <p role="alert" className="settings-error">{error}</p>}
    <div className="settings-actions"><Button disabled={!dirty || busy} onClick={() => void save()}>{busy ? "Applying…" : "Apply changes"}</Button></div>
    <p className="section-caption">Changes are applied by systemd-timedated and may require administrator authorization. Type any installed IANA time zone, such as America/Chicago.</p>
  </>;
}
export function PrinterSettings() {
  const [data, setData] = useState<import("../../lib/generated/PrinterInventory").PrinterInventory | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    let alive = true;
    void request({ method: "settings.printers" }).then(value => { if (alive) setData(value); })
      .catch(e => { if (alive) setError(errorMessage(e)); });
    return () => { alive = false; };
  }, []);
  return <><section className="settings-card"><h3>Printers</h3>
    {data?.printers.length ? data.printers.map(name => <div className="settings-row" key={name}><strong>{name}</strong><small>Installed printer</small></div>)
      : <p className="section-caption">{data ? data.cups_available ? "No installed printers found." : "CUPS printer discovery is unavailable." : "Finding printers…"}</p>}
  </section><section className="settings-card"><h3>Scanners</h3>
    {data?.scanners.length ? data.scanners.map(name => <div className="settings-row" key={name}><span>{name}</span></div>)
      : <p className="section-caption">{data ? data.sane_available ? "No scanners found." : "SANE scanner discovery is unavailable." : "Finding scanners…"}</p>}
  </section>{error && <p className="settings-error" role="alert">{error}</p>}
    <p className="section-caption">Device discovery is read-only. Adding printers or changing drivers still requires distribution printer-management tools.</p></>;
}
export function PrivacySettings() {
  return <><section className="settings-card"><h3>Privacy &amp; security</h3>
    <p className="section-caption">Screen locking and idle behavior are available in Power &amp; Lock. Camera, microphone, and location permissions require application sandbox or portal support; Meridian does not currently enforce per-app access controls.</p>
  </section></>;
}
