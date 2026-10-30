import { useEffect, useRef, useState } from "react";
import { UninstallDialog } from "../../components/UninstallDialog";
import { isBuiltinApp } from "../../lib/apps";
import { Icon } from "../../components/Icon";
import { AppIcon } from "../../components/AppIcon";
import { APP_DRAG } from "../../components/Dock";
import { on, request } from "../../lib/bridge";
import { errorMessage, mount, useMenuFocus, useCallbackRef, usePopover, useWindowKey } from "../../lib/react";
import type { ProviderId } from "../../lib/generated/ProviderId";
import type { SearchItem } from "../../lib/generated/SearchItem";

type Result = { provider: ProviderId; item: SearchItem };

function Applications() {
  const [uninstall, setUninstall] = useState<string | null>(null);
  const contextMenu = useRef<HTMLDivElement>(null);
  const [context, setContext] = useState<{ id: string; x: number; y: number } | null>(null);
  useMenuFocus(contextMenu, !!context, () => setContext(null));
  useEffect(() => on("apps.uninstall_requested", e => setUninstall(e.data.id)), []);
  const [dragging, setDragging] = useState(false);
  const [query, setQuery] = useState("");
  const [allResults, setResults] = useState<Result[]>([]);
  const results = allResults;
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
      setSelected(value.trim() && next.length ? 0 : -1);
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
    if (selected >= 0) list.current?.children[selected]?.scrollIntoView({ block: "nearest", behavior: window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth" });
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
      await request({ method: "search.activate", params: { provider: result.provider, item_id: result.item.id } });
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
        const bounds = list.current?.children[selected]?.getBoundingClientRect();
        if (bounds) setContext({ id: results[selected]!.item.id, x: bounds.left, y: bounds.bottom });
      }
      return;
    }
    const row = list.current?.children[0] as HTMLElement | undefined;
    const page = row?.offsetHeight ? Math.max(1, Math.floor((scroller.current?.clientHeight ?? 0) / row.offsetHeight) - 1) : 1;
    switch (event.key) {
      case "ArrowDown": move(1); break;
      case "ArrowRight": move(1); break;
      case "ArrowLeft": move(-1); break;
      case "ArrowUp": move(-1); break;
      case "Tab": return;
      case "PageDown": move(page); break;
      case "PageUp": move(-page); break;
      case "Enter": if (selected >= 0) void activate(selected); break;
      case "Escape": if (query) changeQuery(""); else void dismiss().catch(report); break;
      default:
        if (event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey) input.current?.focus();
        return;
    }
    event.preventDefault();
  });
  return <>
    <div hidden={!visible} className={`scrim${dragging ? " dragging" : ""}`} id="scrim" onDragOver={event => { if (event.dataTransfer.types.includes(APP_DRAG)) event.preventDefault(); }} onDrop={event => { const id = event.dataTransfer.getData(APP_DRAG); if (!id) return; event.preventDefault(); setDragging(false); void request({ method: "desktop.add_app", params: { id } }).then(() => dismiss()).catch(report); }} onClick={() => void dismiss().catch(report)} />
    <section ref={menu} inert={!visible} aria-hidden={!visible} className={`menu${dragging ? " dragging" : ""}${visible ? "" : " hidden"}`} id="menu" role="dialog" aria-label="Application launcher">
      <div className="apps-toolbar">
      <label className="apps-search"><svg viewBox="0 0 20 20" aria-hidden="true"><circle cx="8.5" cy="8.5" r="5.75"/><path d="m13 13 4 4"/></svg><input ref={input} id="search" className="search" type="text" placeholder="Search…" autoComplete="off"
        spellCheck={false} aria-label="Search applications" aria-controls="list" aria-autocomplete="list"
        aria-activedescendant={selected >= 0 ? `row-${selected}` : undefined}
        value={query} onChange={event => changeQuery(event.target.value)} />{query && <button className="clear-search" type="button" aria-label="Clear search" onClick={() => { changeQuery(""); input.current?.focus(); }}>×</button>}</label>
      <button className="close-applications" aria-label="Close applications" onClick={() => void dismiss().catch(report)}><Icon name="close" width="18" height="18" /></button>
      </div>
      <div ref={scroller} className="scroller" id="scroller">
        <ul ref={list} className="list list-view" id="list" role="listbox" aria-label="Applications">
          {results.map((result, index) => {
            const key = `${result.provider}:${result.item.id}`;
            return <li key={key} id={`row-${index}`} role="option" aria-selected={selected === index}
              onContextMenu={event => { event.preventDefault(); if (result.provider === "apps") setContext({ id: result.item.id, x: event.clientX, y: event.clientY }); }}
              draggable={result.provider === "apps"} onDragEnd={() => setDragging(false)} onDragStart={event => { setDragging(true); event.dataTransfer.setData(APP_DRAG, result.item.id); event.dataTransfer.effectAllowed = "copyMove"; }}
              tabIndex={-1} title={result.item.title}
              className={`row${launching === key ? " launching" : ""}`} onClick={() => { setSelected(index); void activate(index); }}>
              <AppIcon id={result.provider === "apps" ? result.item.id : undefined} name={result.item.title} icon={result.item.icon} />
              <span className="app-label"><span className="name">{result.item.title}</span></span>

            </li>;
          })}
        </ul>
        {!searching && !results.length && <div className="empty" id="empty"><span className="empty-symbol" aria-hidden="true">⌕</span><strong>{query ? "No results found" : "No apps available"}</strong><span>{query ? `Try a different search for “${query}”.` : "Installed apps will appear here."}</span></div>}
        {searching && !results.length && <div className="empty loading" role="status"><span className="search-spinner" /><span>Finding applications…</span></div>}
      </div>
      <p className="status" data-kind={status.startsWith("Added") ? "success" : "error"} id="status" role="status" aria-live="polite">{status}</p>
    </section>
    {context && <div ref={contextMenu} className="app-context" role="menu" aria-label="Application actions" style={{ left: Math.max(8, Math.min(context.x, window.innerWidth - 188)), top: Math.max(8, Math.min(context.y, window.innerHeight - 170)) }}>
      <button role="menuitem" onClick={() => { void request({ method: "dock.pin", params: { id: context.id, pinned: true } }).then(() => setStatus("Added to dock")).catch(report); setContext(null); }}>Add to Dock</button>
      <button role="menuitem" onClick={() => { void request({ method: "desktop.add_app", params: { id: context.id } }).then(() => setStatus("Added to desktop")).catch(report); setContext(null); }}>Add to Desktop</button>
      <button role="menuitem" disabled={isBuiltinApp(context.id)} onClick={() => { setUninstall(context.id); setContext(null); }}>Uninstall…</button>
      <button role="menuitem" onClick={() => setContext(null)}>Cancel</button>
    </div>}
    {uninstall && <UninstallDialog id={uninstall} close={() => { setUninstall(null); refresh(); }} />}
  </>;
}

mount(<Applications />);
