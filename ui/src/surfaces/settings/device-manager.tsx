import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import { errorMessage, useDialogFocus } from "../../lib/react";
import { Icon } from "../../components/Icon";
import type { ManagedDevice } from "../../lib/generated/ManagedDevice";
const labels: Record<string, string> = { connect: "Connect", disconnect: "Disconnect", on: "Turn On", off: "Turn Off", pause: "Pause Printing", resume: "Resume Printing", eject: "Eject safely", unmount: "Unmount", storage: "Open Disks" };
function label(action: string) { return action.startsWith("eject:") ? "Eject safely" : labels[action] ?? action; }
function DeviceGlyph({ category }: { category: string }) {
  const path = category === "Storage" ? "M4 4h16v16H4z M7 16h2 M12 10a3 3 0 1 0 6 0a3 3 0 1 0-6 0" : category === "USB devices" ? "M12 21V3m-3 3 3-3 3 3M12 15l-5-4V8m5 4 5-4V5" : category === "Bluetooth" ? "m7 7 10 10-5 4V3l5 4L7 17" : category === "Printers" ? "M7 8V3h10v5M7 17H3V8h18v9h-4M7 14h10v7H7z" : category.includes("adapters") ? "M3 8a14 14 0 0 1 18 0M6 12a9 9 0 0 1 12 0m-9 4a4 4 0 0 1 6 0M12 20h.01" : "M5 5h14v14H5zM9 9h6v6H9zM9 2v3m6-3v3M9 19v3m6-3v3";
  return <svg aria-hidden="true" width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" style={{ verticalAlign: "middle", marginRight: 8, flexShrink: 0 }}><path d={path} /></svg>;
}
export function DeviceManager({ navigate }: { navigate: (page: string) => void }) {
  const [devices, setDevices] = useState<ManagedDevice[]>([]), [selected, setSelected] = useState("");
  const [query, setQuery] = useState(""), [busy, setBusy] = useState(false), [loaded, setLoaded] = useState(false), [error, setError] = useState(""), [message, setMessage] = useState("");
  const [choice, setChoice] = useState<{ device: ManagedDevice; action: string } | null>(null);
  const pending = useRef(false), dialog = useRef<HTMLElement>(null);
  useDialogFocus(dialog, !!choice, () => { if (!busy) setChoice(null); });
  async function load() {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError(""); setMessage("");
    try { const result = await request({ method: "settings.devices" }); setDevices(result); setLoaded(true); setSelected(old => result.some(d => d.id === old) ? old : result[0]?.id ?? ""); }
    catch (e) { setError(errorMessage(e)); } finally { pending.current = false; setBusy(false); }
  }
  useEffect(() => { void load(); }, []);
  async function act(device: ManagedDevice, action: string) {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError(""); setMessage("");
    try { await request({ method: "settings.device_action", params: { id: device.id, action } }); setChoice(null); if (action.startsWith("eject")) setMessage("Device ejected. It is safe to unplug."); else if (action === "unmount") setMessage("Volume unmounted. Eject the whole device before unplugging it."); }
    catch (e) { setError(errorMessage(e)); setChoice(null); }
    finally { await loadPreservingError(); pending.current = false; setBusy(false); }
  }
  async function loadPreservingError() {
    try { setDevices(await request({ method: "settings.devices" })); } catch (e) { setError(previous => previous || errorMessage(e)); }
  }
  const matches = devices.filter(d => `${d.name} ${d.category} ${d.properties.flat().join(" ")}`.toLowerCase().includes(query.toLowerCase()));
  const groups = [...new Set(matches.map(d => d.category))];
  const device = devices.find(d => d.id === selected && matches.some(m => m.id === d.id)) ?? matches[0];
  return <div className="device-manager">
    <div className="device-toolbar"><input type="search" aria-label="Search devices" placeholder="Search devices" value={query} onChange={e => setQuery(e.target.value)} /><button aria-label="Refresh devices" title="Refresh" disabled={busy} onClick={() => void load()}><Icon name="refresh" /></button></div>
    {message && <p role="status">{message}</p>}
    {error && <p role="alert" className="settings-error">{error}</p>}
    <div className="device-layout" aria-busy={busy}><nav className="device-tree" aria-label="Devices">{groups.map(group => <details key={group} open><summary>{group}</summary>{matches.filter(d => d.category === group).map(d => <button key={d.id} aria-current={device?.id === d.id ? "true" : undefined} onClick={() => setSelected(d.id)}><DeviceGlyph category={d.category} />{d.name}</button>)}</details>)}{!matches.length && <p role="status">{!loaded && busy ? "Loading…" : "No devices found."}</p>}</nav>
    <section className="device-properties" aria-label="Device details">{device && <><h3>{device.name}</h3><p className="device-status">{device.status || "Unavailable"}</p><dl>{device.properties.filter(([, value]) => value).map(([key, value]) => <div key={key}><dt>{key}</dt><dd>{value}</dd></div>)}</dl><div className="settings-actions">{device.actions.map(action => <button key={action} disabled={busy} onClick={() => (["disconnect", "off", "pause", "unmount", "eject"].includes(action) || action.startsWith("eject:")) ? setChoice({ device, action }) : void act(device, action)}>{label(action)}</button>)}{device.category === "Wi-Fi adapters" && <button onClick={() => navigate("Wi-Fi")}>Networks…</button>}{device.category === "Bluetooth" && <button onClick={() => navigate("Bluetooth")}>Pair Devices…</button>}</div></>}</section></div>
    {choice && <div className="device-dialog-scrim"><section ref={dialog} role="dialog" aria-modal="true" aria-label="Confirm device change" className="device-dialog"><h3>{label(choice.action)}?</h3><p>{choice.device.name}{choice.action === "unmount" ? " will be unmounted. Other partitions may remain mounted; eject the whole device before unplugging." : choice.action.startsWith("eject") ? " will be unmounted. Wait for completion before unplugging. Busy devices will remain connected." : choice.action === "disconnect" ? " will disconnect from the network." : choice.action === "off" ? " and its connected Bluetooth devices will disconnect." : " will stop printing until resumed."}</p><div className="settings-actions"><button disabled={busy} onClick={() => setChoice(null)}>Cancel</button><button disabled={busy} onClick={() => void act(choice.device, choice.action)}>{busy ? "Applying…" : label(choice.action)}</button></div></section></div>}
  </div>;
}
