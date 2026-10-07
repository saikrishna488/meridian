import { useEffect, useRef, useState, type CSSProperties } from "react";
import { request } from "../lib/bridge";
export function SystemSlider({ name, label, value, report }: { name: "brightness" | "volume"; label: string; value: number | null; report: (e: unknown) => void }) {
 const [local, setLocal] = useState(value ?? 0), timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
 useEffect(() => { if (value !== null) setLocal(value); }, [value]);
 useEffect(() => () => clearTimeout(timer.current), []);
 function change(percent: number) { setLocal(percent); clearTimeout(timer.current); timer.current = setTimeout(() => { void request({ method: name === "brightness" ? "settings.set_brightness" : "settings.set_volume", params: { percent } }).catch(report); }, 100); }
 return <div className="row slider-row"><label className="label" htmlFor={name}>{label}</label><input className="slider" id={name} aria-label={label} type="range" min="0" max="100" value={local} disabled={value === null} style={{ "--fill": `${local}%` } as CSSProperties} onChange={e => change(Number(e.target.value))} /><output className="value" id={`${name}-value`} htmlFor={name}>{value === null ? "Unavailable" : `${local}%`}</output></div>;
}
