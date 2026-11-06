import { useEffect, useId, useLayoutEffect, useRef, useState, type SelectHTMLAttributes } from "react";
import { createPortal } from "react-dom";
import { on } from "../lib/bridge";
type Props = Omit<SelectHTMLAttributes<HTMLSelectElement>, "multiple"> & { multiple?: false };
type Option = { value: string; label: string; disabled: boolean };
/** Meridian listbox: focus stays on the trigger, including inside modal dialogs. */
export function Select({ size, className = "", id, style, ...props }: Props) {
  const native = useRef<HTMLSelectElement>(null), trigger = useRef<HTMLButtonElement>(null), menu = useRef<HTMLDivElement>(null);
  const menuId = useId();
  const [options, setOptions] = useState<Option[]>([]), [value, setValue] = useState("");
  const [opened, setOpened] = useState(false), [cursor, setCursor] = useState(0), [invalid, setInvalid] = useState(false);
  const [bounds, setBounds] = useState({ left:8, top:8, width:180, maxHeight:280 });
  const typed = useRef({ text:"", time:0 });
  useLayoutEffect(() => {
    const element = native.current; if (!element) return;
    setOptions(Array.from(element.options).map(option => ({ value:option.value, label:option.label, disabled:option.disabled || (option.parentElement instanceof HTMLOptGroupElement && option.parentElement.disabled) })));
    setValue(element.value); setInvalid(!element.validity.valid);
  }, [props.children, props.value, props.defaultValue]);
  useEffect(() => {
    const form = native.current?.form; if (!form) return;
    let frame = 0;
    const reset = () => { setOpened(false); frame = requestAnimationFrame(() => { if (native.current) { setValue(native.current.value); setInvalid(!native.current.validity.valid); } }); };
    form.addEventListener("reset", reset);
    return () => { form.removeEventListener("reset", reset); cancelAnimationFrame(frame); };
  }, []);
  useEffect(() => { if (props.disabled) setOpened(false); }, [props.disabled]);
  useEffect(() => on("popover.dismiss", () => setOpened(false)), []);
  useEffect(() => {
    if (!opened) return;
    const close = () => setOpened(false);
    const outside = (event: PointerEvent) => { if (event.target instanceof Node && !trigger.current?.contains(event.target) && !menu.current?.contains(event.target)) close(); };
    const scroll = (event: Event) => { if (event.target instanceof Node && !menu.current?.contains(event.target)) close(); };
    document.addEventListener("pointerdown", outside); document.addEventListener("scroll", scroll, true);
    window.addEventListener("blur", close); window.addEventListener("resize", close);
    return () => { document.removeEventListener("pointerdown", outside); document.removeEventListener("scroll", scroll, true); window.removeEventListener("blur", close); window.removeEventListener("resize", close); };
  }, [opened]);
  useEffect(() => {
    const list = menu.current;
    const option = list?.querySelector<HTMLElement>('[data-highlighted="true"]');
    if (!opened || !list || !option) return;
    // Scroll only the list: scrollIntoView can scroll the page and dismiss it.
    const top = option.offsetTop, bottom = top + option.offsetHeight;
    if (top < list.scrollTop) list.scrollTop = top;
    else if (bottom > list.scrollTop + list.clientHeight) list.scrollTop = bottom - list.clientHeight;
  }, [cursor, opened]);
  function show() {
    if (props.disabled || !options.some(option => !option.disabled) || !trigger.current) return;
    const rect = trigger.current.getBoundingClientRect();
    const width = Math.max(1, Math.min(Math.max(rect.width, 160), window.innerWidth - 16));
    const below = window.innerHeight - rect.bottom - 12, above = rect.top - 12;
    const upwards = below < 160 && above > below;
    const maxHeight = Math.max(1, Math.min(280, upwards ? above : below));
    const height = Math.min(maxHeight, options.length * 38 + 12);
    setBounds({ left:Math.max(8, Math.min(rect.left, window.innerWidth - width - 8)), top:upwards ? Math.max(8, rect.top - height - 6) : rect.bottom + 6, width, maxHeight });
    const selected = options.findIndex(option => option.value === value && !option.disabled);
    setCursor(selected >= 0 ? selected : options.findIndex(option => !option.disabled));
    setOpened(true);
  }
  function choose(index: number) {
    const option = options[index]; if (!option || option.disabled || !native.current) return;
    Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set?.call(native.current, option.value);
    native.current.dispatchEvent(new Event("change", { bubbles:true }));
    setValue(native.current.value); setInvalid(!native.current.validity.valid); setOpened(false); trigger.current?.focus();
  }
  const selected = options.find(option => option.value === value);
  return <span className={`meridian-select ${className}`} style={style}>
    <select {...props} ref={native} id={id ? `${id}-native` : undefined} hidden aria-hidden="true" tabIndex={-1}
      onInvalid={event => { props.onInvalid?.(event); event.preventDefault(); setInvalid(true); trigger.current?.focus(); }} />
    <button ref={trigger} type="button" id={id} className="meridian-select-input" role="combobox" aria-haspopup="listbox"
      aria-label={props["aria-label"]} aria-labelledby={props["aria-labelledby"]} aria-describedby={props["aria-describedby"]}
      aria-invalid={props["aria-invalid"] ?? (invalid || undefined)} aria-required={props.required || undefined}
      aria-expanded={opened} aria-controls={opened ? menuId : undefined} aria-activedescendant={opened ? `${menuId}-${cursor}` : undefined}
      disabled={props.disabled} title={props.title} onBlur={() => setOpened(false)} onClick={event => { event.preventDefault(); opened ? setOpened(false) : show(); }}
      onKeyDown={event => {
        const enabled = options.map((option,index) => option.disabled ? -1 : index).filter(index => index >= 0);
        if (event.key === "Tab") { setOpened(false); return; }
        if (event.key === "Escape" && opened) { event.preventDefault(); event.stopPropagation(); setOpened(false); return; }
        if (["ArrowDown","ArrowUp","Home","End"].includes(event.key)) {
          event.preventDefault(); event.stopPropagation(); if (!enabled.length) return;
          if (!opened) { show(); if (event.key === "Home") setCursor(enabled[0]!); if (event.key === "End") setCursor(enabled[enabled.length-1]!); }
          else { const current = enabled.indexOf(cursor); setCursor(enabled[event.key === "Home" ? 0 : event.key === "End" ? enabled.length-1 : (current + (event.key === "ArrowUp" ? enabled.length-1 : 1)) % enabled.length]!); }
        } else if (event.key === "Enter" || event.key === " ") { event.preventDefault(); event.stopPropagation(); if (opened) choose(cursor); else show(); }
        else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
          event.preventDefault(); const now = performance.now(); typed.current.text = now - typed.current.time < 700 ? typed.current.text + event.key.toLowerCase() : event.key.toLowerCase(); typed.current.time = now;
          const found = options.findIndex(option => !option.disabled && option.label.toLowerCase().startsWith(typed.current.text));
          if (!opened) show(); if (found >= 0) setCursor(found);
        }
      }}><span>{selected?.label ?? "Choose…"}</span><span aria-hidden="true" className="muk-select-chevron">⌄</span></button>
    {opened && createPortal(<div ref={menu} id={menuId} role="listbox" aria-label={props["aria-label"] ?? "Options"} className="meridian-select-dropdown" style={bounds}
      onPointerDown={event => event.preventDefault()}>{options.map((option,index) => <div key={`${option.value}-${index}`} id={`${menuId}-${index}`} role="option" aria-selected={value === option.value} aria-disabled={option.disabled || undefined}
        className="meridian-select-option" data-highlighted={cursor === index} data-selected={value === option.value} onPointerMove={() => { if (!option.disabled) setCursor(index); }}
        onClick={event => { event.preventDefault(); event.stopPropagation(); choose(index); }}><span>{option.label}</span>{value === option.value && <span aria-hidden="true">✓</span>}</div>)}</div>, trigger.current?.closest('[role="dialog"], dialog') ?? document.body)}
  </span>;
}
