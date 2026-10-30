import { Button } from "../../components/ui/button";
import { useEffect, useRef, useState } from "react";
import { on, request } from "../../lib/bridge";
import { errorMessage, mount, useDialogFocus, usePopover, useWindowKey } from "../../lib/react";
import { Icon } from "../../components/Icon";
import { Toggle } from "../../components/Toggle";
import { SystemSlider } from "../../components/SystemSlider";
import { BatteryIcon } from "../../components/BatteryIcon";
import { usePowerStatus } from "../../lib/power";
import type { SettingsState } from "../../lib/generated/SettingsState";
import type { NotificationState } from "../../lib/generated/NotificationState";
function Options() {
 const [notifications,setNotifications]=useState<NotificationState | null>(null),[dndBusy,setDndBusy]=useState(false);
 const notificationRevision=useRef(0), dndPending=useRef(false);
 const [settings,setSettings]=useState<SettingsState>({wifi:null,bluetooth:null,brightness:null,volume:null}),[status,setStatus]=useState("");
 const power=usePowerStatus();
 const battery=power?.batteries[0];
 const revision=useRef(0);
 const report=(e:unknown)=>setStatus(errorMessage(e));
 const {visible,menu,dismiss}=usePopover(async()=>{setStatus("");const notificationMine=notificationRevision.current;
 void request({method:"notifications.get"}).then(result=>{if(notificationMine===notificationRevision.current&&result)setNotifications(result);}).catch(report);
 const mine=revision.current;const initial=await request({method:"settings.get"});if(mine===revision.current&&initial)setSettings(initial);},()=>menu.current?.focus(),report);
 useEffect(()=>on("settings.changed",event=>{revision.current++;setSettings(event.data);}),[]);
 useEffect(()=>on("notifications.changed",event=>{notificationRevision.current++;setNotifications(event.data);}),[]);
 useWindowKey(event=>{if(event.key==="Escape"){event.preventDefault();void dismiss().catch(report);}});
 useDialogFocus(menu, visible, () => { void dismiss().catch(report); }, false);
 async function toggle(key:"wifi"|"bluetooth",enabled:boolean){const previous=settings[key];if(previous===null)return;const mine=revision.current;setSettings(s=>({...s,[key]:enabled}));setStatus("");try{await request({method:key==="wifi"?"settings.set_wifi":"settings.set_bluetooth",params:{enabled}});}catch(e){if(mine===revision.current)setSettings(s=>({...s,[key]:previous}));report(e);}}
 async function doNotDisturb(enabled:boolean) {
  if(dndPending.current)return;
  dndPending.current=true;setDndBusy(true);setStatus("");const mine=notificationRevision.current;
  try { const result=await request({method:"notifications.do_not_disturb",params:{enabled}});if(mine===notificationRevision.current)setNotifications(result); }
  catch(e){report(e);}finally{dndPending.current=false;setDndBusy(false);}
 }
 return <><div hidden={!visible} className="scrim" id="scrim" onClick={()=>void dismiss().catch(report)}/><section ref={menu} inert={!visible} aria-hidden={!visible} className={`menu control-center${visible?"":" hidden"}`} id="menu" role="dialog" aria-label="Control Center" tabIndex={-1}>
 <header className="control-heading"><h1>Control Center</h1><Button className="settings-shortcut" aria-label="Open Settings" title="Open Settings" onClick={()=>void request({method:"preferences.open"}).then(dismiss).catch(report)}><Icon name="settings" width="17" height="17" /></Button></header>
 <div className="control-grid">{(["wifi","bluetooth"] as const).map(key=><section className="control-card control-connection" key={key}><span className={`control-symbol${settings[key]?" enabled":""}`} aria-hidden="true"><Icon name={key==="wifi"?"wifi":"bluetooth"} width="20" height="20" /></span><div><strong className="visually-hidden" id={`${key}-label`}>{key==="wifi"?"Wi-Fi":"Bluetooth"}</strong><small className="visually-hidden">{settings[key]===null?"Unavailable":settings[key]?"On":"Off"}</small></div><Toggle id={key} labelledBy={`${key}-label`} checked={settings[key]===true} disabled={settings[key]===null} onChange={enabled=>void toggle(key,enabled)}/></section>)}
 <section className="control-card control-dnd"><span className={`control-symbol${notifications?.do_not_disturb?" enabled":""}`} aria-hidden="true"><Icon name="moon" width="20" height="20" /></span><div><strong className="visually-hidden">Do Not Disturb</strong><small className="visually-hidden">{notifications ? notifications.do_not_disturb ? "On · Badges hidden" : "Off" : "Unavailable"}</small></div><Toggle label="Do Not Disturb" checked={notifications?.do_not_disturb===true} disabled={dndBusy||notifications===null} onChange={enabled=>void doNotDisturb(enabled)}/></section>
 <button className="control-card control-status-tile" aria-label={battery ? `Battery ${battery.percent===null?"level unavailable":`${battery.percent}%`}, ${battery.status}. Open Power settings` : "Battery unavailable"} disabled={!battery} onClick={()=>void request({method:"preferences.page_open",params:{page:"power"}}).then(dismiss).catch(report)}>
  <span className={`control-symbol${power?.on_ac?" enabled":""}`} aria-hidden="true">{battery?<BatteryIcon percent={battery.percent} charging={battery.status==="Charging"}/>:<Icon name="power" width="20" height="20"/>}</span>
  <span><strong>{battery&&battery.percent!==null?`${battery.percent}%`:"Battery"}</strong><small>{battery?battery.status:"Unavailable"}</small></span>
 </button>
 <section className="control-card control-wide" aria-label="Display brightness"><div className="control-card-title"><span aria-hidden="true"><Icon name="monitor" width="18" height="18" /></span><strong>Display</strong></div><SystemSlider name="brightness" label="Brightness" value={settings.brightness} report={report}/></section>
 <section className="control-card control-wide" aria-label="Sound volume"><div className="control-card-title"><span aria-hidden="true"><Icon name="sound" width="18" height="18" /></span><strong>Sound</strong><Button className="sound-settings-link" aria-label="Open Sound settings" onClick={()=>void request({method:"preferences.page_open",params:{page:"sound"}}).then(dismiss).catch(report)}><Icon name="forward" width="14" height="14" /></Button></div><SystemSlider name="volume" label="Volume" value={settings.volume} report={report}/></section>
 </div><p className="status" id="status" role="status" aria-live="polite">{status}</p></section></>;
}
mount(<Options/>);
