import { SurfaceButton } from "./ui/button";
import { useEffect, useLayoutEffect, useRef, useState, type DragEvent } from "react";
import { on, request } from "../lib/bridge";
import { isBuiltinApp } from "../lib/apps";
import { Icon } from "./Icon";
import { AppIcon } from "./AppIcon";
import { useDesktop } from "../lib/desktop";
import type { WindowInfo } from "../lib/generated/WindowInfo";
import type { SearchItem } from "../lib/generated/SearchItem";
export const APP_DRAG = "application/x-meridian-app";
export const DOCK_DRAG = "application/x-meridian-dock-item";
const report = (error: unknown) => console.error("meridian panel:", error);
type Item = { key: string; name: string; app?: SearchItem; special?: "applications" };

export function Dock({ hidden = false }: { hidden?: boolean }) {
  const dock = useRef<HTMLElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const preview = useRef<HTMLDivElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const { desktop } = useDesktop();
  const [hovered, setHovered] = useState<string | null>(null);
  const [previewKey, setPreviewKey] = useState<string | null>(null);
  const [previewX, setPreviewX] = useState(0);
  const [pending, setPending] = useState<string | null>(null);
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [apps, setApps] = useState<SearchItem[]>([]);
  const [pins, setPins] = useState<string[]>([]);
  const [layout, setLayout] = useState<string[]>([]);
  const [expanded, setExpanded] = useState(false);
  const dragging = useRef<string | null>(null);
  const positions = useRef(new Map<string, number>());
  const [dragOrder, setDragOrder] = useState<string[] | null>(null);
  const [dropping, setDropping] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const [overflow, setOverflow] = useState(false), [atStart, setAtStart] = useState(true), [atEnd, setAtEnd] = useState(true);
  const closing = useRef(new Set<number>());
  const [closingIds, setClosingIds] = useState<number[]>([]);
  const previewTrigger = useRef<HTMLElement | null>(null);
  function closePreview(restore = false) {
    clearTimeout(timer.current); setPreviewKey(null);
    if (restore) { previewTrigger.current?.focus(); clearTimeout(timer.current); }
  }
  useLayoutEffect(() => {
    const element = list.current;
    if (!element) return;
    const update = () => {
      setOverflow(element.scrollWidth > element.clientWidth + 1);
      setAtStart(element.scrollLeft <= 1);
      setAtEnd(element.scrollLeft + element.clientWidth >= element.scrollWidth - 1);
    };
    const observer = new ResizeObserver(update);
    observer.observe(element);
    for (const child of element.children) observer.observe(child);
    element.addEventListener("scroll", update);
    update();
    return () => { observer.disconnect(); element.removeEventListener("scroll", update); };
  }, [apps, windows, pins, desktop?.taskbar?.icon_size]);
  function scrollDock(direction: number) {
    closePreview();
    list.current?.scrollBy({left: direction * Math.max(48, list.current.clientWidth * .75), behavior:"auto"});
  }

  useEffect(() => {
    let alive = true, revision = 0, layoutRevision = 0, pinsRevision = 0;
    const off = [on("windows.changed", e => { revision++; setWindows(e.data); }), on("dock.changed", e => { pinsRevision++; setPins(e.data); }),
      on("dock.layout_changed", e => { layoutRevision++; setLayout(e.data); }),
      on("popover.state", e => setExpanded(e.data.applications)), on("search.invalidated", () => { void loadApps(); void loadWindows(); })];
    async function loadApps() {
      try { const result = await request({ method: "search.query", params: { query: "", serial: 0 } }); if (alive) setApps(result?.sections.flatMap(s => s.items) ?? []); } catch (e) { report(e); }
    }
    async function loadWindows() {
      const mine = revision;
      try {
        const result = await request({ method: "windows.list" });
        if (alive && mine === revision) setWindows(result ?? []);
      } catch (e) { report(e); }
    }
    const initialLayout = layoutRevision, initialPins = pinsRevision;
    void loadWindows();
    void request({ method: "dock.list" }).then(result => { if (alive && initialPins === pinsRevision) setPins(result ?? []); }).catch(report);
    void request({ method: "dock.layout_get" }).then(result => { if (alive && initialLayout === layoutRevision) setLayout(result ?? []); }).catch(report);
    void loadApps();
    return () => { alive = false; off.forEach(f => f()); };
  }, []);
  // Keep installed identity from the shell: a window name is only a label.
  const appForWindow = (window: WindowInfo): SearchItem | undefined => {
    const installed = window.app_id ? apps.find(app => app.id === window.app_id) : undefined;
    return installed ?? (window.app_id ? { id: window.app_id, title: window.name, icon: window.icon ?? null, subtitle: null }
      : apps.find(app => app.title.toLowerCase() === window.name.toLowerCase()));
  };
  const windowKey = (window: WindowInfo) => appForWindow(window)?.id ?? `@window:${window.name}`;
  const runningApps = windows.flatMap(window => { const app = appForWindow(window); return app ? [app] : []; });
  const pinned = pins.flatMap(id => { const app = apps.find(a => a.id === id) ?? runningApps.find(a => a.id === id); return app ? [app] : []; });
  const groups = Array.from(new Map(windows.map(window => [windowKey(window), window])).entries());
  const items: Item[] = [{ key: "@applications", name: "Applications", special: "applications" },
    ...pinned.map(app => ({ key: app.id, name: app.title, app })),
    ...groups.filter(([key]) => !pinned.some(app => app.id === key)).map(([key, window]) => ({ key, name: window.name, app: appForWindow(window) }))];
  const windowsForItem = (item: Item) => windows.filter(window => windowKey(window) === item.key);
  const order = dragOrder ?? layout;
  const ordered = [...items].sort((a, b) => {
    const group = (item: Item) => item.special ? 0 : pins.includes(item.key) ? 1 : 2;
    if (group(a) !== group(b)) return group(a) - group(b);
    const ai = order.indexOf(a.key), bi = order.indexOf(b.key);
    return (ai < 0 ? order.length + items.indexOf(a) : ai) - (bi < 0 ? order.length + items.indexOf(b) : bi);
  });
  useLayoutEffect(() => {
    const next = new Map<string, number>();
    const reduce = document.documentElement.dataset.engineAnimations === "off" || window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    list.current?.querySelectorAll<HTMLElement>("[data-item-key]").forEach(element => {
      const key = element.dataset.itemKey!;
      const left = element.offsetLeft;
      const previous = positions.current.get(key);
      if (!reduce && previous !== undefined && previous !== left && element.animate) {
        element.getAnimations().forEach(animation => animation.cancel());
        element.animate([{ transform: `translateX(${previous - left}px)` }, { transform: "translateX(0)" }],
          { duration: 220, easing: "cubic-bezier(.2,.8,.2,1)" });
      }
      next.set(key, left);
    });
    positions.current = next;
  }, [ordered.map(item => item.key).join("|")]);
  useEffect(() => {
    const update = () => {
      const bar = dock.current;
      if (!bar) return;
      const taskbar = bar.closest<HTMLElement>(".taskbar");
      const bounds = taskbar ? taskbar.getBoundingClientRect() : bar.getBoundingClientRect();
      const rect = preview.current?.getBoundingClientRect();
      const box = (r: DOMRect) => ({ x: Math.floor(r.left), y: Math.floor(r.top), width: Math.ceil(r.right) - Math.floor(r.left), height: Math.ceil(r.bottom) - Math.floor(r.top) });
      // Include the animation envelope, but leave the rest of the transparent surface click-through.
      void request({ method: "dock.input_region", params: { x: Math.floor(bounds.left), y: Math.max(0, Math.floor(bounds.top - (hidden ? 0 : 20))), width: Math.ceil(bounds.width), height: Math.ceil(bounds.height + (hidden ? 0 : 20)), preview: rect ? box(rect) : null } }).catch(report);
    };
    update();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(update);
    if (dock.current) observer?.observe(dock.current);
    const taskbar = dock.current?.closest<HTMLElement>(".taskbar");
    if (taskbar) observer?.observe(taskbar);
    if (preview.current) observer?.observe(preview.current);
    window.addEventListener("resize", update);
    return () => { observer?.disconnect(); window.removeEventListener("resize", update); };
  }, [ordered.length, previewKey, desktop?.dock_style, hidden, desktop?.taskbar?.icon_size]);
  useEffect(() => () => clearTimeout(timer.current), []);
  useEffect(() => {
    const element = list.current;
    if (!element) return;
    const scroll = (event: WheelEvent) => {
      if (element.scrollWidth <= element.clientWidth || !event.deltaY || Math.abs(event.deltaX) > Math.abs(event.deltaY)) return;
      const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? element.clientWidth : 1;
      const previous = element.scrollLeft;
      element.scrollLeft += event.deltaY * unit;
      if (element.scrollLeft !== previous) event.preventDefault();
    };
    element.addEventListener("wheel", scroll, { passive: false });
    return () => element.removeEventListener("wheel", scroll);
  }, []);
  function showPreview(item: Item, button: HTMLElement) {
    clearTimeout(timer.current);
    previewTrigger.current = button;
    setHovered(item.key);
    const rect = button.getBoundingClientRect();
    setPreviewX(rect.left + rect.width / 2);
    timer.current = setTimeout(() => setPreviewKey(item.key), 400);
  }
  function hidePreview() {
    clearTimeout(timer.current);
    setHovered(null);
    timer.current = setTimeout(() => { if (!preview.current?.contains(document.activeElement)) setPreviewKey(null); }, 220);
  }
  useEffect(() => {
    window.addEventListener("meridian-panel-leave", hidePreview);
    return () => window.removeEventListener("meridian-panel-leave", hidePreview);
  }, []);
  useEffect(() => { if (hidden) { clearTimeout(timer.current); setPreviewKey(null); } }, [hidden]);
  const fail = (e: unknown) => { report(e); setStatus(e instanceof Error ? e.message : "Couldn’t update the panel"); };
  function start(event: DragEvent, item: Item) {
    dragging.current = item.key;
    clearTimeout(timer.current); setPreviewKey(null); setHovered(null);
    event.dataTransfer.setData(DOCK_DRAG, item.key);
    if (item.app && !isBuiltinApp(item.app.id)) event.dataTransfer.setData(APP_DRAG, item.app.id);
    event.dataTransfer.effectAllowed = "copyMove";
  }
  async function drop(event: DragEvent, target?: Item) {
    event.preventDefault(); event.stopPropagation(); setDropping(null); dragging.current = null; setStatus("");
    const moving = event.dataTransfer.getData(DOCK_DRAG), appId = event.dataTransfer.getData(APP_DRAG);
    const key = moving || appId;
    if (!items.some(item => item.key === key) && !apps.some(app => app.id === key)) return;
    if (target?.key === key && moving && !dragOrder) return;
    if (moving && dragOrder) {
      const next = ordered.map(item => item.key);
      setLayout(next); setDragOrder(null);
      try { await request({ method: "dock.layout_set", params: { items: next } }); } catch (e) { fail(e); }
      return;
    }
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
    const moving = dragging.current;
    if (!moving || moving === key) return;
    const source = ordered.find(item => item.key === moving);
    const target = ordered.find(item => item.key === key);
    if (!source || !target || source.special || target.special || pins.includes(moving) !== pins.includes(key)) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    const after = event.clientX >= bounds.left + bounds.width / 2;
    const next = ordered.map(item => item.key).filter(id => id !== moving);
    next.splice(next.indexOf(key) + (after ? 1 : 0), 0, moving);
    if (next.join("|") !== ordered.map(item => item.key).join("|")) setDragOrder(next);
  }
  function click(item: Item) {
    if (item.special === "applications") { void request({ method: "popover.toggle", params: { popover: "applications" } }).catch(fail); return; }
    const running = windowsForItem(item);
    const current = running.findIndex(w => w.active);
    const window = running[current < 0 ? 0 : (current + 1) % running.length];
    if (!window && item.app) setPending(item.key);
    void (window ? request({ method: "windows.activate", params: { id: window.id } }) : item.app ? request({ method: "search.activate", params: { provider: "apps", item_id: item.app.id } }) : Promise.resolve()).catch(fail).finally(() => setPending(null));
  }
  async function closeWindow(id: number) {
    if (closing.current.has(id)) return;
    closing.current.add(id); setClosingIds([...closing.current]); setStatus("");
    try { await request({method:"windows.close",params:{id}}); closePreview(true); }
    catch (error) { fail(error); }
    finally { closing.current.delete(id); setClosingIds([...closing.current]); }
  }
  const previewItem = ordered.find(item => item.key === previewKey);
  const previewWindows = previewItem ? windowsForItem(previewItem) : [];
  return <nav ref={dock} className={`bar dock-${desktop?.dock_style ?? "floating"}${dropping ? " drop-active" : ""}`} aria-label="Application panel"
    onDragOver={event => over(event, "end")} onDrop={event => void drop(event)} onDragEnd={() => { setDropping(null); setDragOrder(null); dragging.current = null; }}
    onDragLeave={event => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDropping(null); }}
    onKeyDown={event => { if (event.key === "Escape") { closePreview(true); setStatus(""); event.stopPropagation(); } }}>
    {overflow && <SurfaceButton className="dock-scroll" aria-label="Previous applications" disabled={atStart} onClick={() => scrollDock(-1)}><Icon name="back" /></SurfaceButton>}
    <ul ref={list} className="windows dock-items" aria-label="Pinned and running applications" onKeyDown={event => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
      const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>("[data-dock-key]"));
      if (!buttons.length) return;
      const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (index + (event.key === "ArrowLeft" ? -1 : 1) + buttons.length) % buttons.length;
      event.preventDefault(); buttons[next]?.focus();
    }}>
      {ordered.map((item, index) => {
        const running = windowsForItem(item), active = running.some(w => w.active);
        const near = hovered !== null && Math.abs(ordered.findIndex(candidate => candidate.key === hovered) - index) === 1;
        return <li key={item.key} data-item-key={item.key} className={`${index > 0 && !item.special && !pins.includes(item.key) && (ordered[index - 1]?.special || pins.includes(ordered[index - 1]?.key ?? "")) ? "dock-running-start " : ""}${dragging.current === item.key ? "dock-dragging " : ""}${dropping === item.key ? "dock-drop-target " : ""}${hovered === item.key ? "dock-hovered" : near ? "dock-neighbor" : ""}`}>
          <SurfaceButton id={item.special} data-dock-key={item.key} className={item.special ? "bar-button" : `window${running.length ? " running" : ""}${pending === item.key ? " pending" : ""}`}
            aria-label={item.name} aria-description={running.length > 1 ? `${running.length} windows. Click to switch to the next window.` : undefined} aria-pressed={item.special ? undefined : active} aria-haspopup={item.special === "applications" ? "dialog" : undefined}
            aria-expanded={item.special === "applications" ? expanded : undefined}
            onMouseEnter={event => showPreview(item, event.currentTarget)} onMouseLeave={hidePreview}
            onFocus={event => { event.currentTarget.scrollIntoView({block:"nearest",inline:"nearest"}); showPreview(item, event.currentTarget); }} onBlur={event => { if (!preview.current?.contains(event.relatedTarget as Node | null)) hidePreview(); }}
            onKeyDown={event => { if (event.key === "ArrowUp" && running.length) { event.preventDefault(); previewTrigger.current = event.currentTarget; clearTimeout(timer.current); const bounds = event.currentTarget.getBoundingClientRect(); setPreviewX(bounds.left + bounds.width / 2); setPreviewKey(item.key); requestAnimationFrame(() => preview.current?.querySelector<HTMLButtonElement>("button")?.focus()); } }}
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
          </SurfaceButton>
        </li>;
      })}
    </ul>
    {overflow && <SurfaceButton className="dock-scroll" aria-label="More applications" disabled={atEnd} onClick={() => scrollDock(1)}><Icon name="forward" /></SurfaceButton>}
    {previewItem && <div ref={preview} className={`dock-preview${previewWindows.length ? " has-windows" : ""}`} role="group" aria-label={`${previewItem.name} windows`} style={{ left: `clamp(8px, ${previewX - 125}px, max(8px, calc(100vw - 258px)))` }}
      onMouseEnter={() => clearTimeout(timer.current)} onMouseLeave={hidePreview} onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) hidePreview(); }}
      onKeyDown={event => { if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return; const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>("button")); if (!buttons.length) return; event.preventDefault(); const index = buttons.indexOf(document.activeElement as HTMLButtonElement); buttons[event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (index + (event.key === "ArrowUp" ? -1 : 1) + buttons.length) % buttons.length]?.focus(); }}>
      <div className="dock-preview-title">{previewItem.name}</div>
      {previewWindows.map(window => <div key={window.id} className="dock-preview-row" data-active={window.active}><SurfaceButton className="dock-preview-window" title={window.title || window.name} onClick={() => { setPreviewKey(null); void request({ method: "windows.activate", params: { id: window.id } }).catch(fail); }}><AppIcon id={previewItem.app?.id} name={previewItem.name} icon={previewItem.app?.icon} /><span>{window.title || window.name}</span>{window.active && <span className="dock-preview-active" aria-label="Active window">●</span>}</SurfaceButton><SurfaceButton className="dock-preview-close" aria-label={`Close ${window.title || window.name}`} disabled={closingIds.includes(window.id)} onClick={() => void closeWindow(window.id)}><Icon name="close" /></SurfaceButton></div>)}
    </div>}
    {status && <SurfaceButton className="dock-error" role="status" onClick={() => setStatus("")}>{status}</SurfaceButton>}
  </nav>;
}
