import { Button } from "../../components/ui/button";
import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import { Icon } from "../../components/Icon";
import { Slider } from "../../components/ui/slider";
import { Select } from "../../components/Select";
import { SystemSlider } from "../../components/SystemSlider";
import { useSystemSettings } from "../../lib/settings";
import type { AudioState } from "../../lib/generated/AudioState";
import type { AudioDevice } from "../../lib/generated/AudioDevice";
import type { AudioAction } from "../../lib/generated/AudioAction";
import { BluetoothAudioSettings } from "./bluetooth-audio";

function AudioLevel({ label, value, disabled, save }: { label: string; value: number | null; disabled: boolean; save: (percent: number) => void }) {
  const [local, setLocal] = useState(value ?? 0), editing = useRef(false);
  useEffect(() => { if (!editing.current) setLocal(value ?? 0); }, [value]);
  function commit(percent: number) { editing.current = false; if (!disabled && percent !== value) save(percent); }
  return <div className="audio-level"><Slider min={0} max={100} aria-label={label} disabled={disabled || value === null} value={local} onValueChange={value => { editing.current = true; setLocal(value); }} onPointerUp={e => commit(Number(e.currentTarget.value))} onKeyUp={e => { if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End", "PageUp", "PageDown"].includes(e.key)) commit(Number(e.currentTarget.value)); }} onBlur={e => commit(Number(e.currentTarget.value))} /><output>{value === null ? "Unavailable" : `${local}%`}</output></div>;
}
export function Sound() {
  const [audio, setAudio] = useState<AudioState | null>(null), [busy, setBusy] = useState(false), [loaded, setLoaded] = useState(false);
  const [error, setError] = useState(""), [message, setMessage] = useState("");
  const { settings } = useSystemSettings();
  const pending = useRef(false), alive = useRef(true);
  async function load(quiet = false) {
    if (pending.current) return;
    pending.current = true; if (!quiet) { setBusy(true); setError(""); }
    try { const result = await request({ method: "settings.audio" }); if (alive.current) { setAudio(result); setLoaded(true); } }
    catch (e) { if (alive.current && !quiet) { setError(errorMessage(e)); setLoaded(true); } }
    finally { pending.current = false; if (alive.current) setBusy(false); }
  }
  useEffect(() => {
    alive.current = true; void load();
    const refresh = () => { if (!document.hidden) void load(true); };
    window.addEventListener("focus", refresh); document.addEventListener("visibilitychange", refresh);
    return () => { alive.current = false; window.removeEventListener("focus", refresh); document.removeEventListener("visibilitychange", refresh); };
  }, []);
  async function act(device: AudioDevice, action: AudioAction["action"], percent: number | null = null) {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError(""); setMessage("");
    try { const result = await request({ method: "settings.audio_action", params: { id: device.id, name: device.name, action, percent } }); if (alive.current) { setAudio(result); if (action === "default") setMessage(`${device.description} is selected for ${device.kind === "input" ? "recording" : "playback"}.`); } }
    catch (e) { if (alive.current) setError(errorMessage(e)); }
    finally { pending.current = false; if (alive.current) setBusy(false); }
  }
  async function virtualOutput(action: "virtual-create" | "virtual-remove") {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError(""); setMessage("");
    try {
      const result = await request({ method: "settings.audio_action", params: { id: 0, name: "", action, percent: null } });
      if (alive.current) { setAudio(result); setMessage(action === "virtual-create" ? "Virtual output is ready. Select it from the playback device list." : "Virtual output removed."); }
    } catch (e) { if (alive.current) setError(errorMessage(e)); }
    finally { pending.current = false; if (alive.current) setBusy(false); }
  }
  return <>
    <div className="sound-intro"><p className="section-caption">Choose where sound plays and which microphone apps use.</p><Button aria-label="Refresh audio devices" disabled={busy} onClick={() => void load()}><Icon name="refresh" width="15" height="15" />Refresh devices</Button></div>
    {!loaded && <p className="device-empty" role="status">Finding audio devices…</p>}
    {audio ? (["output", "input"] as const).map(kind => {
      const devices = audio.devices.filter(d => d.kind === kind), selected = devices.find(d => d.default);
      const volume = kind === "output" ? audio.output_volume : audio.input_volume, muted = kind === "output" ? audio.output_muted : audio.input_muted;
      return <section key={kind} className="settings-card sound-card" aria-labelledby={`sound-${kind}-title`}><header><span className="sound-card-icon"><Icon name={kind === "output" ? "sound" : "microphone"} width="23" height="23" /></span><div><h3 id={`sound-${kind}-title`}>{kind === "output" ? "Output" : "Input"}</h3><p>{kind === "output" ? "Speakers, headphones, and other playback devices" : "Microphones and other recording devices"}</p></div></header>
        {devices.length ? <><div className="settings-row"><label htmlFor={`sound-${kind}-device`}>{kind === "output" ? "Play sound through" : "Record sound from"}</label><Select id={`sound-${kind}-device`} aria-label={kind === "output" ? "Output device" : "Input device"} disabled={busy} value={selected?.id ?? ""} onChange={e => { const device = devices.find(d => d.id === Number(e.target.value)); if (device) void act(device, "default"); }}><option value="" disabled>Choose a device</option>{devices.map(device => <option key={device.id} value={device.id}>{device.description}</option>)}</Select></div>
        <div className="sound-level-row"><div className="sound-level-heading"><span>{kind === "output" ? "Output volume" : "Microphone level"}</span><Button aria-label={muted ? `Unmute ${kind === "output" ? "speakers" : "microphone"}` : `Mute ${kind === "output" ? "speakers" : "microphone"}`} aria-pressed={muted === true} disabled={busy || !selected || muted === null} onClick={() => selected && void act(selected, muted ? "unmute" : "mute")}><Icon name={muted ? "muted" : kind === "output" ? "sound" : "microphone"} width="16" height="16" />{muted ? "Muted" : "Mute"}</Button></div><AudioLevel key={selected?.id ?? "none"} label={kind === "output" ? "Output volume" : "Microphone level"} value={volume} disabled={busy || !selected} save={percent => { if (selected) void act(selected, "volume", percent); }} />{muted && <p className="sound-muted-note">{kind === "output" ? "Sound is muted. Unmute to hear playback." : "Microphone is muted. Apps will not receive audio from this device."}</p>}</div></> : <div className="sound-no-device"><Icon name={kind === "output" ? "sound" : "microphone"} width="26" height="26" /><strong>No {kind === "output" ? "output devices" : "microphones"} connected</strong><p>Connect a device, then refresh to see it here.</p></div>}
        {kind === "output" && <div className="settings-row"><div><strong>Virtual output</strong><small className="row-detail">Create a software playback device for recording or routing audio.</small></div><Button disabled={busy} onClick={() => void virtualOutput(devices.some(device => device.name === "meridian_virtual_output") ? "virtual-remove" : "virtual-create")}>{devices.some(device => device.name === "meridian_virtual_output") ? "Remove virtual output" : "Add virtual output"}</Button></div>}
      </section>;
    }) : loaded ? <div className="settings-card"><div className="settings-row"><Icon name="sound" /><strong>Output volume</strong></div><SystemSlider name="volume" label="Volume" value={settings.volume} report={e => setError(errorMessage(e))} /><p className="section-caption card-caption">Device selection needs PipeWire and WirePlumber. Your existing volume control remains available when supported.</p></div> : null}
    {message && <p className="startup-message" role="status">{message}</p>}{error && <p className="settings-error" role="alert">{error}</p>}
    <BluetoothAudioSettings />
    <p className="section-caption">Changes apply to the default devices. Applications with their own audio device selection may keep that choice.</p>
  </>;
}
