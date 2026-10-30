import { Slider } from "../../components/ui/slider";
import { Button } from "../../components/ui/button";
import { Select } from "../../components/Select";
import { useEffect, useRef, useState } from "react";
import { on, request } from "../../lib/bridge";
import { errorMessage, useCallbackRef, useDialogFocus } from "../../lib/react";
import { useSystemSettings } from "../../lib/settings";
import { Toggle } from "../../components/Toggle";
import { SystemSlider } from "../../components/SystemSlider";
import type { DisplayOutput } from "../../lib/generated/DisplayOutput";
import type { DisplayConfig } from "../../lib/generated/DisplayConfig";
import type { DisplayTrial } from "../../lib/generated/DisplayTrial";

export function Displays() {
 const [primary, setPrimary] = useState<import("../../lib/generated/PrimaryDisplayState").PrimaryDisplayState | null>(null);
 const [capability, setCapability] = useState<import("../../lib/generated/SystemCapability").SystemCapability | null>(null);
 const alive = useRef(true), loading = useRef(false), deciding = useRef(false), applying = useRef(false);
 const trialDialog = useRef<HTMLElement>(null);
 const [outputs, setOutputs] = useState<DisplayOutput[]>([]), [edits, setEdits] = useState<DisplayConfig[]>([]), [selected, select] = useState(""), [busy,setBusy]=useState(false), [error,setError]=useState("");
 const [monitorBrightness,setMonitorBrightness]=useState<number|null>(null), [brightnessBusy,setBrightnessBusy]=useState(false);
 const brightnessRevision=useRef(0);
 const brightnessTimer=useRef<ReturnType<typeof setTimeout>|null>(null);
 const [trial,setTrial]=useState<DisplayTrial|null>(null), [seconds,setSeconds]=useState(0);
 const {settings}=useSystemSettings();
 const report=(e:unknown)=>setError(errorMessage(e));
 async function load() { if(loading.current)return;loading.current=true;try{const cap=await request({method:"settings.capability",params:{feature:"displays"}});if(!alive.current)return;setCapability(cap); const result=await request({method:"settings.displays"});if(!alive.current)return; setOutputs(result ?? []); setEdits((result ?? []).map(o=>{const m=o.current ?? o.modes.find(m=>m.preferred) ?? o.modes[0]; return { name:o.name,enabled:o.enabled,x:o.x,y:o.y,scale:o.scale,transform:o.transform,width:m?.width??0,height:m?.height??0,refresh:m?.refresh??0 };})); select(old=>result?.some(o=>o.name===old)?old:result?.[0]?.name??""); }finally{loading.current=false;} }
 useEffect(()=>{alive.current=true;void load().catch(report);void request({method:"settings.primary_display"}).then(value=>{if(alive.current)setPrimary(value);}).catch(report);return()=>{alive.current=false;};},[]);
 const refreshOutputs=useCallbackRef(()=>{if(busy||trial)return;void load().catch(report);});
 useEffect(()=>{let timer:ReturnType<typeof setTimeout>;const off=on("displays.changed",()=>{clearTimeout(timer);timer=setTimeout(refreshOutputs,200);});return()=>{off();clearTimeout(timer);};},[refreshOutputs]);
 async function primaryDisplay(connector:string) { if(busy)return;setBusy(true);setError("");try{setPrimary(await request({method:"settings.primary_display_set",params:{connector:connector||null}}));}catch(e){report(e);}finally{setBusy(false);} }
 useEffect(()=>{
  const revision=++brightnessRevision.current;
  if(brightnessTimer.current)clearTimeout(brightnessTimer.current);
  setMonitorBrightness(null);setBrightnessBusy(false);
  const output=outputs.find(o=>o.name===selected);
  if(!output||!output.enabled||/^(eDP|LVDS|DSI)/.test(output.name))return;
  setBrightnessBusy(true);
  void request({method:"settings.monitor_brightness",params:{connector:output.name,percent:null}})
    .then(value=>{if(revision===brightnessRevision.current)setMonitorBrightness(value.percent);})
    .catch(()=>{if(revision===brightnessRevision.current)setMonitorBrightness(null);})
    .finally(()=>{if(revision===brightnessRevision.current)setBrightnessBusy(false);});
  return ()=>{brightnessRevision.current++;};
 },[selected,outputs]);
 useEffect(()=>()=>{if(brightnessTimer.current)clearTimeout(brightnessTimer.current);},[]);
 function setExternalBrightness(percent:number){const previous=monitorBrightness,revision=++brightnessRevision.current;setMonitorBrightness(percent);if(brightnessTimer.current)clearTimeout(brightnessTimer.current);brightnessTimer.current=setTimeout(()=>{setBrightnessBusy(true);void request({method:"settings.monitor_brightness",params:{connector:selected,percent}}).catch(e=>{if(revision===brightnessRevision.current){setMonitorBrightness(previous);report(e);}}).finally(()=>{if(revision===brightnessRevision.current)setBrightnessBusy(false);});},250);}
 const readOnly = capability != null && capability.state !== "ready";
 const trialRef=useRef(trial); trialRef.current=trial;
 async function decide(keep:boolean) { const current=trialRef.current; if(!current||deciding.current)return; deciding.current=true; trialRef.current=null; setBusy(true);setError(""); try{await request({method:"settings.display_decide",params:{token:current.token,keep}});setTrial(null);await load();}catch(e){report(e);}finally{deciding.current=false;setBusy(false);} }
 useEffect(()=>{if(!trial)return; const deadline=Date.now()+trial.seconds*1000; setSeconds(trial.seconds);const interval=setInterval(()=>{const remaining=Math.max(0,Math.ceil((deadline-Date.now())/1000));setSeconds(remaining);if(!remaining){clearInterval(interval);void decide(false);}},250); return ()=>clearInterval(interval);},[trial]);
 function update(patch:Partial<DisplayConfig>){setEdits(old=>old.map(o=>o.name===selected?{...o,...patch}:o));}
 async function apply(){if(applying.current||busy||trial||readOnly)return;applying.current=true;setBusy(true);setError("");try{const value=await request({method:"settings.display_apply",params:{outputs:edits}});setSeconds(value.seconds);setTrial(value);}catch(e){report(e);}finally{applying.current=false;setBusy(false);}}
 useDialogFocus(trialDialog, !!trial, () => { if (!busy && trial) void decide(false); });
 const output=outputs.find(o=>o.name===selected), edit=edits.find(o=>o.name===selected), internal=!!output&&(output.name.startsWith("eDP")||output.name.startsWith("LVDS")||output.name.startsWith("DSI"));
 return <>{capability && capability.state !== "ready" && <p role="status" className="section-caption">{capability.message} Display information is read-only.</p>}<div className="settings-card"><div className="settings-row"><div><label htmlFor="primary-display">Primary display</label><small className="row-detail">The dock, top bar and menus appear here. If disconnected, another display is used until it returns.</small></div><Select id="primary-display" value={primary?.connector??""} disabled={!primary||busy||!!trial} onChange={e=>void primaryDisplay(e.target.value)}><option value="">Automatic</option>{primary?.connector&&!outputs.some(o=>o.name===primary.connector)&&<option value={primary.connector}>{primary.connector} (disconnected)</option>}{outputs.filter(o=>o.enabled).map(o=><option key={o.name} value={o.name}>{o.description||o.name}</option>)}</Select></div>{primary?.effective&&<p className="section-caption card-caption">Currently using {primary.effective}</p>}</div><p className="section-caption">Choose a display below. Settings for the selected display appear in a vertical list.</p>
 <section className="settings-card display-list-section" aria-label="Connected displays">
   <div className="sound-section-heading"><h3>Connected displays</h3><p>{outputs.length} detected</p></div>
   <div className="display-vertical-list">
     {outputs.map((o,i)=><Button key={o.name} className="display-list-row" aria-pressed={selected===o.name} onClick={()=>select(o.name)}>
       <span className="display-list-icon"><span>{i+1}</span></span>
       <span className="display-list-copy"><strong>{o.description || o.name}</strong><small>{o.name} · {o.enabled ? `${o.current?.width ?? o.modes[0]?.width ?? "—"} × ${o.current?.height ?? o.modes[0]?.height ?? "—"}` : "Disabled"}{o.enabled && o.current?.refresh ? ` · ${(o.current.refresh/1000).toFixed(0)} Hz` : ""}</small></span>
       <span className="display-list-state">{selected===o.name ? "Selected" : "Select"}</span>
     </Button>)}
     {!outputs.length && <p className="section-caption">No displays available. Reconnect a display and choose Detect displays.</p>}
   </div>
 </section>
 {edit&&output&&<div className="settings-card"><div className="settings-row"><strong>{output.description || output.name}</strong><Toggle label="Enable display" checked={edit.enabled} disabled={busy||!!trial||readOnly||(edit.enabled&&edits.filter(o=>o.enabled).length===1)} onChange={enabled=>update({enabled})}/></div><div className="settings-row"><label htmlFor="resolution">Resolution & refresh rate</label><Select id="resolution" disabled={busy||!!trial||readOnly} value={`${edit.width}:${edit.height}:${edit.refresh}`} onChange={e=>{const[width,height,refresh]=e.target.value.split(':').map(Number);update({width:width!,height:height!,refresh:refresh!});}}>{!output.modes.length&&output.current&&<option value={`${edit.width}:${edit.height}:${edit.refresh}`}>{edit.width} × {edit.height} · {(edit.refresh/1000).toFixed(2)} Hz</option>}{output.modes.map(m=><option key={`${m.width}:${m.height}:${m.refresh}`} value={`${m.width}:${m.height}:${m.refresh}`}>{m.width} × {m.height} · {(m.refresh/1000).toFixed(2)} Hz{m.preferred?" (recommended)":""}</option>)}</Select></div><div className="settings-row"><label htmlFor="display-scale">Scale</label><Select id="display-scale" value={edit.scale} disabled={busy||!!trial||readOnly} onChange={e=>update({scale:Number(e.target.value)})}>{[...new Set([1,1.25,1.5,1.75,2,2.5,3,edit.scale])].sort((a,b)=>a-b).map(v=><option key={v} value={v}>{Math.round(v*100)}%</option>)}</Select></div><div className="settings-row"><label htmlFor="rotation">Rotation</label><Select id="rotation" value={edit.transform} disabled={busy||!!trial||readOnly} onChange={e=>update({transform:Number(e.target.value)})}>{[0,1,2,3].map(v=><option key={v} value={v}>{v*90}°</option>)}{edit.transform>3&&<option value={edit.transform}>Flipped</option>}</Select></div><div className="settings-row"><span>Position in desktop</span><div className="display-position"><label>X<input type="number" aria-label="Display X position" min="-32768" max="32768" value={edit.x} disabled={busy||!!trial||readOnly} onChange={e=>update({x:Number(e.target.value)})}/></label><label>Y<input type="number" aria-label="Display Y position" min="-32768" max="32768" value={edit.y} disabled={busy||!!trial||readOnly} onChange={e=>update({y:Number(e.target.value)})}/></label></div></div></div>}
 <div className="settings-card">{internal?<SystemSlider name="brightness" label="Built-in display brightness" value={settings.brightness} report={report}/>:monitorBrightness===null?<p className="section-caption">{brightnessBusy?"Checking monitor brightness…":"This display does not expose DDC/CI brightness control."}</p>:<div className="monitor-brightness"><label htmlFor="monitor-brightness">Monitor brightness</label><Slider id="monitor-brightness" min={0} max={100} value={monitorBrightness??50} disabled={!output||!output.enabled||monitorBrightness===null||brightnessBusy} onValueChange={setExternalBrightness}/><span>{!output?"Select a display":!output.enabled?"Enable this display to adjust brightness":brightnessBusy?"Updating…":monitorBrightness===null?"DDC/CI brightness unavailable":""}</span></div>}</div><div className="settings-actions"><Button disabled={busy||!!trial} onClick={()=>void load().catch(report)}>Detect displays</Button><Button variant="primary" className="primary" disabled={busy||!!trial||!outputs.length||readOnly} onClick={()=>void apply()}>{busy?"Applying…":"Apply changes"}</Button></div>{error&&!trial&&<p role="alert" className="settings-error">{error}</p>}{trial&&<div className="device-dialog-scrim"><section ref={trialDialog} role="dialog" aria-modal="true" aria-label="Keep display settings" className="device-dialog"><h3>Keep these display settings?</h3><p>{seconds > 0 ? `Reverting automatically in ${seconds} seconds.` : "The confirmation period has ended. Retry Revert if the display did not recover."}</p>{error&&<p role="alert" className="settings-error">{error}</p>}<div className="settings-actions"><Button disabled={busy} onClick={()=>void decide(false)}>Revert</Button><Button variant="primary" className="primary" disabled={busy||seconds===0} onClick={()=>void decide(true)}>Keep changes</Button></div></section></div>}</>;
}
export { Wireless } from "./wireless";
