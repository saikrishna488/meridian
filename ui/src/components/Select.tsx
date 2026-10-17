import { useEffect, useId, useRef, useState, type SelectHTMLAttributes } from "react";
import { createPortal } from "react-dom";

type Props = SelectHTMLAttributes<HTMLSelectElement>;
/** Styled listbox with a native select retained for forms and change semantics. */
export function Select(props: Props) {
  const id = useId(), native = useRef<HTMLSelectElement>(null), button = useRef<HTMLButtonElement>(null), menu = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false), [cursor, setCursor] = useState(0), [revision, setRevision] = useState(0);
  const [bounds, setBounds] = useState({ left: 0, top: 0, width: 180, maxHeight: 260 });
  const options = Array.from(native.current?.options ?? []), selected = options.find(o => o.value === String(props.value ?? native.current?.value));
  useEffect(() => { setRevision(n => n + 1); }, [props.children, props.value]);
  useEffect(() => { if (props.disabled) setOpen(false); }, [props.disabled]);
  function close() { setOpen(false); button.current?.focus(); }
  function show() {
    if (props.disabled) return;
    const b = button.current!.getBoundingClientRect();
    const height = Math.min(260, options.length * 34 + 10);
    const below = window.innerHeight - b.bottom - 8;
    const top = below >= Math.min(height, 140) ? b.bottom + 5 : Math.max(8, b.top - height - 5);
    setBounds({ left: Math.max(8, Math.min(b.left, window.innerWidth - Math.max(b.width, 180) - 8)), top, width: Math.max(b.width, 180), maxHeight: Math.max(80, Math.min(260, window.innerHeight - top - 8)) });
    setCursor(Math.max(0, options.findIndex(o => o.selected && !o.disabled))); setOpen(true);
  }
  function choose(index: number) {
    const option = options[index]; if (!option || option.disabled || !native.current) return;
    const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set;
    setter?.call(native.current, option.value);
    native.current.dispatchEvent(new Event("change", { bubbles: true }));
    setRevision(n => n + 1); close();
  }
  useEffect(() => {
    if (!open) return;
    menu.current?.focus();
    const dismiss = (e: Event) => { if (!menu.current?.contains(e.target as Node) && !button.current?.contains(e.target as Node)) setOpen(false); };
    const hide = () => setOpen(false);
    document.addEventListener("pointerdown", dismiss); window.addEventListener("resize", hide);
    return () => { document.removeEventListener("pointerdown", dismiss); window.removeEventListener("resize", hide); };
  }, [open]);
  useEffect(() => { if (open) menu.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" }); }, [cursor, open]);
  const label = props["aria-label"] ?? (props.id ? document.querySelector(`label[for="${props.id}"]`)?.textContent : null) ?? "Choose an option";
  return <span className="meridian-select" data-revision={revision}>
    <select {...props} ref={native} className="select-native" tabIndex={-1} aria-hidden="true" onFocus={() => button.current?.focus()} />
    <button ref={button} type="button" className="select-trigger" role="combobox" aria-label={label} aria-expanded={open} aria-controls={id} aria-haspopup="listbox" disabled={props.disabled} title={props.title} onClick={() => open ? close() : show()} onKeyDown={e => { if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) { e.preventDefault(); e.stopPropagation(); show(); } }}><span>{selected?.label ?? options[0]?.label ?? "Choose…"}</span><svg viewBox="0 0 12 12" width="12" height="12" fill="none" aria-hidden="true"><path d="m3 4.5 3 3 3-3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg></button>
    {open && createPortal(<div id={id} ref={menu} className="select-menu" role="listbox" aria-label={label} aria-activedescendant={`${id}-${cursor}`} tabIndex={-1} style={bounds} onKeyDown={e => {
      e.stopPropagation();
      if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(); }
      if (e.key === "Tab") { setOpen(false); button.current?.focus(); }
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); choose(cursor); }
      if (["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) {
        e.preventDefault(); const enabled = options.map((o, i) => o.disabled ? -1 : i).filter(i => i >= 0); const index = enabled.indexOf(cursor);
        setCursor(enabled[e.key === "Home" ? 0 : e.key === "End" ? enabled.length - 1 : (index + (e.key === "ArrowUp" ? enabled.length - 1 : 1)) % enabled.length] ?? 0);
      } else if (e.key.length === 1 && e.key !== " ") { const found = options.findIndex(o => !o.disabled && o.label.toLowerCase().startsWith(e.key.toLowerCase())); if (found >= 0) setCursor(found); }
    }}>{options.map((option, i) => <div key={option.value} id={`${id}-${i}`} role="option" aria-selected={cursor === i} aria-disabled={option.disabled || undefined} className={option.selected ? "chosen" : ""} onPointerMove={() => !option.disabled && setCursor(i)} onClick={() => choose(i)}><span>{option.label}</span><span aria-hidden="true">{option.selected ? "✓" : ""}</span></div>)}</div>, document.body)}
  </span>;
}
