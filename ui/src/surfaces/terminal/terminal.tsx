import { Fragment, useEffect, useRef, useState } from "react";
import { Button, ConfirmDialog, Dialog, Input, Select, SurfaceButton } from "../../meridian-ui";
import { hello, on, request } from "../../lib/bridge";
import { errorMessage, mount, useWindowKey } from "../../lib/react";
import { useAppearance } from "../../lib/appearance";
import { Session } from "./session";
import { loadPreferences, localProfile, PreferencesDialog, type Profile } from "./preferences";
import type { TerminalInfo } from "../../lib/generated/TerminalInfo";

type Pane = {key:number; profile:Profile; cwd?:string; session?:number; info?:TerminalInfo; ended?:boolean};
type Tab = {key:number; name:string; panes:Pane[]; focused:number; direction:"row"|"column"; sizes:number[]; zoom:boolean};
type SavedTab = {name:string; direction:"row"|"column"; panes:{cwd:string;profile:string}[]};
function loadWorkspace(): SavedTab[] {
  try {
    const value:unknown=JSON.parse(localStorage.getItem("meridian.terminal.workspace")??"[]");
    if(!Array.isArray(value))return [];
    let count=0;
    return value.slice(0,16).filter(t=>t && typeof t.name==="string" && Array.isArray(t.panes)).map(t=>({name:t.name.slice(0,60),direction:t.direction==="column" ? "column" as const:"row" as const,panes:t.panes.slice(0,4).filter((p:{cwd?:unknown;profile?:unknown})=>p && typeof p.cwd==="string" && typeof p.profile==="string" && ++count<=16)})).filter(t=>t.panes.length);
  } catch{return [];}
}
function Terminal() {
  const appearance=useAppearance(), [preferences,setPreferences]=useState(loadPreferences);
  const [tabs,setTabs]=useState<Tab[]>([]), [active,setActive]=useState(0), [ready,setReady]=useState(false), [error,setError]=useState("");
  const [preferencesOpen,setPreferencesOpen]=useState(false), [profileId,setProfileId]=useState(preferences.defaultProfile);
  const [rename,setRename]=useState<number|null>(null), [name,setName]=useState("");
  const [confirm,setConfirm]=useState<{tab:number;pane?:number}|null>(null), [restoring,setRestoring]=useState(false);
  const [saved,setSaved]=useState<SavedTab[]>(loadWorkspace), [find,setFind]=useState({pane:0,revision:0});
  const next=useRef(1), tabsRef=useRef(tabs), activeRef=useRef(active), prefsRef=useRef(preferences), closing=useRef(false), checking=useRef(false);
  tabsRef.current=tabs;activeRef.current=active;prefsRef.current=preferences;
  const current=tabs.find(t=>t.key===active), currentPane=current?.panes.find(p=>p.key===current.focused);
  const paneCount=tabs.reduce((sum,t)=>sum+t.panes.length,0);
  function update(fn:(tabs:Tab[])=>Tab[]) {const updated=fn(tabsRef.current);tabsRef.current=updated;setTabs(updated);}
  function newTab(cwd?:string, selected?:Profile) {
    if(!readyRef.current || tabsRef.current.reduce((n,t)=>n+t.panes.length,0)>=16)return;
    closing.current=false;
    const profile=selected ?? prefsRef.current.profiles.find(p=>p.id===(selectedProfile.current || prefsRef.current.defaultProfile)) ?? localProfile;
    const pane:Pane={key:next.current++,profile,cwd}, tab:Tab={key:next.current++,name:"",panes:[pane],focused:pane.key,direction:"row",sizes:[100],zoom:false};
    const updated=[...tabsRef.current,tab];tabsRef.current=updated;setTabs(updated);setActive(tab.key);
  }
  const readyRef=useRef(false), selectedProfile=useRef(profileId); selectedProfile.current=profileId;
  useEffect(()=>{
    const opened=on("terminal.opened",()=>{closing.current=false;if(!tabsRef.current.length)newTab();});
    const openedAt=on("terminal.opened_at",event=>{closing.current=false;newTab(event.data.path,{...localProfile,shell:prefsRef.current.profiles.find(p=>p.id===prefsRef.current.defaultProfile)?.shell??""});});
    const closed=on("terminal.closed",()=>{
      if(tabsRef.current.length){const snapshot:SavedTab[]=tabsRef.current.map(t=>({name:t.name,direction:t.direction,panes:t.panes.map(p=>({cwd:p.info?.cwd||p.cwd||p.profile.cwd,profile:p.profile.id}))}));try{localStorage.setItem("meridian.terminal.workspace",JSON.stringify(snapshot));setSaved(snapshot);}catch{}}
      closing.current=true;tabsRef.current=[];setTabs([]);setActive(0);setConfirm(null);setPreferencesOpen(false);setRename(null);setRestoring(false);});
    // Queue surface events until the handshake finishes so early open-at commands survive.
    readyRef.current=true;
    void hello().then(()=>setReady(true)).catch(e=>{readyRef.current=false;setError(errorMessage(e));});
    return ()=>{readyRef.current=false;opened();openedAt();closed();};
  },[]);
  useEffect(()=>{
    if(!tabs.length || closing.current)return;
    const timer=setTimeout(()=>{
      const snapshot:SavedTab[]=tabs.map(t=>({name:t.name,direction:t.direction,panes:t.panes.map(p=>({cwd:p.info?.cwd||p.cwd||p.profile.cwd,profile:p.profile.id}))}));
      try{localStorage.setItem("meridian.terminal.workspace",JSON.stringify(snapshot));}catch{setError("Could not save the folder layout.");}
    },500);
    return ()=>clearTimeout(timer);
  },[tabs]);
  function tabTitle(tab:Tab) {
    const pane=tab.panes.find(p=>p.key===tab.focused)??tab.panes[0]!;
    return tab.name || (pane.info?.busy ? pane.info.command : (pane.info?.cwd||pane.cwd||pane.profile.cwd).split("/").filter(Boolean).pop()) || pane.profile.name;
  }
  function patchPane(tabKey:number,paneKey:number,change:Partial<Pane>) {update(old=>old.map(t=>t.key===tabKey?{...t,panes:t.panes.map(p=>p.key===paneKey?{...p,...change}:p)}:t));}
  function split(direction:"row"|"column") {
    if(!current || paneCount>=16 || current.panes.length>=4)return;
    const pane:Pane={key:next.current++,profile:currentPane?.profile??localProfile,cwd:currentPane?.info?.cwd||currentPane?.cwd};
    update(old=>old.map(t=>t.key===active?{...t,panes:[...t.panes,pane],focused:pane.key,direction,zoom:false,sizes:Array(t.panes.length+1).fill(100/(t.panes.length+1))}:t));
  }
  async function remove(tabKey:number,paneKey?:number) {
    const tab=tabsRef.current.find(t=>t.key===tabKey);if(!tab)return;
    const panes=tab.panes.filter(p=>paneKey===undefined||p.key===paneKey);
    // Stop sessions before removing their output, and surface backend errors.
    await Promise.all(panes.filter(p=>p.session!==undefined).map(p=>request({method:"terminal.close",params:{id:p.session!}})));
    update(old=>old.flatMap(t=>{
      if(t.key!==tabKey)return[t];const remaining=t.panes.filter(p=>!panes.some(removed=>removed.key===p.key));
      return remaining.length?[{...t,panes:remaining,focused:remaining.some(p=>p.key===t.focused)?t.focused:remaining[0]!.key,sizes:remaining.map(()=>100/remaining.length),zoom:false}]:[];
    }));
    if(!tabsRef.current.some(t=>t.key===activeRef.current))setActive(tabsRef.current[0]?.key??0);
  }
  async function close(tabKey:number,paneKey?:number) {
    if(checking.current)return;checking.current=true;setError("");
    try {
      const panes=tabsRef.current.find(t=>t.key===tabKey)?.panes.filter(p=>paneKey===undefined||p.key===paneKey)??[];
      const states=await Promise.all(panes.filter(p=>p.session!==undefined&&!p.ended).map(p=>request({method:"terminal.info",params:{id:p.session!}})));
      if(states.some(info=>info.busy))setConfirm({tab:tabKey,pane:paneKey});else await remove(tabKey,paneKey);
    }catch(e){setError(errorMessage(e));}finally{checking.current=false;}
  }
  function focusNext(delta:number) {
    if(!current)return;const index=current.panes.findIndex(p=>p.key===current.focused), key=current.panes[(index+delta+current.panes.length)%current.panes.length]!.key;
    update(old=>old.map(t=>t.key===active?{...t,focused:key}:t));
  }
  function resize(tabKey:number,index:number,delta:number) {
    update(old=>old.map(t=>{
      if(t.key!==tabKey)return t;const sizes=[...t.sizes],sum=sizes[index]!+sizes[index+1]!;
      sizes[index]=Math.max(10,Math.min(sum-10,sizes[index]!+delta));sizes[index+1]=sum-sizes[index]!;return{...t,sizes};
    }));
  }
  function restore() {
    const available=16-paneCount;let count=0;
    const restored=saved.flatMap(t=>{
      const panes=t.panes.filter(()=>++count<=available).map(p=>({key:next.current++,cwd:p.cwd,profile:preferences.profiles.find(profile=>profile.id===p.profile)??localProfile}));
      return panes.length?[{key:next.current++,name:t.name,panes,focused:panes[0]!.key,direction:t.direction,sizes:panes.map(()=>100/panes.length),zoom:false}]:[];
    });
    if(restored.length){closing.current=false;update(old=>[...old,...restored]);setActive(restored[0]!.key);}setRestoring(false);
  }
  useWindowKey(event=>{
    if(preferencesOpen||rename!==null||confirm||restoring||document.querySelector('[role="dialog"],[role="alertdialog"]'))return;
    if(!event.ctrlKey || event.altKey)return;
    const key=event.key.toLowerCase();
    if(event.shiftKey){
      if(key==="t"){event.preventDefault();newTab();}
      else if(key==="w"&&current){event.preventDefault();void close(current.key);}
      else if(key==="q"){event.preventDefault();void request({method:"terminal.quit"}).catch(e=>setError(errorMessage(e)));}
      else if(key==="d"||key==="e"){event.preventDefault();split(key==="d"?"row":"column");}
      else if(key==="f"&&currentPane){event.preventDefault();setFind({pane:currentPane.key,revision:find.revision+1});}
      else if(key==="enter"&&current){event.preventDefault();update(old=>old.map(t=>t.key===active?{...t,zoom:!t.zoom}:t));}
      else if(["arrowleft","arrowup","arrowright","arrowdown"].includes(key)){event.preventDefault();focusNext(["arrowleft","arrowup"].includes(key)?-1:1);}
    }else if(["PageUp","PageDown"].includes(event.key)&&tabs.length){event.preventDefault();const index=tabs.findIndex(t=>t.key===active);setActive(tabs[(index+(event.key==="PageDown"?1:tabs.length-1))%tabs.length]!.key);}
  });
  const mode=preferences.theme==="system"?appearance:preferences.theme;
  return <main className="terminal-app" data-terminal-theme={mode}>
    <header className="terminal-tabs"><div role="tablist" aria-label="Terminal tabs">{tabs.map(tab=><div className="terminal-tab" key={tab.key} draggable onDragStart={e=>e.dataTransfer.setData("application/x-meridian-terminal-tab",String(tab.key))} onDragOver={e=>{if(e.dataTransfer.types.includes("application/x-meridian-terminal-tab"))e.preventDefault();}} onDrop={e=>{const from=Number(e.dataTransfer.getData("application/x-meridian-terminal-tab"));if(!from)return;e.preventDefault();update(old=>{const source=old.find(t=>t.key===from);if(!source)return old;const reordered=old.filter(t=>t.key!==from);reordered.splice(reordered.findIndex(t=>t.key===tab.key),0,source);return reordered;});}}>
      <SurfaceButton role="tab" aria-selected={active===tab.key} tabIndex={active===tab.key?0:-1} title={tabTitle(tab)} onDoubleClick={()=>{setRename(tab.key);setName(tab.name);}} onKeyDown={e=>{if(["ArrowLeft","ArrowRight","Home","End"].includes(e.key)){e.preventDefault();const index=tabs.findIndex(t=>t.key===tab.key),n=e.key==="Home"?0:e.key==="End"?tabs.length-1:(index+(e.key==="ArrowRight"?1:tabs.length-1))%tabs.length;setActive(tabs[n]!.key);e.currentTarget.closest('[role="tablist"]')?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[n]?.focus();}}} onClick={()=>setActive(tab.key)}>{tabTitle(tab)}{tab.panes.every(p=>p.ended)?" · ended":""}</SurfaceButton><SurfaceButton aria-label={`Close ${tabTitle(tab)}`} onClick={()=>void close(tab.key)}>×</SurfaceButton>
    </div>)}</div><SurfaceButton disabled={!ready||paneCount>=16} title="New tab (Ctrl+Shift+T)" onClick={()=>newTab()}>＋</SurfaceButton><SurfaceButton onClick={()=>setPreferencesOpen(true)}>Preferences</SurfaceButton><SurfaceButton onClick={()=>void request({method:"terminal.quit"}).catch(e=>setError(errorMessage(e)))}>Quit</SurfaceButton></header>
    <nav className="terminal-actions" aria-label="Terminal actions"><Select aria-label="Profile for new tabs" value={profileId} onChange={e=>setProfileId(e.target.value)}>{preferences.profiles.map(p=><option key={p.id} value={p.id}>{p.name}</option>)}</Select>
      <Button disabled={!current||paneCount>=16||current.panes.length>=4} onClick={()=>split("row")} title="Ctrl+Shift+D">Split side by side</Button><Button disabled={!current||paneCount>=16||current.panes.length>=4} onClick={()=>split("column")} title="Ctrl+Shift+E">Split top/bottom</Button>
      <Button disabled={!currentPane} onClick={()=>currentPane&&setFind({pane:currentPane.key,revision:find.revision+1})}>Find</Button><Button disabled={!currentPane||paneCount>=16} onClick={()=>newTab(currentPane?.info?.cwd||currentPane?.cwd,currentPane?.profile)}>Duplicate folder</Button>
      <Button disabled={!current} onClick={()=>{if(current){setRename(current.key);setName(current.name);}}}>Rename tab</Button><Button disabled={!current||tabs.findIndex(t=>t.key===active)===0} onClick={()=>update(old=>{const i=old.findIndex(t=>t.key===active);if(i<1)return old;const copy=[...old];[copy[i-1],copy[i]]=[copy[i]!,copy[i-1]!];return copy;})}>Move tab left</Button>
      <Button disabled={!saved.length||paneCount>=16} onClick={()=>setRestoring(true)}>Reopen saved folders…</Button>
    </nav>
    {error&&<p role="alert" className="terminal-error">{error}<Button onClick={()=>setError("")}>Dismiss</Button></p>}
    {ready&&tabs.map(tab=><section key={tab.key} hidden={active!==tab.key} className="terminal-workspace" style={{flexDirection:tab.direction}} aria-label={`${tabTitle(tab)} panes`}>
      {tab.panes.map((pane,index)=><Fragment key={pane.key}><div className="terminal-pane" hidden={tab.zoom&&pane.key!==tab.focused} data-focused={pane.key===tab.focused} style={{flexBasis:tab.zoom?"100%":`${tab.sizes[index]}%`}}>
        <div className="terminal-pane-heading"><SurfaceButton aria-label="Focus pane" onClick={()=>update(old=>old.map(t=>t.key===tab.key?{...t,focused:pane.key}:t))}>{pane.info?.busy?pane.info.command:pane.info?.cwd||pane.cwd||pane.profile.name}{pane.ended?` · ended${pane.info?.exit_code!=null?` (${pane.info.exit_code})`:""}`:""}</SurfaceButton>
          <SurfaceButton title="Maximize/restore pane (Ctrl+Shift+Enter)" aria-label="Maximize or restore pane" onClick={()=>update(old=>old.map(t=>t.key===tab.key?{...t,focused:pane.key,zoom:!t.zoom}:t))}>{tab.zoom?"Restore":"Expand"}</SurfaceButton><SurfaceButton aria-label="Close pane" onClick={()=>void close(tab.key,pane.key)}>×</SurfaceButton></div>
        <Session active={active===tab.key&&(!tab.zoom||tab.focused===pane.key)} focused={active===tab.key&&tab.focused===pane.key} cwd={pane.cwd} profile={pane.profile} preferences={preferences} appearance={appearance} searchRequest={find.pane===pane.key?find.revision:0} onFocus={()=>{if(tab.focused!==pane.key)update(old=>old.map(t=>t.key===tab.key?{...t,focused:pane.key}:t));}} onReady={session=>patchPane(tab.key,pane.key,{session})} onState={info=>{const old=tabsRef.current.find(t=>t.key===tab.key)?.panes.find(p=>p.key===pane.key)?.info;if(JSON.stringify(old)!==JSON.stringify(info))patchPane(tab.key,pane.key,{info});}} onExit={()=>patchPane(tab.key,pane.key,{ended:true})}/>
      </div>{index<tab.panes.length-1&&!tab.zoom&&<div role="separator" aria-label="Resize panes" aria-orientation={tab.direction==="row"?"vertical":"horizontal"} aria-valuenow={Math.round(tab.sizes[index]!)} aria-valuemin={10} aria-valuemax={90} tabIndex={0} className="terminal-divider" onKeyDown={e=>{if(["ArrowLeft","ArrowUp","ArrowRight","ArrowDown"].includes(e.key)){e.preventDefault();resize(tab.key,index,["ArrowLeft","ArrowUp"].includes(e.key)?-3:3);}}} onPointerDown={e=>{e.currentTarget.setPointerCapture(e.pointerId);e.currentTarget.dataset.last=String(tab.direction==="row"?e.clientX:e.clientY);}} onPointerMove={e=>{if(!e.currentTarget.hasPointerCapture(e.pointerId))return;const coordinate=tab.direction==="row"?e.clientX:e.clientY,last=Number(e.currentTarget.dataset.last),rect=e.currentTarget.parentElement!.getBoundingClientRect();e.currentTarget.dataset.last=String(coordinate);resize(tab.key,index,(coordinate-last)/(tab.direction==="row"?rect.width:rect.height)*100);}} onPointerUp={e=>{if(e.currentTarget.hasPointerCapture(e.pointerId))e.currentTarget.releasePointerCapture(e.pointerId);}}/>}</Fragment>)}
    </section>)}
    {!tabs.length&&<div className="terminal-empty"><h1>Terminal</h1><p>Open a shell, or reopen saved folders as new sessions.</p><Button disabled={!ready} onClick={()=>newTab()}>New terminal</Button></div>}
    <footer className="terminal-hints">Ctrl+Shift+F find · Ctrl+Shift+arrows switch panes · Ctrl+click links · Drag files to insert quoted paths · 16 panes maximum</footer>
    {preferencesOpen&&<PreferencesDialog value={preferences} onClose={()=>setPreferencesOpen(false)} onSave={value=>{setPreferences(value);setProfileId(value.defaultProfile);}}/>}
    {rename!==null&&<Dialog open aria-label="Rename tab" onClose={()=>setRename(null)}><h2>Rename tab</h2><Input autoFocus aria-label="Tab name" value={name} maxLength={60} onChange={e=>setName(e.target.value)} onKeyDown={e=>{if(e.key==="Enter"){update(old=>old.map(t=>t.key===rename?{...t,name:name.trim()}:t));setRename(null);}}}/><p>Leave blank to show the current folder or running program.</p><Button onClick={()=>{update(old=>old.map(t=>t.key===rename?{...t,name:name.trim()}:t));setRename(null);}}>Save name</Button></Dialog>}
    <ConfirmDialog open={!!confirm} title="Stop running jobs?" confirmLabel="Stop and close" danger onClose={()=>setConfirm(null)} onConfirm={async()=>{if(confirm)await remove(confirm.tab,confirm.pane);}}>Closing this {confirm?.pane===undefined?"tab":"pane"} stops its shell and child processes. Unsaved work in terminal programs will be lost.</ConfirmDialog>
    <ConfirmDialog open={restoring} title="Reopen saved folders?" confirmLabel="Open new sessions" onClose={()=>setRestoring(false)} onConfirm={async()=>restore()}>This starts new shells or SSH connections using saved profiles and folders, up to the 16-pane limit. It does not recover processes, replay commands or restore previous output.</ConfirmDialog>
  </main>;
}
mount(<Terminal/>);
