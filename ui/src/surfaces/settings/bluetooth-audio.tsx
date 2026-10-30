import { useEffect, useRef, useState } from "react";
import { Button } from "../../components/ui/button";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import type { BluetoothAudioMode } from "../../lib/generated/BluetoothAudioMode";
import type { BluetoothAudioState } from "../../lib/generated/BluetoothAudioState";

const codecs = [
  ["SBC", "Medium quality", "Low stutter risk", "Broad compatibility"], ["SBC-XQ", "High quality", "Range sensitive", "Higher bitrate"], ["AAC", "Medium–high quality", "Device dependent", "Common on phones"], ["aptX", "High quality", "Low latency", "Device support required"], ["aptX HD", "Very high quality", "Range sensitive", "Higher bitrate"], ["aptX LL", "Medium quality", "Very low latency", "Gaming / video"], ["aptX LL Duplex", "Low quality", "Very low latency", "Two-way audio"], ["FastStream", "Medium quality", "Low latency", "Two-way audio"], ["FastStream Duplex", "Medium quality", "Low latency", "Two-way audio"], ["LDAC", "Very high quality", "Stutter possible at range", "High bitrate modes"], ["LC3", "High efficiency", "Low latency", "LE Audio devices"], ["LC3plus H3", "High quality", "Device dependent", "Hardware support required"], ["Opus 0.5", "High quality", "Low latency", "Stereo profile"], ["Opus 0.5 5.1", "High quality", "Device dependent", "5.1 profile"], ["Opus 0.5 7.1", "High quality", "Device dependent", "7.1 profile"], ["Opus 0.5 Duplex", "Medium quality", "Low latency", "Two-way audio"], ["Opus 0.5 Pro", "Very high quality", "Device dependent", "Pro profile"],
] as const;

export function BluetoothAudioSettings() {
  const [audio, setAudio] = useState<BluetoothAudioState>({ mode: "auto", active_codec: null, active_device: null });
  const [busy, setBusy] = useState(false), [error, setError] = useState(""), [notice, setNotice] = useState("");
  const pending = useRef(false), alive = useRef(true);
  async function load() { try { const result = await request({ method: "settings.bluetooth_audio" }); if (alive.current) setAudio(result); } catch (e) { if (alive.current) setError(errorMessage(e)); } }
  useEffect(() => {
    alive.current = true; void load();
    const refresh = () => { if (!document.hidden) void load(); };
    const timer = window.setInterval(refresh, 4000);
    window.addEventListener("focus", refresh); document.addEventListener("visibilitychange", refresh);
    return () => { alive.current = false; window.clearInterval(timer); window.removeEventListener("focus", refresh); document.removeEventListener("visibilitychange", refresh); };
  }, []);
  async function setQuality(mode: BluetoothAudioMode) {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError(""); setNotice("");
    try { const result = await request({ method: "settings.bluetooth_audio_set", params: { mode } }); if (alive.current) { setAudio(result); setNotice("Preference saved. Bluetooth audio may briefly reconnect while WirePlumber restarts."); } }
    catch (e) { if (alive.current) setError(errorMessage(e)); }
    finally { pending.current = false; if (alive.current) setBusy(false); }
  }
  return <section className="settings-card bluetooth-codecs"><div><h3>Bluetooth audio</h3><p>PipeWire negotiates a codec supported by both your system and headset. Availability depends on installed codec plugins and the connected device.</p></div>
    <div className="bluetooth-active-codec"><span>Active codec</span><strong>{audio.active_codec ? audio.active_codec.toUpperCase().replaceAll("_", " ") : "No Bluetooth audio output"}</strong>{audio.active_device && <small>{audio.active_device}</small>}</div>
    <div className="bluetooth-quality-setting"><div><strong>Quality preference</strong><small>Choose a Bluetooth streaming bitrate. Higher quality uses more bandwidth and may be less stable at range; Auto lets PipeWire adapt.</small></div><div className="bluetooth-quality-options" role="group" aria-label="Bluetooth audio quality preference">{([["high-quality", "High quality"], ["medium-quality", "Medium quality"], ["low-quality", "Low quality"], ["auto", "Auto"]] as const).map(([mode, label]) => <Button key={mode} disabled={busy || audio.mode === mode} aria-pressed={audio.mode === mode} onClick={() => void setQuality(mode)}>{label}</Button>)}</div></div>
    {error && <p className="settings-error" role="alert">{error}</p>}{notice && <p className="startup-message" role="status">{notice}</p>}
    <div className="codec-list">{codecs.map(([codec, quality, stability, detail]) => <div className="codec-row" key={codec}><strong>{codec}</strong><span>{quality}</span><span>{stability}</span><small>{detail}</small></div>)}</div>
    <small className="codec-note">Codec names describe common tradeoffs. Actual sound quality, delay and connection stability vary by headset, radio conditions and system support. Auto adapts LDAC quality; High, Medium and Low request the corresponding LDAC bitrate when the headset supports LDAC. Other codecs are negotiated by PipeWire.</small>
  </section>;
}
