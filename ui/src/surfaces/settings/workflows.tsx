import { useEffect, useRef, useState } from "react";
import { Button, Input, Select, Toggle, ConfirmDialog } from "../../meridian-ui";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import type { WorkflowState } from "../../lib/generated/WorkflowState";

function useWorkflows() {
  const [data,setData] = useState<WorkflowState | null>(null), [error,setError] = useState(""), [busy,setBusy] = useState(false);
  const pending = useRef(false), alive = useRef(true);
  async function reload() {
    if (pending.current) return;
    pending.current=true;setBusy(true);
    try { const state=await request({method:"workflow.get"});if(alive.current){setData(state);setError("");} }
    catch(e){if(alive.current)setError(errorMessage(e));}
    finally{pending.current=false;if(alive.current)setBusy(false);}
  }
  useEffect(()=>{alive.current=true;void reload();return()=>{alive.current=false;};},[]);
  async function change(action:string,id:string,value:string, propagate=false) {
    if(pending.current)return;
    pending.current=true;setBusy(true);setError("");
    try{const state=await request({method:"workflow.set",params:{action,id,value}});if(alive.current)setData(state);}
    catch(e){if(alive.current)setError(errorMessage(e));if(propagate)throw e;}
    finally{pending.current=false;if(alive.current)setBusy(false);}
  }
  return {data,error,busy,change,reload};
}
function Feedback({error,loading}:{error:string;loading:boolean}) {
  return error?<p role="alert">{error} Use Reload page to retry.</p>:loading?<p role="status">Loading desktop preferences…</p>:null;
}
export function EditableShortcuts() {
  const {data,error,busy,change}=useWorkflows();
  const [drafts,setDrafts]=useState<Record<string,string>>({});
  return <section className="settings-card"><h3>Keyboard shortcuts</h3><p>Use W for Super, C for Control, A for Alt and S for Shift, for example W-S-e. Reserved and duplicate bindings are rejected. Changes affect Meridian’s managed labwc session.</p>
    <Feedback error={error} loading={!data}/>
    {data?.shortcuts.map(shortcut=><div className="settings-row workflow-shortcut" key={shortcut.id}><label htmlFor={`shortcut-${shortcut.id}`}>{shortcut.label}</label>
      <Input id={`shortcut-${shortcut.id}`} value={drafts[shortcut.id] ?? shortcut.key} disabled={busy} maxLength={64} onChange={e=>setDrafts(old=>({...old,[shortcut.id]:e.target.value}))}/>
      <Button disabled={busy || !drafts[shortcut.id] || drafts[shortcut.id]===shortcut.key} onClick={()=>void change("shortcut",shortcut.id,drafts[shortcut.id] ?? shortcut.key)}>Save</Button>
      <Button disabled={busy || (shortcut.key===shortcut.default_key && !drafts[shortcut.id])} onClick={()=>{setDrafts(old=>{const next={...old};delete next[shortcut.id];return next;});void change("shortcut",shortcut.id,"");}}>Reset</Button>
    </div>)}
  </section>;
}
export function ComfortSettings() {
  const {data,error,busy,change}=useWorkflows();
  return <section className="settings-card"><h3>Reading &amp; motion</h3><Feedback error={error} loading={!data}/>
    <label className="settings-row"><strong>Text size</strong><Select aria-label="Text size" disabled={!data || busy} value={String(data?.text_scale ?? 1)} onChange={e=>void change("text-scale","",e.target.value)}>{[1,1.15,1.25,1.5].map(n=><option value={n} key={n}>{Math.round(n*100)}%</option>)}</Select></label>
    <div className="settings-row"><strong>High contrast controls</strong><Toggle label="High contrast controls" disabled={!data || busy} checked={data?.high_contrast ?? false} onChange={v=>void change("high-contrast","",String(v))}/></div>
    <div className="settings-row"><strong>Reduce motion</strong><Toggle label="Reduce motion" disabled={!data || busy} checked={data?.reduced_motion ?? false} onChange={v=>void change("reduced-motion","",String(v))}/></div>
    <p>Text size scales Meridian Settings, Files and Pulse. High contrast and motion preferences apply to Meridian web surfaces. Third-party apps keep their own settings.</p>
  </section>;
}
export function FirstRunChecklist({navigate}:{navigate:(page:string)=>void}) {
  const {data,error,busy,change}=useWorkflows();
  const [show,setShow]=useState(false);
  const tasks=[['displays','Displays','Check display scaling'],['keyboard','Keyboard','Review keyboard layout'],['sound','Sound','Choose audio devices'],['applications','Applications','Choose default apps']] as const;
  if(!data)return <Feedback error={error} loading={false}/>;
  if(data.completed.includes("dismissed") && !show)return <Button onClick={()=>setShow(true)}>Review desktop setup</Button>;
  return <section className="settings-card"><h3>Set up your desktop</h3><p>Review each preference, then mark it done. You can return to this checklist anytime.</p>
    {tasks.map(([id,page,label])=><div className="settings-row" key={id}><label><input type="checkbox" checked={data.completed.includes(id)} disabled={busy} onChange={e=>void change("checklist",id,String(e.target.checked))}/> {label}</label><Button onClick={()=>navigate(page)}>Open {page}</Button></div>)}
    <Button disabled={busy} onClick={()=>{setShow(false);void change("checklist","dismissed","true");}}>Hide checklist</Button><Feedback error={error} loading={false}/>
  </section>;
}
export function RecoverySettings() {
  const {data,error,busy,change,reload}=useWorkflows();
  const [target,setTarget]=useState<string|null>(null),[reset,setReset]=useState(false),[resetError,setResetError]=useState("");
  return <><section className="settings-card"><h3>Recover a desktop component</h3><p>Reload a panel, menu or Pulse without ending your session. Open application documents are preserved. A component reload clears its temporary UI state.</p>
    <Feedback error={error} loading={!data}/>{data?.surfaces.map(name=><div className="settings-row" key={name}><strong>{name}</strong><Button disabled={busy} onClick={()=>setTarget(name)}>Reload…</Button></div>)}
    <Button disabled={busy} onClick={()=>void reload()}>Refresh status</Button>
    <ConfirmDialog open={target!==null} title={`Reload ${target ?? "component"}?`} confirmLabel="Reload" onClose={()=>setTarget(null)} onConfirm={async()=>{if(target)await change("recover",target,"",true);}}>Any open menus in this component will close.</ConfirmDialog>
  </section><section className="settings-card"><h3>Appearance recovery</h3><Button onClick={()=>setReset(true)}>Restore appearance defaults…</Button>
    <ConfirmDialog open={reset} title="Restore appearance defaults?" confirmLabel="Restore" onClose={()=>setReset(false)} onConfirm={async()=>{
      setResetError("");try{await request({method:"desktop.appearance_set",params:{id:"light"}});await request({method:"desktop.dock_style_set",params:{id:"floating"}});await change("text-scale","","1",true);await change("high-contrast","","false",true);await change("reduced-motion","","false",true);}catch(e){setResetError(errorMessage(e));throw e;}
    }}>Restore Light theme, Floating dock, normal text size, contrast and motion. Wallpapers, shortcuts, files and pins are preserved. Changes save separately; a failed step may leave earlier steps applied.</ConfirmDialog>{resetError&&<p role="alert">{resetError}</p>}
  </section><section className="settings-card"><h3>Recent session errors</h3><p>Up to 30 component and request failures from this shell session. Request details stay with the originating control.</p>{data?.errors.length?<ul>{data.errors.map((text,i)=><li key={i}>{text}</li>)}</ul>:<p>No recorded errors. If a control fails, retry it and refresh this list.</p>}</section></>;
}
