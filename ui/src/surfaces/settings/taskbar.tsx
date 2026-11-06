import { useRef, useState } from "react";
import { Alert, Button, ConfirmDialog, Section, SettingRow, Select, Switch } from "../../meridian-ui";
import { useDesktop } from "../../lib/desktop";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import type { TaskbarPreferences } from "../../lib/generated/TaskbarPreferences";
export function TaskbarSettings() {
  const { desktop, setDesktop, error: loadError } = useDesktop();
  const pending = useRef(false), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const [saved, setSaved] = useState(false), [reset, setReset] = useState(false);
  const value = desktop?.taskbar ?? {icon_size:32, auto_hide:false, clock_format:"system", battery_percentage:true};
  async function save(change: Partial<TaskbarPreferences>) {
    if (pending.current || !desktop) return;
    pending.current = true; setBusy(true); setSaved(false); setError("");
    try { setDesktop(await request({method:"desktop.taskbar_set", params:{...value,...change}})); setSaved(true); }
    catch (cause) { setError(errorMessage(cause)); }
    finally { pending.current = false; setBusy(false); }
  }
  const disabled = busy || !desktop;
  return <Section title="Taskbar" description="Changes are saved and applied across the desktop.">
    <SettingRow title="App icon size"><Select aria-label="App icon size" value={String(value.icon_size)} disabled={disabled} onChange={event => void save({icon_size:Number(event.target.value)})}><option value="24">Small</option><option value="32">Medium</option><option value="40">Large</option></Select></SettingRow>
    <SettingRow title="Auto-hide" description="Reveal the taskbar at the bottom edge. Keyboard focus and open menus keep it visible."><Switch label="Auto-hide taskbar" checked={value.auto_hide} disabled={disabled} onChange={auto_hide => void save({auto_hide})} /></SettingRow>
    <SettingRow title="Clock format"><Select aria-label="Clock format" value={value.clock_format} disabled={disabled} onChange={event => void save({clock_format:event.target.value})}><option value="system">System default</option><option value="12">12-hour</option><option value="24">24-hour</option></Select></SettingRow>
    <SettingRow title="Battery percentage"><Switch label="Show battery percentage" checked={value.battery_percentage} disabled={disabled} onChange={battery_percentage => void save({battery_percentage})} /></SettingRow>
    <p role="status">{busy ? "Saving…" : saved ? "Saved · Applied immediately" : ""}</p>
    <Button disabled={disabled} onClick={() => setReset(true)}>Restore taskbar defaults…</Button>
    <ConfirmDialog open={reset} title="Restore taskbar defaults?" confirmLabel="Restore defaults" onClose={() => setReset(false)} onConfirm={async () => {
      if (pending.current || !desktop) throw new Error("Taskbar preferences are not ready.");
      pending.current = true; setBusy(true); setSaved(false); setError("");
      try { setDesktop(await request({method:"desktop.taskbar_set", params:{icon_size:32, auto_hide:false, clock_format:"system", battery_percentage:true}})); setSaved(true); }
      finally { pending.current = false; setBusy(false); }
    }}>Use medium app icons, a visible taskbar, system clock format and battery percentage. Pinned apps are preserved.</ConfirmDialog>
    {(error || loadError) && <Alert tone="error">{error || loadError}</Alert>}
  </Section>;
}
