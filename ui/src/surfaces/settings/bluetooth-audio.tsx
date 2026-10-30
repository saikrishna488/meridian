import { useEffect, useRef, useState } from "react";
import { Button } from "../../components/ui/button";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import type { BluetoothAudioMode } from "../../lib/generated/BluetoothAudioMode";
import type { BluetoothAudioState } from "../../lib/generated/BluetoothAudioState";

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
  const options: { mode: BluetoothAudioMode; label: string; description: string }[] = [
    { mode: "auto", label: "Auto", description: "Recommended. Let the system negotiate the best supported quality." },
    { mode: "high-quality", label: "High quality", description: "Prefer higher fidelity when your headset and connection support it." },
    { mode: "medium-quality", label: "Medium quality", description: "Balance sound quality and wireless reliability." },
    { mode: "low-quality", label: "Low quality", description: "Prefer a more reliable connection when wireless conditions are poor." },
  ];
  return <section className="settings-card bluetooth-codecs sound-list-section" aria-labelledby="bluetooth-quality-title">
    <div className="sound-section-heading"><h3 id="bluetooth-quality-title">Bluetooth audio quality</h3><p>Choose a simple quality preference. Meridian uses your system's supported Bluetooth audio capabilities.</p></div>
    <div className="sound-option-list" role="group" aria-label="Bluetooth audio quality">
      {options.map(option => <Button key={option.mode} className="sound-option-row" aria-pressed={audio.mode === option.mode} disabled={busy} onClick={() => void setQuality(option.mode)}>
        <span className="sound-option-indicator" aria-hidden="true">{audio.mode === option.mode && "✓"}</span>
        <span className="sound-option-copy"><strong>{option.label}</strong><small>{option.description}</small></span>
      </Button>)}
    </div>
    {audio.active_device && <p className="section-caption card-caption">Connected device: {audio.active_device}</p>}
    <p className="section-caption card-caption">Manual quality presets affect supported settings such as LDAC bitrate. Other Bluetooth formats continue to negotiate automatically; actual results depend on the device.</p>
    {error && <p className="settings-error" role="alert">{error}</p>}{notice && <p className="startup-message" role="status">{notice}</p>}
  </section>;
}
