import { useEffect, useRef, useState } from "react";
import { on, request } from "../../lib/bridge";
import { errorMessage, mount, useDialogFocus, usePopover, useWindowKey } from "../../lib/react";
import { Icon } from "../../components/Icon";
import { ConfirmButton } from "../../components/ConfirmButton";
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
 useDialogFocus(menu, visible, () => { void dismiss().catch(report); });
 async function toggle(key:"wifi"|"bluetooth",enabled:boolean){const previous=settings[key];if(previous===null)return;const mine=revision.current;setSettings(s=>({...s,[key]:enabled}));setStatus("");try{await request({method:key==="wifi"?"settings.set_wifi":"settings.set_bluetooth",params:{enabled}});}catch(e){if(mine===revision.current)setSettings(s=>({...s,[key]:previous}));report(e);}}
 async function sessionAction(method: "session.lock" | "session.sleep" | "session.restart" | "session.shutdown" | "session.sign_out") {
  if (busy) return;
  setBusy(true); setStatus("");
  try { await request({method}); await dismiss(); }
  finally { setBusy(false); }
 }
 async function appearance(){if(busy)return;setBusy(true);try{setDesktop(await request({method:"desktop.appearance_set",params:{id:desktop?.appearance==="dark"?"light":"dark"}}));}catch(e){report(e);}finally{setBusy(false);}}
 return <><div className="scrim" id="scrim" onClick={()=>void dismiss().catch(report)}/><section ref={menu} className={`menu control-center${visible?"":" hidden"}`} id="menu" role="dialog" aria-label="Control Center" tabIndex={-1}>
 <header className="control-heading"><h1>Control Center</h1><button aria-label="Open Settings" onClick={()=>void request({method:"preferences.open"}).then(dismiss).catch(report)}><Icon name="settings" width="17" height="17" /><span>Settings</span></button></header>
 <div className="control-grid">{(["wifi","bluetooth"] as const).map(key=><section className="control-card control-connection" key={key}><span className={`control-symbol${settings[key]?" enabled":""}`} aria-hidden="true"><Icon name={key==="wifi"?"wifi":"bluetooth"} width="20" height="20" /></span><div><strong id={`${key}-label`}>{key==="wifi"?"Wi-Fi":"Bluetooth"}</strong><small>{settings[key]===null?"Unavailable":settings[key]?"On":"Off"}</small></div><Toggle id={key} labelledBy={`${key}-label`} checked={settings[key]===true} disabled={settings[key]===null} onChange={enabled=>void toggle(key,enabled)}/></section>)}
 <button className="control-card appearance-tile" disabled={busy} onClick={()=>void appearance()} aria-pressed={desktop?.appearance==="dark"}><span className="control-symbol" aria-hidden="true"><Icon name="appearance" width="24" height="24" /></span><strong>Appearance</strong><small>{desktop?.appearance==="dark"?"Dark":"Light"}</small></button>
 <section className="control-card control-wide" aria-label="Display brightness"><div className="control-card-title"><span aria-hidden="true"><Icon name="monitor" width="18" height="18" /></span><strong>Display</strong></div><SystemSlider name="brightness" label="Brightness" value={settings.brightness} report={report}/></section>
 <section className="control-card control-wide" aria-label="Sound volume"><div className="control-card-title"><span aria-hidden="true"><Icon name="sound" width="18" height="18" /></span><strong>Sound</strong></div><SystemSlider name="volume" label="Volume" value={settings.volume} report={report}/></section>
 </div><footer className="control-footer"><button onClick={()=>void request({method:"preferences.wallpaper_open"}).then(dismiss).catch(report)}><Icon name="image" width="16" height="16" />Personalize</button><button id="lock" disabled={busy} onClick={()=>void sessionAction("session.lock").catch(report)}><Icon name="lock" width="16" height="16" />Lock screen<kbd>Super L</kbd></button><div className="power-actions" role="group" aria-label="Power options"><button id="sleep" disabled={busy} onClick={()=>void sessionAction("session.sleep").catch(report)}>Sleep</button><ConfirmButton id="restart" className="power-button" disabled={busy} label="Restart" report={report} action={()=>sessionAction("session.restart")}/><ConfirmButton id="shutdown" className="power-button" disabled={busy} label="Shut Down" report={report} action={()=>sessionAction("session.shutdown")}/><ConfirmButton id="sign-out" className="power-button" disabled={busy} label="Sign out" report={report} action={()=>sessionAction("session.sign_out")}/></div></footer><p className="status" id="status" role="status" aria-live="polite">{status}</p></section></>;
}
mount(<Options/>);
