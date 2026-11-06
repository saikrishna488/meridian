import { useEffect, useRef, useState } from "react";
import { Slider } from "./ui/slider";
import { request } from "../lib/bridge";
export function SystemSlider({ name, label, value, report }: { name: "brightness" | "volume"; label: string; value: number | null; report: (e: unknown) => void }) {
 const [local, setLocal] = useState(value ?? 0);
 const [status, setStatus] = useState("");
 const current = useRef(value), pending = useRef<number | null>(null), inFlight = useRef(false), editing = useRef(false), alive = useRef(true);
 const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
 useEffect(() => { current.current = value; if (!editing.current && !inFlight.current && pending.current === null && value !== null) setLocal(value); }, [value]);
 useEffect(() => { alive.current = true; return () => { alive.current = false; clearTimeout(timer.current); }; }, []);
 async function flush() {
  clearTimeout(timer.current); timer.current = undefined;
  if (inFlight.current || pending.current === null || !alive.current) return;
  const percent = pending.current; pending.current = null; inFlight.current = true;
  try {
   await request({ method: name === "brightness" ? "settings.set_brightness" : "settings.set_volume", params: { percent } });
   if (pending.current === null && !editing.current) {
    const actual = await request({method:"settings.get"});
    if (alive.current && pending.current === null && !editing.current) {
     const confirmed = actual?.[name] ?? null;
     current.current = confirmed;
     setLocal(confirmed ?? 0);
     setStatus(confirmed === null ? "Unable to read device state" : Math.abs(confirmed - percent) <= 1 ? "Updated" : "Device reported a different value");
    }
   }
  }
  catch(error) { if (alive.current) { if (pending.current === null && !editing.current) setLocal(current.current ?? 0); setStatus("Update failed"); report(error); } }
  finally {
   inFlight.current = false;
   if (alive.current && pending.current !== null) timer.current = setTimeout(() => void flush(), 80);
  }
 }
 function change(percent: number) {
  editing.current = true; setStatus("Updating…"); setLocal(percent); pending.current = percent;
  // Throttle, rather than debounce: apply values during a continuous drag.
  if (!inFlight.current && timer.current === undefined) timer.current = setTimeout(() => { timer.current = undefined; void flush(); }, 80);
 }
 function commit(percent: number) { editing.current = false; if (pending.current === null && !inFlight.current) pending.current = percent; clearTimeout(timer.current); timer.current = undefined; void flush(); }
 return <><div className="row slider-row"><label className="label" htmlFor={name}>{label}</label><Slider id={name} aria-label={label} min={0} max={100} value={local} disabled={value === null} onValueChange={change} onChangeEnd={commit} aria-describedby={`${name}-status`} /><output className="value" id={`${name}-value`} htmlFor={name}>{value === null ? "Unavailable" : `${local}%`}</output></div><small className="system-control-status" id={`${name}-status`} role="status">{status}</small></>;
}
