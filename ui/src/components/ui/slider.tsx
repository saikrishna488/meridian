import { useEffect, useRef, useState, type CSSProperties, type ComponentProps, type PointerEvent } from "react";
type Props = Omit<ComponentProps<"input">, "type" | "onChange" | "value" | "defaultValue"> & {
  resetToken?: number; value?: number; defaultValue?: number; onValueChange?: (value: number) => void; onChangeEnd?: (value: number) => void;
};
// Interaction reference: Radix Primitives Slider (MIT), slide/move/commit model.
// Keep a native range for labels, keyboard stepping and assistive technology.
export function Slider({ value, defaultValue, resetToken, onValueChange, onChangeEnd, onPointerDown, onPointerMove, onPointerUp, onPointerCancel, onLostPointerCapture, onInput, onKeyUp, onBlur, className = "", style, ...props }: Props) {
  const min = Number(props.min ?? 0), max = Number(props.max ?? 100);
  const [local, setLocal] = useState(value ?? defaultValue ?? (min + max) / 2);
  const latest = useRef(local), changed = useRef(false), pointer = useRef<number | null>(null);
  useEffect(() => { if (value !== undefined && pointer.current === null) { latest.current = value; setLocal(value); } }, [value, resetToken]);
  function update(next: number) {
    if (!Number.isFinite(next) || next === latest.current) return;
    latest.current = next; changed.current = true; setLocal(next); onValueChange?.(next);
  }
  function commit() { if (changed.current) { changed.current = false; onChangeEnd?.(latest.current); } }
  function move(event: PointerEvent<HTMLInputElement>) {
    const element = event.currentTarget, rect = element.getBoundingClientRect();
    // Native thumbs travel inside the input edges, rather than beyond them.
    const inset = Math.min(8, rect.width / 2);
    let ratio = Math.max(0, Math.min(1, (event.clientX - rect.left - inset) / Math.max(1, rect.width - inset * 2)));
    if (getComputedStyle(element).direction === "rtl") ratio = 1 - ratio;
    const raw = min + ratio * (max - min), step = Number(props.step ?? 1);
    const next = props.step === "any" ? raw : min + Math.round((raw - min) / (step > 0 ? step : 1)) * (step > 0 ? step : 1);
    const clamped = Math.max(min, Math.min(max, Number(next.toFixed(8))));
    element.value = String(clamped); update(element.valueAsNumber);
  }
  function finish(event: PointerEvent<HTMLInputElement>) {
    if (pointer.current !== event.pointerId) return;
    pointer.current = null; commit();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  }
  const percent = max > min ? Math.max(0, Math.min(100, (local - min) / (max - min) * 100)) : 0;
  return <input {...props} value={local} type="range" style={{ ...style, "--muk-range-fill": `${percent}%`, touchAction: "none" } as CSSProperties} className={`ui-slider ${className}`}
    onInput={event => { update(event.currentTarget.valueAsNumber); onInput?.(event); }}
    onChange={event => update(event.currentTarget.valueAsNumber)}
    onPointerDown={event => {
      onPointerDown?.(event);
      if (event.defaultPrevented || props.disabled || event.button !== 0 || pointer.current !== null) return;
      // Own the drag explicitly: embedded engines can defer native range events.
      event.preventDefault(); event.currentTarget.focus(); pointer.current = event.pointerId;
      event.currentTarget.setPointerCapture(event.pointerId); move(event);
    }}
    onPointerMove={event => { if (pointer.current === event.pointerId) move(event); onPointerMove?.(event); }}
    onPointerUp={event => { if (pointer.current === event.pointerId) move(event); finish(event); onPointerUp?.(event); }}
    onPointerCancel={event => { finish(event); onPointerCancel?.(event); }}
    onLostPointerCapture={event => { finish(event); onLostPointerCapture?.(event); }}
    onKeyUp={event => { commit(); onKeyUp?.(event); }} onBlur={event => { commit(); onBlur?.(event); }} />;
}
