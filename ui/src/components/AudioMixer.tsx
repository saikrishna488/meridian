import { useEffect, useRef, useState } from "react";
import { Button, Select, Slider, Disclosure } from "../meridian-ui";
import { request } from "../lib/bridge";
import { errorMessage } from "../lib/react";
import type { AudioState } from "../lib/generated/AudioState";

/** Mounted only while Control Center is visible: no background audio polling. */
export function AudioMixer() {
  const [audio, setAudio] = useState<AudioState | null>(null), [error,setError] = useState("");
  const [revision,setRevision]=useState(0);
  const [busy,setBusy] = useState(false), pending = useRef(false), alive = useRef(true);
  async function load() {
    if (pending.current) return;
    pending.current = true; setBusy(true);
    try { const value = await request({method:"settings.audio"}); if (alive.current) { setAudio(value); setError(""); } }
    catch (e) { if (alive.current) setError(errorMessage(e)); }
    finally { pending.current = false; if (alive.current) {setBusy(false);setRevision(n=>n+1);} }
  }
  useEffect(() => { alive.current = true; void load(); return () => { alive.current = false; }; }, []);
  async function act(id: number, name: string, action: string, percent: number | null = null) {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError("");
    try { const value = await request({method:"settings.audio_action",params:{id,name,action,percent}}); if (alive.current) setAudio(value); }
    catch (e) { if (alive.current) setError(errorMessage(e)); }
    finally { pending.current = false; if (alive.current) {setBusy(false);setRevision(n=>n+1);} }
  }
  const outputs = audio?.devices.filter(d => d.kind === "output") ?? [];
  return <div className="audio-mixer">
    <label>Playback device<Select aria-label="Playback device" disabled={busy || !outputs.length} value={outputs.find(d=>d.default)?.id.toString() ?? ""} onChange={e=>{const d=outputs.find(d=>d.id.toString()===e.target.value);if(d)void act(d.id,d.name,"default");}}>
      <option value="" disabled>{outputs.length ? "Choose output" : "No outputs available"}</option>{outputs.map(d=><option key={d.id} value={d.id}>{d.description}</option>)}
    </Select></label>
    <Disclosure title="Application volumes">
      {(audio?.streams ?? []).map(stream=><div key={`${stream.id}:${stream.name}`} className="mixer-stream"><strong>{stream.description}</strong>
        <Slider resetToken={revision} aria-label={`${stream.description} volume`} min={0} max={100} value={stream.volume ?? 0} disabled={busy || stream.volume === null} onChangeEnd={percent=>void act(stream.id,stream.name,"volume",percent)} />
        <Button disabled={busy || stream.muted === null} aria-pressed={stream.muted === true} onClick={()=>void act(stream.id,stream.name,stream.muted?"unmute":"mute")}>{stream.muted?"Unmute":"Mute"}</Button>
      </div>)}
      {audio && !audio.streams.length && <p>Play audio in an app, then refresh to adjust its volume.</p>}
      <Button disabled={busy} onClick={()=>void load()}>{busy?"Refreshing…":"Refresh audio"}</Button>
    </Disclosure>
    {error && <p role="alert">{error} <Button disabled={busy} onClick={()=>void load()}>Retry</Button></p>}
  </div>;
}
