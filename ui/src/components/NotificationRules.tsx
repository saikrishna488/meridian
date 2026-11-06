import { useEffect, useRef, useState } from "react";
import { Button, Select, Toggle } from "../meridian-ui";
import { on, request } from "../lib/bridge";
import { errorMessage } from "../lib/react";
import type { NotificationState } from "../lib/generated/NotificationState";
export function NotificationRules() {
  const [data,setData]=useState<NotificationState|null>(null),[error,setError]=useState(""),[busy,setBusy]=useState(false);
  const pending=useRef(false),revision=useRef(0);
  useEffect(()=>{let alive=true;const off=on("notifications.changed",e=>{revision.current++;setData(e.data);});const mine=revision.current;void request({method:"notifications.get"}).then(value=>{if(alive&&mine===revision.current)setData(value);}).catch(e=>{if(alive)setError(errorMessage(e));});return()=>{alive=false;off();};},[]);
  async function change(action:string,id:string,value:string) {
    if(pending.current)return;pending.current=true;setBusy(true);setError("");const mine=revision.current;
    try{const state=await request({method:"notifications.configure",params:{action,id,value}});if(mine===revision.current)setData(state);}
    catch(e){setError(errorMessage(e));}finally{pending.current=false;setBusy(false);}
  }
  const apps=[...new Set([...(data?.items.map(item=>item.app).filter(Boolean)??[]),...(data?.muted_apps??[])])].sort();
  return <div className="notification-rules">
    <label className="settings-row">Pause notifications<Select aria-label="Pause notifications" value="" disabled={!data||busy} onChange={e=>void change("quiet","",e.target.value)}><option value="" disabled>Choose duration</option>{[15,30,60,120].map(n=><option key={n} value={n}>{n} minutes</option>)}</Select></label>
    {data?.quiet_until && <p role="status">Paused until {new Date(data.quiet_until).toLocaleTimeString([],{hour:"2-digit",minute:"2-digit"})}. <Button disabled={busy} onClick={()=>void change("quiet","","0")}>Resume now</Button></p>}
    <details><summary>Application notification controls</summary><p>Muted applications stay in history without toasts or unread badges. Rules use the name supplied by the application.</p>
    {apps.length?apps.map(app=><div className="settings-row" key={app}><span>{app}</span><Toggle label={`Allow alerts from ${app}`} disabled={busy} checked={!data?.muted_apps.includes(app)} onChange={enabled=>void change("mute-app",app,String(!enabled))}/></div>):<p>After an app sends a notification, its controls will appear here.</p>}</details>
    {error&&<p role="alert">{error}</p>}
  </div>;
}
