import { useEffect, useRef, useState, type DragEvent } from "react";
import { on, request } from "../lib/bridge";
import { isBuiltinApp } from "../lib/apps";
import { AppIcon } from "./AppIcon";
import type { WindowInfo } from "../lib/generated/WindowInfo";
import type { SearchItem } from "../lib/generated/SearchItem";
export const APP_DRAG = "application/x-meridian-app";
export const DOCK_DRAG = "application/x-meridian-dock-item";
const report = (error: unknown) => console.error("meridian panel:", error);
type Item = { key: string; name: string; app?: SearchItem; special?: "applications" };

export function Dock() {
  const dock = useRef<HTMLElement>(null);
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [apps, setApps] = useState<SearchItem[]>([]);
  const [pins, setPins] = useState<string[]>([]);
  const [layout, setLayout] = useState<string[]>([]);
  const [expanded, setExpanded] = useState(false);
  const [dropping, setDropping] = useState<string | null>(null);
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
    })];
  const ordered = [...items].sort((a, b) => {
    const ai = layout.indexOf(a.key), bi = layout.indexOf(b.key);
    return (ai < 0 ? layout.length + items.indexOf(a) : ai) - (bi < 0 ? layout.length + items.indexOf(b) : bi);
  });
  const fail = (e: unknown) => { report(e); setStatus(e instanceof Error ? e.message : "Couldn’t update the panel"); };
  function start(event: DragEvent, item: Item) {
    event.dataTransfer.setData(DOCK_DRAG, item.key);
    if (item.app && !isBuiltinApp(item.app.id)) event.dataTransfer.setData(APP_DRAG, item.app.id);
    event.dataTransfer.effectAllowed = "copyMove";
  }
  async function drop(event: DragEvent, target?: Item) {
    event.preventDefault(); event.stopPropagation(); setDropping(null); setStatus("");
    const moving = event.dataTransfer.getData(DOCK_DRAG), appId = event.dataTransfer.getData(APP_DRAG);
    const key = moving || appId;
    if (!items.some(item => item.key === key) && !apps.some(app => app.id === key)) return;
    if (target?.key === key && moving) return;
    let next = ordered.map(item => item.key).filter(id => id !== key);
    if (!moving && appId) {
      try { await request({ method: "dock.pin", params: { id: appId, pinned: true } }); } catch (e) { fail(e); return; }
    }
    const buttons = Array.from(dock.current?.querySelectorAll<HTMLElement>("[data-dock-key]") ?? []);
    const targetButton = target ? buttons.find(button => button.dataset.dockKey === target.key) : undefined;
    const bounds = targetButton?.getBoundingClientRect();
    const index = target && bounds ? next.indexOf(target.key) + (event.clientX >= bounds.left + bounds.width / 2 ? 1 : 0) : next.findIndex(id => {
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
    const running = windows.filter(w => w.name.toLowerCase() === item.name.toLowerCase());
    const current = running.findIndex(w => w.active);
    const window = running[current < 0 ? 0 : (current + 1) % running.length];
    void (window ? request({ method: "windows.activate", params: { id: window.id } }) : item.app ? request({ method: "search.activate", params: { provider: "apps", item_id: item.app.id } }) : Promise.resolve()).catch(fail);
  }
  return <nav ref={dock} className={`bar${dropping ? " drop-active" : ""}`} aria-label="Application panel"
    onDragOver={event => over(event, "end")} onDrop={event => void drop(event)} onDragEnd={() => setDropping(null)}
    onDragLeave={event => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDropping(null); }}
    onKeyDown={event => { if (event.key === "Escape") { setStatus(""); event.stopPropagation(); } }}>
    <ul className="windows dock-items" aria-label="Pinned and running applications">
      {ordered.map(item => {
        const running = windows.filter(w => w.name.toLowerCase() === item.name.toLowerCase()), active = running.some(w => w.active);
        return <li key={item.key} className={dropping === item.key ? "dock-drop-target" : ""}>
          <button id={item.special} data-dock-key={item.key} className={item.special ? "bar-button" : `window${running.length ? " running" : ""}`}
            title={running.length > 1 ? `${item.name} · ${running.length} windows` : item.name} aria-label={item.name} aria-description={running.length > 1 ? `${running.length} windows. Click to switch to the next window.` : undefined} aria-pressed={item.special ? undefined : active} aria-haspopup={item.special === "applications" ? "dialog" : undefined}
            aria-expanded={item.special === "applications" ? expanded : undefined} data-tooltip={running.length > 1 ? `${item.name} · ${running.length} windows` : item.name}
            draggable onDragStart={event => start(event, item)} onDragOver={event => over(event, item.key)} onDrop={event => void drop(event, item)}
            onContextMenu={event => {
              event.preventDefault();
              if (item.special) return;
              const activeWindow = running.find(window => window.active) ?? running[0];
              void request({ method: "menu.open", params: {
                menu: "DockItem", x: Math.round(event.clientX),
                window_id: activeWindow?.id,
                item_id: item.app?.id, item_name: item.name, pinned: pins.includes(item.key),
              } }).catch(fail);
            }} onClick={() => click(item)}>
            {item.special === "applications" ? <AppIcon name="Applications" icon="view-app-grid" /> : <AppIcon id={item.app?.id} name={item.name} icon={item.app?.icon} />}
            {running.length > 1 && <span className="dock-window-count" aria-hidden="true">{running.length > 99 ? "99+" : running.length}</span>}
          </button>
        </li>;
      })}
    </ul>
    {status && <button className="dock-error" role="status" onClick={() => setStatus("")}>{status}</button>}
  </nav>;
}
