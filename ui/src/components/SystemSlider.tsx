import { useEffect, useRef, useState } from "react";
import { Slider } from "./ui/slider";
import { request } from "../lib/bridge";
export function SystemSlider({ name, label, value, report }: { name: "brightness" | "volume"; label: string; value: number | null; report: (e: unknown) => void }) {
 const [local, setLocal] = useState(value ?? 0), timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined), current = useRef(value), revision = useRef(0);
 useEffect(() => { current.current = value; if (value !== null) setLocal(value); }, [value]);
 useEffect(() => () => clearTimeout(timer.current), []);
 function change(percent: number) { const mine = ++revision.current; setLocal(percent); clearTimeout(timer.current); timer.current = setTimeout(() => { void request({ method: name === "brightness" ? "settings.set_brightness" : "settings.set_volume", params: { percent } }).catch(error => { if (mine === revision.current) setLocal(current.current ?? 0); report(error); }); }, 100); }
 return <div className="row slider-row"><label className="label" htmlFor={name}>{label}</label><Slider id={name} aria-label={label} min={0} max={100} value={local} disabled={value === null} onValueChange={change} /><output className="value" id={`${name}-value`} htmlFor={name}>{value === null ? "Unavailable" : `${local}%`}</output></div>;
}
