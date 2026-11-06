import { IconTile, HoverTip } from "../../meridian-ui";
import { SurfaceButton } from "../../meridian-ui";
import { Button } from "../../meridian-ui";
import { useEffect, useRef, useState } from "react";
import { on, request } from "../../lib/bridge";
import { errorMessage, mount, useDialogFocus, usePopover, useWindowKey } from "../../lib/react";
import { Icon } from "../../components/Icon";
import { Toggle } from "../../meridian-ui";
import { SystemSlider } from "../../components/SystemSlider";
import { ConfirmDialog } from "../../meridian-ui";
import { BatteryIcon } from "../../components/BatteryIcon";
import { usePowerStatus } from "../../lib/power";
import type { SettingsState } from "../../lib/generated/SettingsState";
import type { NotificationState } from "../../lib/generated/NotificationState";
import { AudioMixer } from "../../components/AudioMixer";
function Options() {
 const [notifications,setNotifications]=useState<NotificationState | null>(null),[dndBusy,setDndBusy]=useState(false);
 const notificationRevision=useRef(0), dndPending=useRef(false);
 const [settings,setSettings]=useState<SettingsState>({wifi:null,bluetooth:null,brightness:null,volume:null}),[status,setStatus]=useState("");
 const [sessionBusy,setSessionBusy]=useState(false);
 const sessionPending=useRef(false);
 const connectionPending = useRef({wifi:false, bluetooth:false});
 const [connectionBusy, setConnectionBusy] = useState({wifi:false, bluetooth:false});
 const [confirmation, setConfirmation] = useState<"session.restart" | "session.sign_out" | "session.shutdown" | null>(null);

 const power=usePowerStatus();
 const battery=power?.batteries[0];
 const revision=useRef(0);
 const report=(e:unknown)=>setStatus(errorMessage(e));
 const {visible,menu,dismiss}=usePopover(async()=>{setStatus("");const notificationMine=notificationRevision.current;
 void request({method:"notifications.get"}).then(result=>{if(notificationMine===notificationRevision.current&&result)setNotifications(result);}).catch(report);
 const mine=revision.current;const initial=await request({method:"settings.get"});if(mine===revision.current&&initial)setSettings(initial);},()=>menu.current?.focus(),report);
 useEffect(() => { if (!visible) setConfirmation(null); }, [visible]);
 useEffect(()=>on("settings.changed",event=>{revision.current++;setSettings(event.data);}),[]);
 useEffect(()=>on("notifications.changed",event=>{notificationRevision.current++;setNotifications(event.data);}),[]);
 useWindowKey(event=>{if(event.key==="Escape" && !confirmation){event.preventDefault();void dismiss().catch(report);}});
 useDialogFocus(menu, visible, () => { void dismiss().catch(report); }, false);
 async function toggle(key:"wifi"|"bluetooth",enabled:boolean) {
  if (connectionPending.current[key] || settings[key] === null) return;
  connectionPending.current[key] = true;
  setConnectionBusy(value => ({...value, [key]:true})); setStatus("");
  try {
   await request({method:key === "wifi" ? "settings.set_wifi" : "settings.set_bluetooth", params:{enabled}});
   const mine = revision.current;
   const actual = await request({method:"settings.get"});
   if (mine === revision.current && actual) {
    setSettings(actual);
    if (actual[key] !== enabled) setStatus("The requested connection state has not been applied. Check the device and try again.");
   }
  } catch(error) { report(error); }
  finally { connectionPending.current[key] = false; setConnectionBusy(value => ({...value,[key]:false})); }
 }
 async function sessionAction(method:"session.lock"|"session.sleep"|"session.restart"|"session.shutdown"|"session.sign_out") {
  if(sessionPending.current)return;
  sessionPending.current=true;setSessionBusy(true);setStatus("");
  try { await request({method});await dismiss(); }
  catch(error){report(error);}
  finally {sessionPending.current=false;setSessionBusy(false);}
 }
 async function doNotDisturb(enabled:boolean) {
  if(dndPending.current)return;
  dndPending.current=true;setDndBusy(true);setStatus("");const mine=notificationRevision.current;
  try { const result=await request({method:"notifications.do_not_disturb",params:{enabled}});if(mine===notificationRevision.current)setNotifications(result); }
  catch(e){report(e);}finally{dndPending.current=false;setDndBusy(false);}
 }
 return <><div hidden={!visible} className="scrim" id="scrim" onClick={()=>void dismiss().catch(report)}/><section ref={menu} inert={!visible} aria-hidden={!visible} className={`menu control-center${visible?"":" hidden"}`} id="menu" role="dialog" aria-label="Control Center" tabIndex={-1}>
 <header className="control-heading"><h1>Control Center</h1><HoverTip label="Open Settings"><Button className="settings-shortcut" aria-label="Open Settings" onClick={()=>void request({method:"preferences.open"}).then(dismiss).catch(report)}><Icon name="settings" width="17" height="17" /></Button></HoverTip></header>
 <div className="control-grid">{(["wifi","bluetooth"] as const).map(key=><section className="control-card control-connection" data-enabled={settings[key] === true} key={key}><span className={`control-symbol${settings[key]?" enabled":""}`} aria-hidden="true"><IconTile icon={key==="wifi"?"wifi":"bluetooth"} tone="blue" /></span><div><strong id={`${key}-label`}>{key==="wifi"?"Wi-Fi":"Bluetooth"}</strong><small>{connectionBusy[key] ? "Updating…" : settings[key]===null?"Unavailable":settings[key]?"On":"Off"}</small></div><Toggle id={key} labelledBy={`${key}-label`} checked={settings[key]===true} disabled={connectionBusy[key] || settings[key]===null} onChange={enabled=>void toggle(key,enabled)}/></section>)}
 <section className="control-card control-dnd" data-enabled={notifications?.do_not_disturb === true}><span className={`control-symbol${notifications?.do_not_disturb?" enabled":""}`} aria-hidden="true"><IconTile icon="moon" tone="orange" /></span><div><strong>Do Not Disturb</strong><small>{dndBusy ? "Updating…" : notifications ? notifications.do_not_disturb ? "On · Badges hidden" : "Off" : "Unavailable"}</small></div><Toggle label="Do Not Disturb" checked={notifications?.do_not_disturb===true} disabled={dndBusy||notifications===null} onChange={enabled=>void doNotDisturb(enabled)}/></section>
 <SurfaceButton className="control-card control-status-tile" aria-label={battery ? `Battery ${battery.percent===null?"level unavailable":`${battery.percent}%`}, ${battery.status}. Open Power settings` : "Battery unavailable"} disabled={!battery} onClick={()=>void request({method:"preferences.page_open",params:{page:"power"}}).then(dismiss).catch(report)}>
  <span className={`control-symbol${power?.on_ac?" enabled":""}`} aria-hidden="true">{battery?<BatteryIcon percent={battery.percent} charging={battery.status==="Charging"}/>:<Icon name="power" width="20" height="20"/>}</span>
  <span><strong>{battery&&battery.percent!==null?`${battery.percent}%`:"Battery"}</strong><small>{battery?battery.status:"Unavailable"}</small></span>
 </SurfaceButton>
 <section className="control-card control-wide" aria-label="Display brightness"><div className="control-card-title"><IconTile icon="monitor" tone="blue" /><strong>Display</strong></div><SystemSlider name="brightness" label="Brightness" value={settings.brightness} report={report}/></section>
 <section className="control-card control-wide" aria-label="Sound volume"><div className="control-card-title"><IconTile icon="sound" tone="red" /><strong>Sound</strong><Button className="sound-settings-link" aria-label="Open Sound settings" onClick={()=>void request({method:"preferences.page_open",params:{page:"sound"}}).then(dismiss).catch(report)}><Icon name="forward" width="14" height="14" /></Button></div><SystemSlider name="volume" label="Volume" value={settings.volume} report={report}/>{visible && <AudioMixer />}</section>
 </div>
 <section className="control-session" aria-labelledby="session-heading" key={visible ? "open" : "closed"}>
  <h2 id="session-heading">Session &amp; power</h2>
  <div className="control-session-actions">
   <SurfaceButton disabled={sessionBusy} onClick={()=>void sessionAction("session.lock")}>Lock</SurfaceButton>
   <SurfaceButton disabled={sessionBusy} onClick={()=>void sessionAction("session.sleep")}>Sleep</SurfaceButton>
   <SurfaceButton disabled={sessionBusy} onClick={() => setConfirmation("session.restart")}>Restart</SurfaceButton>
   <SurfaceButton disabled={sessionBusy} onClick={() => setConfirmation("session.sign_out")}>Sign out</SurfaceButton>
   <SurfaceButton disabled={sessionBusy} className="session-shutdown" onClick={() => setConfirmation("session.shutdown")}>Shut Down</SurfaceButton>
  </div>
  <p className="session-hint">Restart, Sign out and Shut Down ask for confirmation.</p>
 </section><ConfirmDialog open={confirmation !== null} title={confirmation === "session.restart" ? "Restart Meridian?" : confirmation === "session.sign_out" ? "Sign out?" : "Shut down?"} confirmLabel="Continue" danger onClose={() => setConfirmation(null)} onConfirm={async () => { if (confirmation) { await request({method:confirmation}); await dismiss(); } }}>Save your work before continuing.</ConfirmDialog><p className="status" id="status" role="status" aria-live="polite">{status}</p></section></>;
}
mount(<Options/>);
