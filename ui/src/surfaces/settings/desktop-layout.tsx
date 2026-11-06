import { useRef, useState } from "react";
import { Alert, Button, ConfirmDialog, Section, SettingRow, Select, Switch } from "../../meridian-ui";
import { useDesktop } from "../../lib/desktop";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import type { DesktopLayout } from "../../lib/generated/DesktopLayout";
export function DesktopLayoutSettings() {
  const { desktop, setDesktop, error: loadError } = useDesktop();
  const pending = useRef(false);
  const [busy, setBusy] = useState(false), [saved, setSaved] = useState(false), [error, setError] = useState("");
  const [reset, setReset] = useState(false);
  const value = desktop?.layout;
  async function save(change: Partial<DesktopLayout>) {
    if (!value || pending.current) return;
    pending.current = true; setBusy(true); setSaved(false); setError("");
    try { setDesktop(await request({method:"desktop.layout_set", params:{...value,...change}})); setSaved(true); }
    catch (cause) { setError(errorMessage(cause)); throw cause; }
    finally { pending.current = false; setBusy(false); }
  }
  const update = (change: Partial<DesktopLayout>) => { void save(change).catch(() => {}); };
  return <Section title="Desktop & window controls" description="Layout changes apply immediately. Button placement applies to Meridian app windows. Right-click Maximize or press Alt+Space for window layout; Super+D shows or restores the desktop in a Meridian session.">
    <SettingRow title="Desktop icon size"><Select aria-label="Desktop icon size" disabled={busy || !value} value={String(value?.icon_size ?? 64)} onChange={e => update({icon_size:Number(e.target.value)})}><option value="48">Small</option><option value="64">Medium</option><option value="80">Large</option></Select></SettingRow>
    <SettingRow title="Desktop icon spacing"><Select aria-label="Desktop icon spacing" disabled={busy || !value} value={String(value?.spacing ?? 16)} onChange={e => update({spacing:Number(e.target.value)})}><option value="8">Compact</option><option value="16">Comfortable</option><option value="24">Spacious</option></Select></SettingRow>
    <SettingRow title="Snap to grid" description="Align manually dragged icons to the grid."><Switch label="Snap to grid" disabled={busy || !value} checked={value?.snap_to_grid ?? true} onChange={snap_to_grid => update({snap_to_grid})} /></SettingRow>
    <SettingRow title="Show desktop icons" description="Hiding icons preserves their saved positions."><Switch label="Show desktop icons" disabled={busy || !value} checked={value?.show_icons ?? true} onChange={show_icons => update({show_icons})} /></SettingRow>
    <SettingRow title="Window button placement"><Select aria-label="Window button placement" disabled={busy || !value} value={value?.window_buttons ?? "left"} onChange={e => update({window_buttons:e.target.value})}><option value="left">Left</option><option value="right">Right</option></Select></SettingRow>
    <p role="status">{busy ? "Saving…" : saved ? "Saved · Applied immediately" : ""}</p>
    {(error || loadError) && <Alert tone="error">{error || loadError}</Alert>}
    <Button disabled={busy || !value} onClick={() => setReset(true)}>Restore desktop defaults…</Button>
    <ConfirmDialog open={reset} title="Restore desktop defaults?" confirmLabel="Restore defaults" onClose={() => setReset(false)} onConfirm={() => save({icon_size:64, spacing:16, snap_to_grid:true, show_icons:true, window_buttons:"left"})}>Use medium icons, comfortable spacing, grid snapping, visible icons and left-side window buttons. Your shortcuts and saved icon positions are preserved.</ConfirmDialog>
  </Section>;
}
