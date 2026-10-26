import { useEffect, useRef, useState } from "react";
import { Icon } from "../../components/Icon";
import { AppIcon } from "../../components/AppIcon";
import { APP_DRAG } from "../../components/Dock";
import { hello, request } from "../../lib/bridge";
import { useDesktop } from "../../lib/desktop";
import { errorMessage, mount, useMenuFocus } from "../../lib/react";
import type { DesktopItem } from "../../lib/generated/DesktopItem";
function Wallpaper() {
  const { desktop, setDesktop } = useDesktop();
  const contextMenu = useRef<HTMLDivElement>(null), removing = useRef(false);
  const [dropping, setDropping] = useState<"app" | "file" | null>(null);
  const [context, setContext] = useState<{ x: number; y: number; item?: DesktopItem } | null>(null);
  const [selected, setSelected] = useState<string | null>(null), [notice, setNotice] = useState("");
  const [sortBy, setSortBy] = useState<"manual" | "name" | "kind">(() => {
    try { const value = localStorage.getItem("meridian.desktop-sort"); return value === "name" || value === "kind" ? value : "manual"; } catch { return "manual"; }
  });
  const report = (e: unknown) => setNotice(errorMessage(e));
  useEffect(() => { void hello().catch(report); }, []);
  useMenuFocus(contextMenu, !!context, () => setContext(null));
  async function open(item: DesktopItem) { setContext(null); try { await request({ method: "desktop.open", params: { id: item.id } }); } catch (e) { report(e); } }
  async function addFiles() { setContext(null); try { setDesktop(await request({ method: "desktop.choose_files" })); } catch (e) { report(e); } }
  function chooseSort(value: "manual" | "name" | "kind") {
    setSortBy(value);
    try { localStorage.setItem("meridian.desktop-sort", value); } catch {}
    setContext(null);
  }
  async function addDroppedFiles(paths: string[]) {
    for (const path of [...new Set(paths)]) setDesktop(await request({ method: "desktop.add_file", params: { path } }));
  }
  const items = [...(desktop?.items ?? [])];
  if (sortBy === "name") items.sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" }));
  if (sortBy === "kind") items.sort((a, b) => ({ folder: 0, app: 1, file: 2 }[a.kind as "folder" | "app" | "file"] ?? 3) - ({ folder: 0, app: 1, file: 2 }[b.kind as "folder" | "app" | "file"] ?? 3) || a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" }));
  return <main className={`desktop${dropping ? " drop-active" : ""}`} tabIndex={0} onClick={() => { setContext(null); setSelected(null); }} onKeyDown={e => {
    if ((e.target as HTMLElement).closest?.(".desktop-context")) return;
    if (e.key === "Escape") { setContext(null); setSelected(null); }
    if (["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key) && items.length) {
      e.preventDefault(); const index = items.findIndex(item => item.id === selected);
      const rows = Math.max(1, Math.floor((document.querySelector(".desktop-items")?.clientHeight ?? 500) / 116));
      const offset = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : e.key === "ArrowRight" ? rows : -rows;
      const next = e.key === "Home" || index < 0 && e.key !== "End" ? 0 : e.key === "End" ? items.length - 1 : Math.max(0, Math.min(items.length - 1, index + offset));
      setSelected(items[next]!.id); document.querySelectorAll<HTMLButtonElement>(".desktop-item")[next]?.focus(); return;
    }
    const item = desktop?.items.find(i => i.id === selected);
    if (e.key === "ContextMenu" || e.shiftKey && e.key === "F10") {
      e.preventDefault(); const bounds = (document.activeElement as HTMLElement | null)?.getBoundingClientRect();
      setContext({ x: item && bounds ? bounds.left : 32, y: item && bounds ? bounds.bottom : 32, item }); return;
    }
    if (e.key === "Delete" && item && !e.repeat && !removing.current) {
      e.preventDefault(); removing.current = true;
      void request({ method: "desktop.remove", params: { id: item.id } }).then(state => { setDesktop(state); setSelected(null); }).catch(report).finally(() => { removing.current = false; }); return;
    }
    if (e.key === "Enter" && item) { e.preventDefault(); void open(item); }
  }} onContextMenu={e => { e.preventDefault(); setContext({ x: e.clientX, y: e.clientY }); }} onDragOver={e => { if ([APP_DRAG, "text/uri-list", "Files"].some(type => e.dataTransfer.types.includes(type))) { e.preventDefault(); setDropping(e.dataTransfer.types.includes(APP_DRAG) ? "app" : "file"); e.dataTransfer.dropEffect = e.dataTransfer.types.includes(APP_DRAG) ? "copy" : "move"; } }} onDragLeave={e => { if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setDropping(null); }} onDragEnd={() => setDropping(null)} onDrop={async e => {
    e.preventDefault(); setDropping(null); const id = e.dataTransfer.getData(APP_DRAG);
    if (id) { void request({ method: "desktop.add_app", params: { id } }).then(setDesktop).catch(report); return; }
    const paths: string[] = [];
    const uriList = e.dataTransfer.getData("text/uri-list") || e.dataTransfer.getData("text/plain");
    for (const line of uriList.split(/\r?\n/).filter(line => line && !line.startsWith("#"))) {
      try { const uri = new URL(line); if (uri.protocol === "file:" && (!uri.hostname || uri.hostname === "localhost")) paths.push(decodeURIComponent(uri.pathname)); } catch {}
    }
    for (const file of Array.from(e.dataTransfer.files ?? [])) { const path = (file as File & { path?: string }).path; if (path) paths.push(path); }
    if (paths.length) try { await addDroppedFiles(paths); } catch (error) { report(error); }
  }}>
    <div className="wallpaper" aria-hidden="true" style={desktop ? { backgroundImage: `url("${desktop.wallpaper_uri}")` } : undefined} />
    <section className="desktop-items" aria-label="Desktop items">{items.map(item => <button key={item.id} aria-label={item.name} aria-pressed={selected === item.id} title={item.name} draggable onDragStart={event => { if (item.kind === "app") event.dataTransfer.setData(APP_DRAG, item.target); else event.dataTransfer.setData("text/uri-list", `file://${item.target.split("/").map(encodeURIComponent).join("/")}`); event.dataTransfer.effectAllowed = "copyMove"; }} className={`desktop-item${selected === item.id ? " selected" : ""}`} onClick={e => { e.stopPropagation(); setSelected(item.id); setContext(null); }} onDoubleClick={() => void open(item)} onContextMenu={e => { e.preventDefault(); e.stopPropagation(); setSelected(item.id); setContext({ x: e.clientX, y: e.clientY, item }); }}>{item.kind === "app" ? <AppIcon id={item.target} name={item.name} icon={item.icon} /> : <svg viewBox="0 0 64 64" className="desktop-file" aria-hidden="true">{item.kind === "folder" ? <><path d="M4 14h23l6 6h27v36H4Z" fill="#6ab7ec" /><path d="M4 26h56v30H4Z" fill="#83c8f3" /></> : <><path d="M15 5h24l12 12v43H15Z" fill="#f9fafc" stroke="#b8c5d5" /><path d="M39 5v13h12M23 31h21M23 39h21M23 47h15" fill="none" stroke="#a7b6ca" strokeWidth="2" /></>}</svg>}<span>{item.name}</span></button>)}</section>
    {context && <div ref={contextMenu} className="desktop-context" role="menu" aria-label="Desktop actions" style={{ left: Math.max(8, Math.min(context.x, window.innerWidth - 248)), top: Math.max(8, Math.min(context.y, window.innerHeight - 420)) }} onClick={e => e.stopPropagation()}>{context.item ? <><button role="menuitem" onClick={() => void open(context.item!)}>Open</button><button role="menuitem" onClick={() => { void request({ method: "desktop.remove", params: { id: context.item!.id } }).then(setDesktop).catch(report); setContext(null); }}>{context.item.kind === "app" || !context.item.id.startsWith("file:") ? "Remove shortcut" : "Move to Trash"}</button><hr /></> : null}<strong className="context-heading">Sort by</strong><button role="menuitemradio" aria-checked={sortBy === "name"} onClick={() => chooseSort("name")}>Name</button><button role="menuitemradio" aria-checked={sortBy === "kind"} onClick={() => chooseSort("kind")}>Kind</button><button role="menuitemradio" aria-checked={sortBy === "manual"} onClick={() => chooseSort("manual")}>Manual</button><hr /><button role="menuitem" onClick={() => void addFiles()}>Move files here…</button><button role="menuitem" onClick={() => { void request({ method: "popover.toggle", params: { popover: "applications" } }).catch(report); setContext(null); }}>Applications…</button><button role="menuitem" onClick={() => { void request({ method: "finder.open" }).catch(report); setContext(null); }}>Open File Manager</button><hr /><button role="menuitem" onClick={() => { void request({ method: "preferences.wallpaper_open" }).catch(report); setContext(null); }}>Change wallpaper…</button></div>}
    {dropping && <div className="desktop-drop-indicator" aria-hidden="true"><span><Icon name={dropping === "app" ? "pin" : "folder"} />{dropping === "app" ? "Add app shortcut to desktop" : "Move to Desktop"}</span></div>}

    {notice && <div className="desktop-notice" role="alert">{notice}<button aria-label="Dismiss message" onClick={e => { e.stopPropagation(); setNotice(""); }}>×</button></div>}
  </main>;
}
mount(<Wallpaper />);
