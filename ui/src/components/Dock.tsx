import { useEffect, useRef, useState, type DragEvent } from "react";
import { on, request } from "../lib/bridge";
import { isBuiltinApp } from "../lib/apps";
import { AppIcon } from "./AppIcon";
import type { WindowInfo } from "../lib/generated/WindowInfo";
import type { SearchItem } from "../lib/generated/SearchItem";
export const APP_DRAG = "application/x-meridian-app";
export const DOCK_DRAG = "application/x-meridian-dock-item";
const report = (error: unknown) => console.error("meridian dock:", error);
type Item = { key: string; name: string; app?: SearchItem; special?: "applications" | "trash" };

export function Dock() {
  const dock = useRef<HTMLElement>(null);
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [apps, setApps] = useState<SearchItem[]>([]);
  const [pins, setPins] = useState<string[]>([]);
  const [layout, setLayout] = useState<string[]>([]);
  const [expanded, setExpanded] = useState(false);
  const [dropping, setDropping] = useState<string | null>(null);
  const [context, setContext] = useState<Item | null>(null);
  const [status, setStatus] = useState("");
  useEffect(() => {
    let alive = true, revision = 0, layoutRevision = 0, pinsRevision = 0;
    const off = [on("windows.changed", e => { revision++; setWindows(e.data); }), on("dock.changed", e => { pinsRevision++; setPins(e.data); }),
      on("dock.layout_changed", e => { layoutRevision++; setLayout(e.data); }),
      on("popover.state", e => setExpanded(e.data.applications)), on("search.invalidated", () => { void loadApps(); })];
    async function loadApps() {
      try { const result = await request({ method: "search.query", params: { query: "", serial: 0 } }); if (alive) setApps(result?.sections.flatMap(s => s.items) ?? []); } catch (e) { report(e); }
    }
    const mine = revision, initialLayout = layoutRevision, initialPins = pinsRevision;
    void request({ method: "windows.list" }).then(result => { if (alive && mine === revision) setWindows(result ?? []); }).catch(report);
    void request({ method: "dock.list" }).then(result => { if (alive && initialPins === pinsRevision) setPins(result ?? []); }).catch(report);
    void request({ method: "dock.layout_get" }).then(result => { if (alive && initialLayout === layoutRevision) setLayout(result ?? []); }).catch(report);
    void loadApps();
    return () => { alive = false; off.forEach(f => f()); };
  }, []);
  const pinned = pins.flatMap(id => { const app = apps.find(a => a.id === id); return app ? [app] : []; });
  const groups = Array.from(new Set(windows.map(w => w.name)));
  const items: Item[] = [{ key: "@applications", name: "Applications", special: "applications" },
    ...pinned.map(app => ({ key: app.id, name: app.title, app })),
    ...groups.filter(name => !pinned.some(app => app.title.toLowerCase() === name.toLowerCase())).map(name => {
      const app = apps.find(a => a.title.toLowerCase() === name.toLowerCase());
      return { key: app?.id ?? `@window:${name}`, name, app };
    }), { key: "@trash", name: "Trash", special: "trash" }];
  const ordered = [...items].sort((a, b) => {
    const ai = layout.indexOf(a.key), bi = layout.indexOf(b.key);
    return (ai < 0 ? layout.length + items.indexOf(a) : ai) - (bi < 0 ? layout.length + items.indexOf(b) : bi);
  });
  const fail = (e: unknown) => { report(e); setStatus(e instanceof Error ? e.message : "Couldn’t update the dock"); };
  function start(event: DragEvent, item: Item) {
    event.dataTransfer.setData(DOCK_DRAG, item.key);
    if (item.app && !isBuiltinApp(item.app.id)) event.dataTransfer.setData(APP_DRAG, item.app.id);
    event.dataTransfer.effectAllowed = "copyMove";
    setContext(null);
  }
  async function drop(event: DragEvent, target?: Item) {
    event.preventDefault(); event.stopPropagation(); setDropping(null); setStatus("");
    const moving = event.dataTransfer.getData(DOCK_DRAG), appId = event.dataTransfer.getData(APP_DRAG);
    // Dropping an app on Trash always uses the same reviewed uninstall dialog.
    if (target?.special === "trash" && appId && moving !== target.key) {
      if (isBuiltinApp(appId)) return;
      if (apps.some(app => app.id === appId)) await request({ method: "apps.uninstall_prompt", params: { id: appId } }).catch(fail);
      return;
    }
    const key = moving || appId;
    if (!items.some(item => item.key === key) && !apps.some(app => app.id === key)) return;
    if (target?.key === key && moving) return;
    let next = ordered.map(item => item.key).filter(id => id !== key);
    if (!moving && appId) {
      try { await request({ method: "dock.pin", params: { id: appId, pinned: true } }); } catch (e) { fail(e); return; }
    }
    const buttons = Array.from(dock.current?.querySelectorAll<HTMLElement>("[data-dock-key]") ?? []);
    const index = next.findIndex(id => {
      const button = buttons.find(element => element.dataset.dockKey === id);
      return button ? event.clientX < button.getBoundingClientRect().left + button.getBoundingClientRect().width / 2 : false;
    });
    next.splice(index < 0 ? next.length : index, 0, key);
    try { await request({ method: "dock.layout_set", params: { items: next } }); setLayout(next); } catch (e) { fail(e); }
  }
  function over(event: DragEvent, key: string) {
    if (![APP_DRAG, DOCK_DRAG].some(type => Array.from(event.dataTransfer.types).includes(type))) return;
    event.preventDefault(); event.stopPropagation(); event.dataTransfer.dropEffect = key === "@trash" ? "move" : "copy"; setDropping(key);
  }
  function click(item: Item) {
    if (item.special === "applications") { void request({ method: "popover.toggle", params: { popover: "applications" } }).catch(fail); return; }
    if (item.special === "trash") { void request({ method: "trash.open" }).catch(fail); return; }
    const running = windows.filter(w => w.name.toLowerCase() === item.name.toLowerCase());
    const window = running.find(w => w.active) ?? running[0];
    void (window ? request({ method: "windows.activate", params: { id: window.id } }) : item.app ? request({ method: "search.activate", params: { provider: "apps", item_id: item.app.id } }) : Promise.resolve()).catch(fail);
  }
  return <nav ref={dock} className={`bar${dropping ? " drop-active" : ""}`} aria-label="Application dock"
    onDragOver={event => over(event, "end")} onDrop={event => void drop(event)} onDragEnd={() => setDropping(null)}
    onDragLeave={event => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDropping(null); }}
    onKeyDown={event => { if (event.key === "Escape") { setContext(null); setStatus(""); event.stopPropagation(); } }}>
    <ul className="windows dock-items" aria-label="Pinned and running applications">
      {ordered.map(item => {
        const running = windows.filter(w => w.name.toLowerCase() === item.name.toLowerCase()), active = running.some(w => w.active);
        return <li key={item.key} className={dropping === item.key ? "dock-drop-target" : undefined}>
          <button id={item.special} data-dock-key={item.key} className={item.special ? "bar-button" : `window${running.length ? " running" : ""}`}
            aria-label={item.name} aria-pressed={item.special ? undefined : active} aria-haspopup={item.special === "applications" ? "dialog" : undefined}
            aria-expanded={item.special === "applications" ? expanded : undefined} data-tooltip={item.name}
            draggable onDragStart={event => start(event, item)} onDragOver={event => over(event, item.key)} onDrop={event => void drop(event, item)}
            onContextMenu={event => { event.preventDefault(); if (item.app) setContext(item); }} onClick={() => click(item)}>
            {item.special === "applications" ? <AppIcon name="Applications" icon="view-app-grid" /> : item.special === "trash" ? <AppIcon name="Trash" icon="user-trash" /> : <AppIcon id={item.app?.id} name={item.name} icon={item.app?.icon} />}
          </button>
        </li>;
      })}
    </ul>
    {context && <div className="dock-context" role="menu" aria-label={`${context.name} actions`}>
      <strong>{context.name}</strong>
      {pins.includes(context.key) && <button role="menuitem" onClick={() => { void request({ method: "dock.pin", params: { id: context.key, pinned: false } }).catch(fail); setContext(null); }}>Remove from dock</button>}
      {context.app && !isBuiltinApp(context.app.id) && <button role="menuitem" autoFocus onClick={() => { void request({ method: "apps.uninstall_prompt", params: { id: context.app!.id } }).catch(fail); setContext(null); }}>Uninstall…</button>}
      <button role="menuitem" onClick={() => setContext(null)}>Cancel</button>
    </div>}
    {status && <button className="dock-error" role="status" onClick={() => setStatus("")}>{status}</button>}
  </nav>;
}
