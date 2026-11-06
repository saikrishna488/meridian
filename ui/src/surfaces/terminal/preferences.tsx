import { useState } from "react";
import { Button, Dialog, Input, Select, Switch, SettingRow } from "../../meridian-ui";
export type Profile = { id:string; name:string; shell:string; cwd:string; host:string; port:number; integration:boolean };
export type Preferences = { font:string; size:number; cursor:"bar"|"block"|"underline"; scrollback:number; theme:"system"|"light"|"dark"; notifications:boolean; profiles:Profile[]; defaultProfile:string };
export const localProfile: Profile = {id:"local",name:"Local shell",shell:"",cwd:"",host:"",port:22,integration:false};
const defaults: Preferences = {font:"DejaVu Sans Mono",size:13,cursor:"bar",scrollback:5000,theme:"system",notifications:false,profiles:[localProfile],defaultProfile:"local"};
export function loadPreferences(): Preferences {
  try {
    const value = JSON.parse(localStorage.getItem("meridian.terminal.preferences") ?? "null");
    if (!value || !Array.isArray(value.profiles)) return defaults;
    const profiles: Profile[] = value.profiles.slice(0,16).filter((p: Profile) => p && typeof p.id === "string" && typeof p.name === "string").map((p: Profile) => ({...localProfile,...p,shell:typeof p.shell === "string" ? p.shell : "",cwd:typeof p.cwd === "string" ? p.cwd : "",host:typeof p.host === "string" ? p.host : "",port:Number.isInteger(p.port) && p.port>0 && p.port<65536 ? p.port : 22,integration:p.integration === true}));
    if (!profiles.length) profiles.push(localProfile);
    return {...defaults,profiles,defaultProfile:profiles.some(p => p.id===value.defaultProfile) ? value.defaultProfile : profiles[0]!.id,
      font:typeof value.font==="string" && value.font.length<100 ? value.font : defaults.font,
      size:Number.isInteger(value.size) && value.size>=9 && value.size<=32 ? value.size : 13,
      scrollback:[1000,5000,10000,20000].includes(value.scrollback) ? value.scrollback : 5000,
      cursor:["bar","block","underline"].includes(value.cursor) ? value.cursor : "bar",
      theme:["system","light","dark"].includes(value.theme) ? value.theme : "system",notifications:value.notifications===true};
  } catch { return defaults; }
}
export function PreferencesDialog({value,onSave,onClose}:{value:Preferences;onSave:(value:Preferences)=>void;onClose:()=>void}) {
  const [draft,setDraft] = useState(value), [selected,setSelected] = useState(value.defaultProfile), [error,setError] = useState("");
  const profile = draft.profiles.find(p=>p.id===selected) ?? draft.profiles[0]!;
  const update = (change:Partial<Profile>) => setDraft({...draft,profiles:draft.profiles.map(p=>p.id===profile.id ? {...p,...change}:p)});
  return <Dialog open onClose={onClose} aria-label="Terminal preferences"><h2>Terminal preferences</h2>
    <SettingRow title="Font"><Input aria-label="Monospace font" value={draft.font} maxLength={100} onChange={e=>setDraft({...draft,font:e.target.value})} /></SettingRow>
    <SettingRow title="Text size"><Input aria-label="Text size" type="number" min={9} max={32} value={draft.size} onChange={e=>setDraft({...draft,size:Number(e.target.value)})} /></SettingRow>
    <SettingRow title="Cursor"><Select aria-label="Cursor style" value={draft.cursor} onChange={e=>setDraft({...draft,cursor:e.target.value as Preferences["cursor"]})}>{["bar","block","underline"].map(x=><option key={x}>{x}</option>)}</Select></SettingRow>
    <SettingRow title="Scrollback"><Select aria-label="Scrollback lines" value={draft.scrollback} onChange={e=>setDraft({...draft,scrollback:Number(e.target.value)})}>{[1000,5000,10000,20000].map(n=><option key={n} value={n}>{n.toLocaleString()} lines per pane</option>)}</Select></SettingRow>
    <SettingRow title="Appearance"><Select aria-label="Terminal appearance" value={draft.theme} onChange={e=>setDraft({...draft,theme:e.target.value as Preferences["theme"]})}><option value="system">Follow desktop</option><option value="light">Light</option><option value="dark">Dark</option></Select></SettingRow>
    <SettingRow title="Command notifications" description="Notify after integrated commands lasting at least 10 seconds finish while their pane is unfocused."><Switch label="Command notifications" checked={draft.notifications} onChange={notifications=>setDraft({...draft,notifications})} /></SettingRow>
    <h3>Profiles</h3><p>Profiles save folders, shells and SSH destinations, never passwords. Shell/profile changes apply to new panes.</p>
    <div className="terminal-preference-actions"><Select aria-label="Edit profile" value={profile.id} onChange={e=>setSelected(e.target.value)}>{draft.profiles.map(p=><option key={p.id} value={p.id}>{p.name}</option>)}</Select><Button disabled={draft.profiles.length>=16} onClick={()=>{const p={...localProfile,id:crypto.randomUUID(),name:"New profile"};setDraft({...draft,profiles:[...draft.profiles,p]});setSelected(p.id);}}>Add profile</Button><Button disabled={draft.profiles.length===1} onClick={()=>{const profiles=draft.profiles.filter(p=>p.id!==profile.id);setDraft({...draft,profiles,defaultProfile:draft.defaultProfile===profile.id ? profiles[0]!.id:draft.defaultProfile});setSelected(profiles[0]!.id);}}>Remove</Button></div>
    <SettingRow title="Profile name"><Input aria-label="Profile name" value={profile.name} maxLength={60} onChange={e=>update({name:e.target.value})}/></SettingRow>
    <SettingRow title="Shell" description="Absolute executable path; blank uses your account shell."><Input aria-label="Shell path" placeholder="/bin/bash" value={profile.shell} onChange={e=>update({shell:e.target.value})}/></SettingRow>
    <SettingRow title="Starting folder"><Input aria-label="Starting folder" placeholder="Account home directory" value={profile.cwd} onChange={e=>update({cwd:e.target.value})}/></SettingRow>
    <SettingRow title="SSH destination" description="Optional host or user@host. Uses your existing SSH config and keys."><Input aria-label="SSH destination" value={profile.host} onChange={e=>update({host:e.target.value})}/></SettingRow>
    <SettingRow title="SSH port"><Input aria-label="SSH port" type="number" min={1} max={65535} value={profile.port} onChange={e=>update({port:Number(e.target.value)})}/></SettingRow>
    <SettingRow title="Shell integration" description="Local Bash only: show command duration and exit status. Other shells keep standard terminal behavior."><Switch label="Enable Bash integration" checked={profile.integration} onChange={integration=>update({integration})}/></SettingRow>
    <SettingRow title="Default profile"><Select aria-label="Default profile" value={draft.defaultProfile} onChange={e=>setDraft({...draft,defaultProfile:e.target.value})}>{draft.profiles.map(p=><option key={p.id} value={p.id}>{p.name}</option>)}</Select></SettingRow>
    {error && <p role="alert">{error}</p>}<div className="muk-dialog-actions"><Button onClick={onClose}>Cancel</Button><Button variant="primary" onClick={()=>{
      if (!draft.font.trim() || !Number.isInteger(draft.size) || draft.size<9 || draft.size>32 || draft.profiles.some(p=>!p.name.trim() || !Number.isInteger(p.port) || p.port<1 || p.port>65535 || p.shell && !p.shell.startsWith("/"))) {setError("Check the font, text size, profile names, shell paths and ports.");return;}
      try {localStorage.setItem("meridian.terminal.preferences",JSON.stringify(draft));onSave(draft);onClose();} catch {setError("Preferences could not be saved.");}
    }}>Save preferences</Button></div>
  </Dialog>;
}
