import { useEffect, useRef, useState } from "react";
import { on, request } from "../../lib/bridge";
import { errorMessage, mount, usePopover, useWindowKey } from "../../lib/react";
import { Toggle } from "../../components/Toggle";
import { SystemSlider } from "../../components/SystemSlider";
import { useDesktop } from "../../lib/desktop";
import type { SettingsState } from "../../lib/generated/SettingsState";
function Options() {
 const [settings,setSettings]=useState<SettingsState>({wifi:null,bluetooth:null,brightness:null,volume:null}),[status,setStatus]=useState(""),[busy,setBusy]=useState(false);
 const revision=useRef(0); const {desktop,setDesktop}=useDesktop();
 const report=(e:unknown)=>setStatus(errorMessage(e));
 const {visible,menu,dismiss}=usePopover(async()=>{setStatus("");const mine=revision.current;const initial=await request({method:"settings.get"});if(mine===revision.current&&initial)setSettings(initial);},()=>menu.current?.focus(),report);
 useEffect(()=>on("settings.changed",event=>{revision.current++;setSettings(event.data);}),[]);
 useWindowKey(event=>{if(event.key==="Escape"){event.preventDefault();void dismiss().catch(report);}});
 async function toggle(key:"wifi"|"bluetooth",enabled:boolean){const previous=settings[key];if(previous===null)return;const mine=revision.current;setSettings(s=>({...s,[key]:enabled}));setStatus("");try{await request({method:key==="wifi"?"settings.set_wifi":"settings.set_bluetooth",params:{enabled}});}catch(e){if(mine===revision.current)setSettings(s=>({...s,[key]:previous}));report(e);}}
 async function appearance(){if(busy)return;setBusy(true);try{setDesktop(await request({method:"desktop.appearance_set",params:{id:desktop?.appearance==="dark"?"light":"dark"}}));}catch(e){report(e);}finally{setBusy(false);}}
 return <><div className="scrim" id="scrim" onClick={()=>void dismiss().catch(report)}/><section ref={menu} className={`menu control-center${visible?"":" hidden"}`} id="menu" role="dialog" aria-label="Control Center" tabIndex={-1}>
 <header className="control-heading"><h1>Control Center</h1><button aria-label="Open Settings" onClick={()=>void request({method:"preferences.open"}).then(dismiss).catch(report)}>Settings…</button></header>
 <div className="control-grid"><section className="control-card connectivity" aria-label="Connectivity">{(["wifi","bluetooth"] as const).map(key=><div className="control-connection" key={key}><span className={`control-symbol${settings[key]?" enabled":""}`} aria-hidden="true">{key==="wifi"?"⌁":"ᛒ"}</span><div><strong id={`${key}-label`}>{key==="wifi"?"Wi-Fi":"Bluetooth"}</strong><small>{settings[key]===null?"Unavailable":settings[key]?"On":"Off"}</small></div><Toggle id={key} labelledBy={`${key}-label`} checked={settings[key]===true} disabled={settings[key]===null} onChange={enabled=>void toggle(key,enabled)}/></div>)}</section>
 <button className="control-card appearance-tile" disabled={busy} onClick={()=>void appearance()} aria-pressed={desktop?.appearance==="dark"}><span className="control-symbol" aria-hidden="true">◐</span><strong>Appearance</strong><small>{desktop?.appearance==="dark"?"Dark":"Light"}</small></button>
 <section className="control-card control-wide" aria-label="Display brightness"><div className="control-card-title"><span aria-hidden="true">☼</span><strong>Display</strong></div><SystemSlider name="brightness" label="Brightness" value={settings.brightness} report={report}/></section>
 <section className="control-card control-wide" aria-label="Sound volume"><div className="control-card-title"><span aria-hidden="true">♪</span><strong>Sound</strong></div><SystemSlider name="volume" label="Volume" value={settings.volume} report={report}/></section>
 </div><p className="status" id="status" role="status" aria-live="polite">{status}</p></section></>;
}
mount(<Options/>);
