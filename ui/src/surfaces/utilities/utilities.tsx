import { Tabs } from "../../components/ui/tabs";
import { Button } from "../../components/ui/button";
import { useEffect, useRef, useState } from "react";
import { hello, request } from "../../lib/bridge";
import { errorMessage, mount } from "../../lib/react";
import type { SurfaceKind } from "../../lib/generated/SurfaceKind";
type Appointment = { id: string; date: string; title: string; time: string };
const key = (d: Date) => `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,"0")}-${String(d.getDate()).padStart(2,"0")}`;
function Calendar() {
  const [month, setMonth] = useState(new Date()), [selected, setSelected] = useState(key(new Date())), [events, setEvents] = useState<Appointment[]>([]), [title, setTitle] = useState(""), [time, setTime] = useState("09:00"), [error, setError] = useState("");
  useEffect(() => { void request({method:"calendar.list"}).then(setEvents).catch(e => setError(errorMessage(e))); }, []);
  const saving = useRef(false), days = useRef<HTMLDivElement>(null), focusDay = useRef<string | null>(null);
  useEffect(() => {
    if (focusDay.current) {
      days.current?.querySelector<HTMLButtonElement>(`[data-date="${focusDay.current}"]`)?.focus();
      focusDay.current = null;
    }
  }, [selected, month]);
  function navigateDay(event: React.KeyboardEvent<HTMLButtonElement>, date: Date) {
    const offset = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7, Home: -date.getDay(), End: 6 - date.getDay() }[event.key];
    if (offset === undefined) return;
    event.preventDefault();
    const next = new Date(date); next.setDate(next.getDate() + offset);
    focusDay.current = key(next); setSelected(key(next));
    if (next.getMonth() !== month.getMonth() || next.getFullYear() !== month.getFullYear()) setMonth(new Date(next.getFullYear(), next.getMonth(), 1));
  }
  const today = key(new Date());
  const eventCounts = new Map<string, number>();
  for (const event of events) eventCounts.set(event.date, (eventCounts.get(event.date) ?? 0) + 1);
  async function save(next: Appointment[]) { if (saving.current) return; saving.current = true; try { await request({method:"calendar.save", params:next}); setEvents(next); setError(""); } catch(e) { setError(errorMessage(e)); } finally { saving.current = false; } }
  const first = new Date(month.getFullYear(),month.getMonth(),1), start = new Date(first); start.setDate(1-first.getDay());
  return <><header><span className="app-mark">▦</span><div><h1>Calendar</h1></div><Button onClick={() => { setMonth(new Date()); setSelected(key(new Date())); }}>Today</Button></header><div className="calendar-layout"><section><div className="toolbar"><Button aria-label="Previous month" onClick={() => setMonth(new Date(month.getFullYear(),month.getMonth()-1,1))}>‹</Button><h2 aria-live="polite">{month.toLocaleDateString(undefined,{month:"long",year:"numeric"})}</h2><Button aria-label="Next month" onClick={() => setMonth(new Date(month.getFullYear(),month.getMonth()+1,1))}>›</Button></div><div ref={days} className="month" role="group" aria-label="Month dates. Use arrow keys to navigate.">{["Sun","Mon","Tue","Wed","Thu","Fri","Sat"].map(day => <small key={day}>{day}</small>)}{Array.from({length:42},(_,i) => { const d = new Date(start); d.setDate(start.getDate()+i); const id = key(d); return <Button key={id} data-date={id} tabIndex={selected===id || (selected < key(start) || selected > key(new Date(start.getFullYear(), start.getMonth(), start.getDate()+41))) && id===key(first) ? 0 : -1} aria-current={id===today ? "date" : undefined} onKeyDown={e => navigateDay(e, d)} className={`${d.getMonth()!==month.getMonth()?"outside":""} ${id===today?"today":""}`} aria-pressed={selected===id} aria-label={`${d.toDateString()}${eventCounts.has(id) ? `, ${eventCounts.get(id)} events` : ""}`} onClick={() => setSelected(id)}>{d.getDate()}<span>{eventCounts.has(id)?"•":" "}</span></Button>; })}</div></section><aside><h2>{new Date(selected+"T12:00:00").toLocaleDateString(undefined,{month:"long",day:"numeric"})}</h2>{error && <p role="alert">{error}</p>}{events.filter(e => e.date===selected).sort((a,b) => a.time.localeCompare(b.time)).map(e => <article className="event" key={e.id}><time>{e.time}</time><strong>{e.title}</strong><Button aria-label={`Delete ${e.title}`} onClick={() => save(events.filter(v => v.id!==e.id))}>×</Button></article>)}{!events.some(e => e.date===selected) && <p className="muted">No events</p>}<form onSubmit={e => { e.preventDefault(); if(title.trim()) { save([...events,{id:crypto.randomUUID(),date:selected,title:title.trim(),time}]); setTitle(""); } }}><label><span className="sr-only">Event</span><input required maxLength={200} value={title} onChange={e => setTitle(e.target.value)} placeholder="New event" /></label><label><span className="sr-only">Time</span><input required type="time" value={time} onChange={e => setTime(e.target.value)} /></label><Button type="submit">Add event</Button></form></aside></div></>;
}
function Clock() {
  const [now,setNow] = useState(Date.now()), [tab,setTab] = useState("Clock"), [running,setRunning] = useState(false), [elapsed,setElapsed] = useState(0), [started,setStarted] = useState(0), [minutes,setMinutes] = useState("5"), [deadline,setDeadline] = useState<number | null>(null), [done,setDone] = useState(false), [laps,setLaps] = useState<number[]>([]);
  useEffect(() => {
    let timer: ReturnType<typeof setInterval> | undefined;
    const sync = () => {
      if (timer !== undefined) clearInterval(timer);
      timer = undefined;
      if (!document.hidden) {
        setNow(Date.now());
        timer = setInterval(() => setNow(Date.now()), tab === "Stopwatch" && running ? 100 : 1000);
      }
    };
    sync(); document.addEventListener("visibilitychange", sync);
    return () => { if (timer !== undefined) clearInterval(timer); document.removeEventListener("visibilitychange", sync); };
  }, [tab, running]);
  useEffect(() => { if(deadline!==null && now>=deadline) { setDeadline(null); setDone(true); } },[now,deadline]);
  const value = elapsed+(running?Math.max(0,now-started):0);
  function duration(ms:number) { const s=Math.max(0,Math.floor(ms/1000)); return `${String(Math.floor(s/3600)).padStart(2,"0")}:${String(Math.floor(s/60)%60).padStart(2,"0")}:${String(s%60).padStart(2,"0")}`; }
  return <><header><span className="app-mark">◷</span><div><h1>Clock</h1></div></header><Tabs className="tabs" label="Clock mode" value={tab} onValueChange={setTab} items={["Clock", "Stopwatch", "Timer"].map(value => ({ value, label: value }))}><section className="clock-face">{tab==="Clock" ? <><p className="muted">{Intl.DateTimeFormat().resolvedOptions().timeZone.split("/").pop()?.replaceAll("_", " ")}</p><div className="digits">{new Date(now).toLocaleTimeString()}</div><p>{new Date(now).toLocaleDateString(undefined,{weekday:"long",month:"long",day:"numeric"})}</p></> : tab==="Stopwatch" ? <><div className="digits">{duration(value)}<small>.{String(Math.floor(value%1000/10)).padStart(2,"0")}</small></div><div className="toolbar"><Button onClick={() => { if(running) setElapsed(value); else setStarted(Date.now()); setRunning(!running); }}>{running?"Pause":"Start"}</Button><Button disabled={!running} onClick={() => setLaps([...laps,value])}>Lap</Button><Button onClick={() => { setRunning(false); setElapsed(0); setLaps([]); }}>Reset</Button></div>{laps.map((v,i) => <p key={i}>Lap {i+1} · {duration(v)}</p>)}</> : <><div className="digits">{duration(deadline===null?Number(minutes)*60000:deadline-now)}</div><label>Minutes<input type="number" min="1" max="1440" disabled={deadline!==null} value={minutes} onChange={e => setMinutes(e.target.value)} /></label><div className="toolbar"><Button disabled={deadline!==null || !Number.isFinite(Number(minutes)) || Number(minutes)<1 || Number(minutes)>1440} onClick={() => { setDeadline(Date.now()+Number(minutes)*60000); setDone(false); }}>Start timer</Button><Button onClick={() => { setDeadline(null); setDone(false); }}>Reset</Button></div>{done && <p role="status">Time’s up!</p>}<p className="muted">Keep Clock open. Timer is silent.</p></>}</section></Tabs></>;
}
function Utilities() { const [kind,setKind] = useState<SurfaceKind | null>(null),[error,setError] = useState(""); useEffect(() => { void hello().then(info => setKind(info.surface)).catch(e => setError(errorMessage(e))); },[]); return <main>{error?<p role="alert">{error}</p>:kind==="calendar"?<Calendar/>:kind==="clock"?<Clock/>:<p>Opening…</p>}</main>; }
mount(<Utilities/>);
