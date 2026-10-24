import { Dialog } from "../../components/ui/dialog";
import { Button } from "../../components/ui/button";
import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import type { DriverInfo } from "../../lib/generated/DriverInfo";
export function Drivers() {
  const [drivers, setDrivers] = useState<DriverInfo[]>([]), [search, setSearch] = useState("");
  const [filter, setFilter] = useState("loaded"), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const [choice, setChoice] = useState<DriverInfo | null>(null), [message, setMessage] = useState("");
  const pending = useRef(false);
  async function load() {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError("");
    try { setDrivers(await request({ method: "settings.drivers" }) ?? []); }
    catch (e) { setError(errorMessage(e)); } finally { pending.current = false; setBusy(false); }
  }
  useEffect(() => { void load(); }, []);
  async function apply() {
    if (!choice || pending.current) return;
    pending.current = true; setBusy(true); setError(""); setMessage("");
    try {
      await request({ method: "settings.driver_set", params: { name: choice.name, enabled: !choice.loaded } });
      setMessage(`${choice.name} ${choice.loaded ? "disabled" : "enabled"}.`); setChoice(null);
    } catch (e) { setError(errorMessage(e)); setChoice(null); }
    finally {
      try { setDrivers(await request({ method: "settings.drivers" }) ?? []); } catch (e) { setError(previous => previous || errorMessage(e)); }
      pending.current = false; setBusy(false);
    }
  }
  const matches = drivers.filter(d => (filter === "all" || (filter === "loaded" ? d.loaded : !d.loaded)) && `${d.name} ${d.path} ${d.devices.join(" ")}`.toLowerCase().includes(search.trim().toLowerCase()));
  return <>
    <p className="section-caption">Manage kernel modules for the running kernel. Changes last until reboot or automatic hardware detection reloads a driver. Built-in drivers cannot be disabled here.</p>
    <div className="settings-actions management-toolbar"><input aria-label="Search drivers" placeholder="Search driver, device or module path" value={search} onChange={e => setSearch(e.target.value)} /><select aria-label="Driver filter" value={filter} onChange={e => setFilter(e.target.value)}><option value="loaded">Enabled</option><option value="available">Not loaded</option><option value="all">All drivers</option></select><Button disabled={busy} onClick={() => void load()}>Refresh</Button></div>
    <p role="status" className="section-caption">{busy ? "Reading driver state…" : `${matches.length} drivers · ${drivers.filter(d => d.loaded).length} enabled`}{message && ` · ${message}`}</p>
    {error && <p role="alert" className="settings-error">{error}</p>}
    <div className="settings-card">{matches.slice(0, 100).map(d => <div className="settings-row" key={d.name}><div className="management-details"><strong>{d.name}</strong><small className="row-detail">{d.builtin ? "Built into kernel" : d.loaded ? `Enabled · ${d.users} references` : "Not loaded"}{d.devices.length > 0 && ` · ${d.devices.join(", ")}`}</small><small className="row-detail management-path">{d.path}</small></div><Button disabled={busy || d.builtin || (d.loaded && d.users > 0)} title={d.loaded && d.users > 0 ? "Driver is in use" : undefined} onClick={() => setChoice(d)}>{d.builtin ? "Built-in" : d.loaded ? "Disable…" : "Enable…"}</Button></div>)}{!busy && !matches.length && <p className="device-empty">No drivers match. Try another filter or search.</p>}</div>
    {matches.length > 100 && <p className="section-caption">Showing the first 100 results. Search to narrow the list.</p>}
    {choice && <Dialog open onClose={() => setChoice(null)} busy={busy} scrimClassName="device-dialog-scrim" aria-label="Confirm driver change" className="device-dialog"><h3>{choice.loaded ? "Disable" : "Enable"} {choice.name}?</h3><p>{choice.loaded ? "Disabling this module affects every device using it. Your network, display, input or storage may stop working." : "Load this kernel module and its dependencies for this session?"}</p><p>Administrator authentication is required. Changes do not edit boot settings.</p><div className="settings-actions"><Button disabled={busy} onClick={() => setChoice(null)}>Cancel</Button><Button disabled={busy} onClick={() => void apply()}>{busy ? "Authenticating…" : choice.loaded ? "Disable driver" : "Enable driver"}</Button></div></Dialog>}
  </>;
}
