import { Button, SearchField } from "../../meridian-ui";
import { SurfaceButton } from "../../meridian-ui";
import { useDesktop } from "../../lib/desktop";
import type { WindowInfo } from "../../lib/generated/WindowInfo";
import { Fragment, useEffect, useRef, useState } from "react";
import { UninstallDialog } from "../../components/UninstallDialog";
import { isBuiltinApp } from "../../lib/apps";
import { Icon } from "../../components/Icon";
import { AppIcon } from "../../components/AppIcon";
import { APP_DRAG } from "../../components/Dock";
import { on, request } from "../../lib/bridge";
import { errorMessage, mount, useMenuFocus, useCallbackRef, usePopover, useWindowKey } from "../../lib/react";
import type { ProviderId } from "../../lib/generated/ProviderId";
import type { SearchItem } from "../../lib/generated/SearchItem";

type Result = { provider: ProviderId | "desktop-files"; item: SearchItem };

function Applications() {
  const [uninstall, setUninstall] = useState<string | null>(null);
  const contextMenu = useRef<HTMLDivElement>(null);
  const [context, setContext] = useState<{ id: string; x: number; y: number } | null>(null);
  useMenuFocus(contextMenu, !!context, () => setContext(null));
  useEffect(() => on("apps.uninstall_requested", e => setUninstall(e.data.id)), []);
  const [dragging, setDragging] = useState(false);
  const [query, setQuery] = useState("");
  const [allResults, setResults] = useState<Result[]>([]);
  const { desktop } = useDesktop();
  const [pins, setPins] = useState<string[]>([]), [windows, setWindows] = useState<WindowInfo[]>([]);
  const group = (result: Result) => result.provider === "apps" ? (!query.trim() && pins.includes(result.item.id) ? "Pinned apps" : "Apps") : result.provider === "settings" ? "Settings" : result.provider === "desktop-files" ? "Desktop files" : "Calculator";
  const groups = ["Pinned apps", "Apps", "Settings", "Desktop files", "Calculator"];
  const files: Result[] = query.trim() ? (desktop?.items ?? []).filter(item => item.kind !== "app" && item.name.toLowerCase().includes(query.trim().toLowerCase())).map(item => ({provider:"desktop-files", item:{id:item.id, title:item.name, subtitle:"Desktop", icon:item.icon}})) : [];
  const results = [...allResults, ...files].sort((a,b) => groups.indexOf(group(a)) - groups.indexOf(group(b)) || (group(a) === "Pinned apps" ? pins.indexOf(a.item.id) - pins.indexOf(b.item.id) : 0));
  const running = (id: string) => windows.filter(window => window.app_id === id);
  useEffect(() => {
    let alive = true, pr = 0, wr = 0;
    const offPins = on("dock.changed", e => { pr++; setPins(e.data); });
    const offWindows = on("windows.changed", e => { wr++; setWindows(e.data); });
    void request({method:"dock.list"}).then(value => { if (alive && !pr) setPins(value); }).catch(report);
    void request({method:"windows.list"}).then(value => { if (alive && !wr) setWindows(value); }).catch(report);
    return () => { alive = false; offPins(); offWindows(); };
  }, []);
  const [searching, setSearching] = useState(false);
  const [selected, setSelected] = useState(-1);
  const [status, setStatus] = useState("");
  const [launching, setLaunching] = useState<string | null>(null);
  const serial = useRef(0);
  const alive = useRef(true);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const report = (error: unknown) => setStatus(errorMessage(error, "Couldn’t search applications"));
  const runQuery = useCallbackRef(async (value: string) => {
    const mine = ++serial.current;
    setSearching(true);
    setResults([]);
    setSelected(-1);
    try {
      const response = await request({ method: "search.query", params: { query: value, serial: mine } });
      if (!alive.current || response.serial !== serial.current) return;
      // Meridian Files replaces Nautilus in the launcher without removing the
      // system package or hiding it from app-management/default-app settings.
      const hiddenSystemFiles = new Set(["org.gnome.Nautilus.desktop", "org.gnome.nautilus.desktop", "nautilus.desktop"]);
      const next = response.sections.flatMap(section => section.items
        .filter(item => section.provider !== "apps" || !hiddenSystemFiles.has(item.id))
        .map(item => ({ provider: section.provider, item })));
      setResults(next);
      setSelected(value.trim() ? 0 : -1);
      if (!value.trim()) scroller.current?.scrollTo({ top: 0, behavior: "instant" });
    } finally { if (alive.current && mine === serial.current) setSearching(false); }
  });
  const { visible, menu, dismiss } = usePopover(async () => {
    setStatus(""); setQuery(""); setLaunching(null); setContext(null); setUninstall(null);
    await runQuery("");
  }, () => input.current?.focus(), report);
  const refresh = useCallbackRef(() => { if (visible) void runQuery(query).catch(report); });
  useEffect(() => {
    alive.current = true;
    const off = on("search.invalidated", refresh);
    return () => { alive.current = false; serial.current++; off(); };
  }, [refresh]);
  useEffect(() => { if (selected >= results.length) setSelected(-1); }, [results.length, selected]);
  useEffect(() => {
    if (selected >= 0) list.current?.querySelectorAll<HTMLElement>('[role="option"]')[selected]?.scrollIntoView({ block: "nearest", behavior: window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth" });
  }, [selected]);
  function changeQuery(value: string) { setQuery(value); void runQuery(value).catch(report); }
  function move(delta: number) {
    if (results.length) setSelected(index => index < 0 ? 0 : Math.min(results.length - 1, Math.max(0, index + delta)));
  }
  async function activate(index: number) {
    const result = results[index];
    if (!result || searching || launching) return;
    setLaunching(`${result.provider}:${result.item.id}`);
    try {
      const openWindows = result.provider === "apps" ? running(result.item.id) : [];
      if (result.provider === "desktop-files") await request({method:"desktop.open",params:{id:result.item.id}});
      else if (openWindows.length) await request({method:"windows.activate",params:{id:(openWindows.find(w => w.active) ?? openWindows[0]!).id}});
      else await request({ method: "search.activate", params: { provider: result.provider, item_id: result.item.id } });
      await dismiss();
    } catch (error) {
      console.error(error);
      setStatus(`Couldn’t open ${result.item.title}`);
    } finally { setLaunching(null); }
  }
  useWindowKey(event => {
    if (event.isComposing || uninstall) return;
    if (event.target instanceof HTMLButtonElement) {
      if (event.key === "Escape") { event.preventDefault(); if (context) setContext(null); else void dismiss().catch(report); }
      return;
    }
    if (context) { if (event.key === "Escape") { setContext(null); event.preventDefault(); } return; }
    if ((event.key === "ContextMenu" || (event.shiftKey && event.key === "F10")) && selected >= 0) {
      event.preventDefault();
      if (results[selected]?.provider === "apps") {
        const bounds = list.current?.querySelectorAll<HTMLElement>('[role="option"]')[selected]?.getBoundingClientRect();
        if (bounds) setContext({ id: results[selected]!.item.id, x: bounds.left, y: bounds.bottom });
      }
      return;
    }
    const options = Array.from(list.current?.querySelectorAll<HTMLElement>('[role="option"]') ?? []);
    const row = options[selected] ?? options[0];
    const columns = list.current ? getComputedStyle(list.current).gridTemplateColumns.split(" ").length : 1;
    const page = row?.offsetHeight ? Math.max(1, Math.floor((scroller.current?.clientHeight ?? 0) / (row.offsetHeight + 12)) - 1) * columns : columns;
    function moveVertical(direction: number) {
      if (selected < 0) { move(1); return; }
      const current = options[selected]?.getBoundingClientRect();
      if (!current) return;
      const candidates = options.map((option, index) => ({ index, rect: option.getBoundingClientRect() }))
        .filter(({ rect }) => direction > 0 ? rect.top > current.top + 1 : rect.top < current.top - 1)
        .sort((a, b) => Math.abs(a.rect.top - current.top) - Math.abs(b.rect.top - current.top)
          || Math.abs(a.rect.left - current.left) - Math.abs(b.rect.left - current.left));
      if (candidates[0]) setSelected(candidates[0].index);
    }
    switch (event.key) {
      case "ArrowDown": moveVertical(1); break;
      case "ArrowRight": if (selected < 0) return; move(1); break;
      case "ArrowLeft": if (selected < 0) return; move(-1); break;
      case "ArrowUp": moveVertical(-1); break;
      case "Tab": return;
      case "PageDown": move(page); break;
      case "PageUp": move(-page); break;
      case "Enter": if (selected >= 0) void activate(selected); break;
      case "Escape": void dismiss().catch(report); break;
      default:
        if (event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey) input.current?.focus();
        return;
    }
    event.preventDefault();
  });
  return <>
    <div hidden={!visible} className={`scrim${dragging ? " dragging" : ""}`} id="scrim" onDragOver={event => { if (event.dataTransfer.types.includes(APP_DRAG)) event.preventDefault(); }} onDrop={event => { const id = event.dataTransfer.getData(APP_DRAG); if (!id) return; event.preventDefault(); setDragging(false); void request({ method: "desktop.add_app", params: { id } }).then(() => dismiss()).catch(report); }} onClick={() => void dismiss().catch(report)} />
    <section ref={menu} inert={!visible} aria-hidden={!visible} className={`menu app-launcher${dragging ? " dragging" : ""}${visible ? "" : " hidden"}`} id="menu" role="dialog" aria-label="Application launcher">
      <header className="apps-toolbar">
      <h1>Applications</h1>
      <SearchField className="apps-search" ref={input} id="search" placeholder="Search apps, settings and files…" autoComplete="off"
        spellCheck={false} role="combobox" aria-expanded={visible} aria-label="Search applications" aria-controls="list" aria-autocomplete="list"
        aria-activedescendant={selected >= 0 && selected < results.length ? `row-${selected}` : undefined}
        value={query} onValueChange={changeQuery} />
      <SurfaceButton className="close-applications" aria-label="Close applications" onClick={() => void dismiss().catch(report)}><Icon name="close" width="18" height="18" /></SurfaceButton>
      </header>
      <div ref={scroller} className="scroller" id="scroller">
        <ul ref={list} className="list app-grid" id="list" role="listbox" aria-label="Applications" aria-busy={searching}>
          {results.map((result, index) => {
            const key = `${result.provider}:${result.item.id}`;
            const openWindows = result.provider === "apps" ? running(result.item.id) : [];
            return <Fragment key={key}>{(index === 0 || group(results[index - 1]!) !== group(result)) && <li role="presentation" className="launcher-group">{group(result)}</li>}<li id={`row-${index}`} role="option" aria-selected={selected === index}
              onContextMenu={event => { event.preventDefault(); if (result.provider === "apps") setContext({ id: result.item.id, x: event.clientX, y: event.clientY }); }}
              draggable={result.provider === "apps"} onDragEnd={() => setDragging(false)} onDragStart={event => { setDragging(true); event.dataTransfer.setData(APP_DRAG, result.item.id); event.dataTransfer.effectAllowed = "copyMove"; }}
              tabIndex={-1} title={result.item.title}
              className={`row${launching === key ? " launching" : ""}`} onClick={() => { setSelected(index); void activate(index); }}>
              <AppIcon id={result.provider === "apps" ? result.item.id : undefined} name={result.item.title} icon={result.item.icon} />
              <span className="app-label"><span className="name">{result.item.title}</span><span className="app-description" hidden={result.provider === "apps" && !openWindows.length}>{openWindows.length ? `● Running${openWindows.length > 1 ? ` (${openWindows.length})` : ""}` : result.item.subtitle}</span></span>

            </li></Fragment>;
          })}
        </ul>
        {!searching && !results.length && <div className="empty" id="empty"><span className="empty-symbol" aria-hidden="true">⌕</span><strong>{query ? "No results found" : "No apps available"}</strong><span>{query ? `Try a different search for “${query}”.` : "Installed apps will appear here."}</span><Button onClick={()=>changeQuery("")}>Clear search</Button><Button onClick={()=>void request({method:"preferences.page_open",params:{page:"applications"}}).catch(report)}>Manage applications</Button></div>}
        {searching && !results.length && <div className="empty loading" role="status"><span className="search-spinner" /><span>Finding applications…</span></div>}
      </div>
      <p className="status" data-kind={status.startsWith("Added") ? "success" : "error"} id="status" role="status" aria-live="polite">{status}</p>
    </section>
    {context && <div ref={contextMenu} className="app-context" role="menu" aria-label="Application actions" style={{ left: Math.max(8, Math.min(context.x, window.innerWidth - 248)), top: Math.max(8, Math.min(context.y, window.innerHeight - 428)) }}>
      {running(context.id).map(window => <SurfaceButton key={window.id} role="menuitem" onClick={() => { setContext(null); void request({method:"windows.activate",params:{id:window.id}}).then(dismiss).catch(report); }}>Switch to {window.title || window.name}</SurfaceButton>)}
      <SurfaceButton role="menuitem" onClick={() => { const id = context.id; setContext(null); void request({method:"search.activate",params:{provider:"apps",item_id:id}}).then(dismiss).catch(report); }}>Open new window (if supported)</SurfaceButton>
      <SurfaceButton role="menuitem" onClick={() => { void request({ method: "dock.pin", params: { id: context.id, pinned: !pins.includes(context.id) } }).then(() => setStatus("Updated dock pins")).catch(report); setContext(null); }}>{pins.includes(context.id) ? "Unpin from Dock" : "Pin to Dock"}</SurfaceButton>
      <SurfaceButton role="menuitem" onClick={() => { void request({ method: "desktop.add_app", params: { id: context.id } }).then(() => setStatus("Added to desktop")).catch(report); setContext(null); }}>Add to Desktop</SurfaceButton>
      <SurfaceButton role="menuitem" disabled={isBuiltinApp(context.id)} onClick={() => { setUninstall(context.id); setContext(null); }}>Uninstall…</SurfaceButton>
      <SurfaceButton role="menuitem" onClick={() => setContext(null)}>Cancel</SurfaceButton>
    </div>}
    {uninstall && <UninstallDialog id={uninstall} close={() => { setUninstall(null); refresh(); }} />}
  </>;
}

mount(<Applications />);
