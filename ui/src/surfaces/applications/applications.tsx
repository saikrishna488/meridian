import { useEffect, useRef, useState } from "react";
import { UninstallDialog } from "../../components/UninstallDialog";
import { isBuiltinApp } from "../../lib/apps";
import { AppIcon } from "../../components/AppIcon";
import { Dock, APP_DRAG } from "../../components/Dock";
import { on, request } from "../../lib/bridge";
import { errorMessage, mount, useCallbackRef, usePopover, useWindowKey } from "../../lib/react";
import type { ProviderId } from "../../lib/generated/ProviderId";
import type { SearchItem } from "../../lib/generated/SearchItem";

type Result = { provider: ProviderId; item: SearchItem };

function Applications() {
  const [uninstall, setUninstall] = useState<string | null>(null);
  const [context, setContext] = useState<{ id: string; x: number; y: number } | null>(null);
  useEffect(() => on("apps.uninstall_requested", e => setUninstall(e.data.id)), []);
  const [dragging, setDragging] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Result[]>([]);
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
    try {
      const response = await request({ method: "search.query", params: { query: value, serial: mine } });
      if (!alive.current || response.serial !== serial.current) return;
      const next = response.sections.flatMap(section => section.items.map(item => ({ provider: section.provider, item })));
      setResults(next);
      setSelected(value.trim() && next.length ? 0 : -1);
      if (!value.trim()) scroller.current?.scrollTo({ top: 0, behavior: "instant" });
    } finally { if (alive.current && mine === serial.current) setSearching(false); }
  });
  const { visible, menu, dismiss } = usePopover(async () => {
    setStatus(""); setQuery(""); setLaunching(null); setContext(null); setUninstall(null);
    await runQuery("");
  }, () => input.current?.focus(), report);
  const refresh = useCallbackRef(() => { void runQuery(query).catch(report); });
  useEffect(() => {
    alive.current = true;
    const off = on("search.invalidated", refresh);
    return () => { alive.current = false; serial.current++; off(); };
  }, [refresh]);
  useEffect(() => {
    if (selected >= 0) list.current?.children[selected]?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [selected]);
  function changeQuery(value: string) { setQuery(value); void runQuery(value).catch(report); }
  function move(delta: number) {
    if (results.length) setSelected(index => index < 0 ? 0 : Math.min(results.length - 1, Math.max(0, index + delta)));
  }
  async function activate(index: number) {
    const result = results[index];
    if (!result || launching) return;
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
    if (context) { if (event.key === "Escape") { setContext(null); event.preventDefault(); } return; }
    const row = list.current?.children[0] as HTMLElement | undefined;
    const page = row ? Math.max(1, Math.floor((scroller.current?.clientHeight ?? 0) / row.offsetHeight) - 1) : 1;
    switch (event.key) {
      case "ArrowDown": move(Math.max(1, Math.floor((list.current?.clientWidth ?? 600) / 112))); break;
      case "ArrowRight": move(1); break;
      case "ArrowLeft": move(-1); break;
      case "ArrowUp": move(-Math.max(1, Math.floor((list.current?.clientWidth ?? 600) / 112))); break;
      case "Tab": move(event.shiftKey ? -1 : 1); break;
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
    <div className={`scrim${dragging ? " dragging" : ""}`} id="scrim" onDragOver={event => { if (event.dataTransfer.types.includes(APP_DRAG)) event.preventDefault(); }} onDrop={event => { const id = event.dataTransfer.getData(APP_DRAG); if (!id) return; event.preventDefault(); setDragging(false); void request({ method: "desktop.add_app", params: { id } }).then(() => dismiss()).catch(report); }} onClick={() => void dismiss().catch(report)} />
    <section ref={menu} className={`menu${dragging ? " dragging" : ""}${visible ? "" : " hidden"}`} id="menu" role="dialog" aria-label="Applications"
      >
      <header className="apps-heading"><div><span className="apps-eyebrow">MERIDIAN</span><h1>Applications</h1><p>Everything you need, in one place.</p></div><button aria-label="Close applications" onClick={() => void dismiss().catch(report)}>×</button></header>
      <label className="apps-search"><svg viewBox="0 0 20 20" aria-hidden="true"><circle cx="8.5" cy="8.5" r="5.75"/><path d="m13 13 4 4"/></svg><input ref={input} id="search" className="search" type="text" placeholder="Search applications" autoComplete="off"
        spellCheck={false} aria-label="Search applications" aria-controls="list" aria-autocomplete="list"
        aria-activedescendant={selected >= 0 ? `row-${selected}` : undefined}
        value={query} onChange={event => changeQuery(event.target.value)} />{query && <button className="clear-search" type="button" aria-label="Clear search" onClick={() => { changeQuery(""); input.current?.focus(); }}>×</button>}</label>
      <div className="apps-section-heading"><h2>{query ? "Search Results" : "All Applications"}</h2><span>{searching ? "Searching…" : `${results.length} ${results.length === 1 ? "item" : "items"}`}</span></div>
      <div ref={scroller} className="scroller" id="scroller">
        <ul ref={list} className="list" id="list" role="listbox" aria-label="Applications">
          {results.map((result, index) => {
            const key = `${result.provider}:${result.item.id}`;
            return <li key={key} id={`row-${index}`} role="option" aria-selected={selected === index}
              onContextMenu={event => { event.preventDefault(); setContext({ id: result.item.id, x: event.clientX, y: event.clientY }); }}
              draggable onDragEnd={() => setDragging(false)} onDragStart={event => { setDragging(true); event.dataTransfer.setData(APP_DRAG, result.item.id); event.dataTransfer.effectAllowed = "copyMove"; }}
              tabIndex={-1} title={result.item.subtitle ?? result.item.title}
              className={`row${launching === key ? " launching" : ""}`} onClick={() => { setSelected(index); void activate(index); }}>
              <AppIcon id={result.item.id} name={result.item.title} icon={result.item.icon} />
              <span className="name">{result.item.title}</span>

            </li>;
          })}
        </ul>
        {!searching && !results.length && <div className="empty" id="empty"><span className="empty-symbol" aria-hidden="true">⌕</span><strong>{query ? "No results found" : "No applications available"}</strong><span>{query ? `Try a different search for “${query}”.` : "Applications will appear here when they are installed."}</span></div>}
        {searching && !results.length && <div className="empty loading" role="status"><span className="search-spinner" /><span>Finding applications…</span></div>}
      </div>
      <p className="status" id="status" role="status" aria-live="polite">{status}</p>
      <footer className="apps-footer"><span className="apps-hint">Drag an app to the dock to keep it there</span><kbd>Esc</kbd><span className="apps-hint">to clear or close</span></footer>
    </section>
    <div className="launcher-dock"><Dock /></div>
    {context && <div className="app-context" role="menu" aria-label="Application actions" style={{ left: Math.min(context.x, window.innerWidth - 180), top: Math.min(context.y, window.innerHeight - 100) }}>
      <button role="menuitem" autoFocus onClick={() => { void request({ method: "desktop.add_app", params: { id: context.id } }).then(() => setStatus("Added to desktop")).catch(report); setContext(null); }}>Add to Desktop</button>
      <button role="menuitem" disabled={isBuiltinApp(context.id)} onClick={() => { setUninstall(context.id); setContext(null); }}>Uninstall…</button>
      <button role="menuitem" onClick={() => setContext(null)}>Cancel</button>
    </div>}
    {uninstall && <UninstallDialog id={uninstall} close={() => { setUninstall(null); refresh(); }} />}
  </>;
}

mount(<Applications />);
