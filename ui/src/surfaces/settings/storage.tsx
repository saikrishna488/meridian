import { useEffect, useRef, useState } from "react";
import { Button } from "../../components/ui/button";
import { Dialog } from "../../components/ui/dialog";
import { Progress } from "../../components/ui/progress";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import type { StorageState } from "../../lib/generated/StorageState";
import type { StorageVolume } from "../../lib/generated/StorageVolume";
const bytes = (n: number) => new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(n / 1e9) + " GB";
const labels: Record<string, string> = { open: "Open in Files", mount: "Mount", unmount: "Unmount", eject: "Eject safely" };
export function Storage() {
  const [state, setState] = useState<StorageState | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const [capability, setCapability] = useState<import("../../lib/generated/SystemCapability").SystemCapability | null>(null);
  const [choice, setChoice] = useState<{ volume: StorageVolume; action: string } | null>(null);
  const alive = useRef(true), pending = useRef(false);
  async function load() {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError("");
    try {
      const [result, capability] = await Promise.allSettled([request({ method: "settings.storage" }), request({ method: "settings.capability", params: { feature: "storage" } })]);
      if (!alive.current) return;
      if (capability.status === "fulfilled") setCapability(capability.value);
      if (result.status === "rejected") throw result.reason;
      setState(result.value);
    }
    catch (e) { if (alive.current) setError(errorMessage(e)); }
    finally { pending.current = false; if (alive.current) setBusy(false); }
  }
  useEffect(() => { alive.current = true; void load(); return () => { alive.current = false; }; }, []);
  async function act(volume: StorageVolume, action: string) {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError("");
    try {
      await request({ method: "settings.storage_action", params: { id: volume.path, action, identity: volume.identity ?? null } });
      const result = await request({ method: "settings.storage" });
      if (alive.current) { setState(result); setChoice(null); }
    } catch (e) { if (alive.current) setError(errorMessage(e)); }
    finally { pending.current = false; if (alive.current) setBusy(false); }
  }
  const used = state ? Math.max(0, state.total - state.available) : 0;
  return <div aria-busy={busy}>
    <div className="settings-actions"><Button disabled={busy} onClick={() => void load()}>{busy ? "Refreshing…" : "Refresh storage"}</Button></div>
    {error && <p role="alert" className="settings-error">{error} <Button disabled={busy} onClick={() => void load()}>Retry</Button></p>}
    {capability && capability.state !== "ready" && <p role="status">{capability.message} Storage information remains available.</p>}
    {state && <><section className="settings-card storage-overview" aria-label="System filesystem">
      <h3>System filesystem · /</h3>{state.total > 0 ? <><p>{bytes(used)} used of {bytes(state.total)} · {Math.round(used / state.total * 100)}%</p><Progress aria-label="System filesystem usage" max={state.total} value={used} /><p>{bytes(state.available)} available</p></> : <p>Capacity unavailable</p>}
      <p className="section-caption">{state.source} · {state.filesystem || "Unknown filesystem"}</p>{state.warning && <p role="status">{state.warning}</p>}
    </section><h3>Disks and volumes</h3>{!state.volumes.length && <p role="status">No block devices found. Network and virtual filesystems may only appear in the system overview.</p>}
    {state.volumes.map(volume => <section className="settings-card storage-volume" key={volume.path} aria-label={volume.name}>
      <div className="settings-row"><strong>{volume.name}</strong><span>{volume.size > 0 ? bytes(volume.size) : "Capacity unavailable"}</span></div>
      <p>{volume.path.startsWith("/dev/") ? volume.path : volume.mount_points[0]} · {volume.kind}{volume.removable ? " · Removable" : ""}{volume.read_only ? " · Read-only" : ""}</p>
      <p className="section-caption">{volume.filesystem || "No accessible filesystem"} · {volume.mount_points.length ? volume.mount_points.join(", ") : "Not mounted"}{volume.parent ? ` · On ${volume.parent}` : ""}</p>
      <div className="settings-actions">{volume.actions.map(action => <Button key={action} disabled={busy || (action !== "open" && capability != null && capability.state !== "ready")} onClick={() => action === "unmount" || action === "eject" ? setChoice({ volume, action }) : void act(volume, action)}>{labels[action] ?? action}</Button>)}</div>
    </section>)}</>}
    <Dialog open={!!choice} onClose={() => setChoice(null)} busy={busy} aria-labelledby="storage-confirm-title" className="device-dialog" scrimClassName="device-dialog-scrim">
      <h3 id="storage-confirm-title">{choice && labels[choice.action]}?</h3><p>Close files on {choice?.volume.name} first. Busy volumes will remain connected. Wait for completion before unplugging a device.</p>
      <div className="settings-actions"><Button data-autofocus disabled={busy} onClick={() => setChoice(null)}>Cancel</Button><Button disabled={busy} onClick={() => choice && void act(choice.volume, choice.action)}>{busy ? "Working…" : "Continue"}</Button></div>
    </Dialog>
  </div>;
}
