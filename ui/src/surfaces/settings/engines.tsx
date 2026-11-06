import { Container, SettingRow } from "../../meridian-ui";
import { useEffect, useRef, useState } from "react";
import { Button } from "../../meridian-ui";
import { Switch } from "../../meridian-ui";
import { on, request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import type { EngineState } from "../../lib/generated/EngineState";

type EngineKey = "font_enabled" | "gpu_enabled" | "cpu_enabled" | "audio_enabled";
const engines: { key: EngineKey; name: string; description: string }[] = [
  { key:"font_enabled", name:"Font engine", description:"Grayscale antialiasing and crisp hinting for Meridian text." },
  { key:"gpu_enabled", name:"GPU engine", description:"Request accelerated rendering and detailed icon artwork. Availability depends on your driver." },
  { key:"cpu_enabled", name:"CPU engine", description:"Coalesce timers and reduce Meridian animations and transparency to lower visual workload." },
  { key:"audio_enabled", name:"Audio engine", description:"Higher-quality audio resampling. Keeps your device's volume and sample-rate negotiation." },
];
export function Engines() {
  const [state, setState] = useState<EngineState | null>(null);
  const [busy, setBusy] = useState<EngineKey | "load" | null>(null);
  const [error, setError] = useState("");
  const alive = useRef(true), pending = useRef(false), revision = useRef(0);
  useEffect(() => {
    alive.current = true;
    const off = on("engines.changed", event => { revision.current++; if (alive.current) setState(event.data); });
    const mine = revision.current;
    void request({ method:"engines.get" }).then(value => { if (alive.current && revision.current === mine) setState(value); })
      .catch(cause => { if (alive.current) setError(errorMessage(cause)); });
    return () => { alive.current = false; off(); };
  }, []);
  async function load() {
    if (pending.current) return;
    pending.current = true; setBusy("load"); setError("");
    try { const value = await request({ method:"engines.get" }); if (alive.current) setState(value); }
    catch (cause) { if (alive.current) setError(errorMessage(cause)); }
    finally { pending.current = false; if (alive.current) setBusy(null); }
  }
  async function toggle(key: EngineKey, enabled: boolean) {
    if (!state || pending.current) return;
    pending.current = true; revision.current++; setBusy(key); setError("");
    try {
      const value = await request({ method:"engines.set", params:{ ...state.config, [key]:enabled } });
      if (alive.current) setState(value);
    } catch (cause) { if (alive.current) setError(errorMessage(cause)); }
    finally { pending.current = false; if (alive.current) setBusy(null); }
  }
  if (!state) return <section className="settings-card settings-fallback" role={error ? "alert" : "status"}><p>{error || "Loading engines…"}</p>{error && <Button disabled={busy !== null} onClick={() => void load()}>Retry</Button>}</section>;
  return <div className="engine-settings" aria-busy={busy !== null}>
    <p className="section-caption">Four engines. Turn each on or off; changes save automatically.</p>
    {state.restart_required && <p className="settings-notice" role="status">Font, GPU, or CPU changes are saved. Sign out and back in to apply them consistently.</p>}
    <Container className="settings-card engine-switches" role="group" aria-label="Meridian engines">
      {engines.map(engine => <SettingRow key={engine.key}
        title={<span id={`${engine.key}-label`}>{engine.name}</span>}
        description={<span id={`${engine.key}-description`}>{engine.description}</span>}
        status={busy === engine.key ? "Saving…" : engine.key === "font_enabled" && state.config.font_enabled ? state.font_status : engine.key === "cpu_enabled" && state.config.cpu_enabled ? state.cpu_status : engine.key === "audio_enabled" ? state.audio_status : undefined}>
        <Switch checked={state.config[engine.key] === true} disabled={busy !== null} labelledBy={`${engine.key}-label`}
          describedBy={`${engine.key}-description`} onChange={enabled => void toggle(engine.key, enabled)} />
      </SettingRow>)}
    </Container>
    {error && <p className="settings-error" role="alert">{error}</p>}
  </div>;
}
