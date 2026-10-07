import { useEffect, useRef, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { errorMessage, mount } from "../../lib/react";
import { AppIcon } from "../../components/AppIcon";
import { UninstallDialog } from "../../components/UninstallDialog";
import { isBuiltinApp } from "../../lib/apps";
import type { FileEntry } from "../../lib/generated/FileEntry";
import type { FileListing } from "../../lib/generated/FileListing";
import type { FileLocation } from "../../lib/generated/FileLocation";
import type { SearchItem } from "../../lib/generated/SearchItem";

type Item = { id: string; name: string; file?: FileEntry; app?: SearchItem };
function FileIcon({ directory }: { directory: boolean }) {
  return <svg className="file-art" viewBox="0 0 72 64" aria-hidden="true">{directory ? <><path d="M6 14h24l6 6h30v38H6Z" fill="#6ab7ec" /><path d="M6 26h60v28a5 5 0 0 1-5 5H11a5 5 0 0 1-5-5Z" fill="#83c8f3" /><path d="M7 26h58" stroke="#bce5ff" /></> : <><path d="M16 5h28l13 13v42H16Z" fill="#fff" stroke="#c8cdd5" /><path d="M44 5v14h13M24 30h25M24 37h25M24 44h18" fill="none" stroke="#c8cdd5" /></>}</svg>;
}
function size(bytes: number) { return bytes < 1024 ? `${bytes} bytes` : bytes < 1048576 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / 1048576).toFixed(1)} MB`; }
function Finder() {
  const [locations, setLocations] = useState<FileLocation[]>([]);
  const [history, setHistory] = useState<string[]>([]), [position, setPosition] = useState(0);
  const [listing, setListing] = useState<FileListing | null>(null), [apps, setApps] = useState<SearchItem[]>([]);
  const [query, setQuery] = useState(""), [view, setView] = useState<"grid" | "list">("grid");
  const [hidden, setHidden] = useState(false), [selected, setSelected] = useState<string | null>(null);
  const [busy, setBusy] = useState(false), [notice, setNotice] = useState(""), [refresh, setRefresh] = useState(0);
  const [context, setContext] = useState<{ item: Item; x: number; y: number } | null>(null);
  const [dialog, setDialog] = useState<{ action: "mkdir" | "rename" | "trash"; item?: Item } | null>(null), [name, setName] = useState("");
  const [uninstall, setUninstall] = useState<string | null>(null), [address, setAddress] = useState("");
  const serial = useRef(0), search = useRef<HTMLInputElement>(null);
  const path = history[position] ?? "", applications = path === "@applications";
  const title = applications ? "Applications" : locations.find(l => l.path === path)?.name ?? path.split('/').filter(Boolean).at(-1) ?? "Computer";
  const report = (e: unknown) => setNotice(errorMessage(e));
  useEffect(() => {
    let alive = true;
    void hello().then(() => request({ method: "files.locations" })).then(result => { if (alive) { setLocations(result); setHistory([result[0]?.path ?? "/"]); } }).catch(report);
    const off = on("search.invalidated", () => setRefresh(n => n + 1));
    const offLocation = on("finder.location", e => { setHistory([e.data.path]); setPosition(0); setQuery(""); });
    return () => { alive = false; off(); offLocation(); };
  }, []);
  useEffect(() => {
    if (!path) return;
    const mine = ++serial.current; setBusy(true); setNotice(""); setContext(null); setSelected(null); setAddress(path);
    const load = applications ? request({ method: "search.query", params: { query: "", serial: mine } }).then(r => { if (mine === serial.current) setApps(r.sections.filter(s => s.provider === "apps").flatMap(s => s.items)); }) : request({ method: "files.list", params: { path, hidden } }).then(result => { if (mine === serial.current) setListing(result); });
    void load.catch(e => { if (mine === serial.current) { setListing(null); setApps([]); report(e); } }).finally(() => { if (mine === serial.current) setBusy(false); });
    return () => { serial.current++; };
  }, [path, hidden, refresh]);
  const items: Item[] = (applications ? apps.map(app => ({ id: app.id, name: app.title, app })) : (listing?.entries ?? []).map(file => ({ id: file.path, name: file.name, file }))).filter(item => item.name.toLowerCase().includes(query.toLowerCase()));
  const chosen = items.find(item => item.id === selected);
  function navigate(next: string) { if (!next || next === path) return; setHistory(old => [...old.slice(0, position + 1), next]); setPosition(position + 1); setQuery(""); }
  function travel(delta: number) { setPosition(n => Math.max(0, Math.min(history.length - 1, n + delta))); setQuery(""); }
  async function open(item: Item) {
    setContext(null);
    try { if (item.file?.directory) navigate(item.file.path); else if (item.file) await request({ method: "files.open", params: { path: item.file.path } }); else if (item.app) await request({ method: "search.activate", params: { provider: "apps", item_id: item.app.id } }); } catch (e) { report(e); }
  }
  async function addDesktop(item: Item) {
    setContext(null);
    try { if (item.file) await request({ method: "desktop.add_file", params: { path: item.file.path } }); else if (item.app) await request({ method: "desktop.add_app", params: { id: item.app.id } }); setNotice("Added to desktop"); } catch (e) { report(e); }
  }
  async function openTerminal(item: Item) {
    setContext(null);
    if (!item.file?.directory) return;
    try { await request({ method: "terminal.open_at", params: { path: item.file.path } }); setNotice(`Opened Terminal in ${item.name}`); }
    catch (e) { report(e); }
  }
  function showDialog(action: "mkdir" | "rename" | "trash", item?: Item) { setContext(null); setName(action === "rename" ? item?.name ?? "" : "Untitled folder"); setDialog({ action, item }); }
  async function mutate() {
    if (!dialog || busy) return; setBusy(true); setNotice("");
    try {
      if (dialog.action === "mkdir") await request({ method: "files.mkdir", params: { path: listing?.path ?? path, name } });
      else if (dialog.action === "rename" && dialog.item?.file) await request({ method: "files.rename", params: { path: dialog.item.file.path, name } });
      else if (dialog.action === "trash" && dialog.item?.file) await request({ method: "files.trash", params: { path: dialog.item.file.path } });
      setDialog(null); setRefresh(n => n + 1);
    } catch (e) { report(e); } finally { setBusy(false); }
  }
  return <main className="finder-app" onClick={() => setContext(null)} onKeyDown={e => {
    if (dialog || uninstall) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") { e.preventDefault(); search.current?.focus(); }
    if (e.ctrlKey && e.key.toLowerCase() === "h") { e.preventDefault(); setHidden(!hidden); }
    if (e.key === "Escape") { setContext(null); setQuery(""); }
    if (e.target instanceof HTMLInputElement) return;
    if (e.key === "Enter" && chosen) { e.preventDefault(); void open(chosen); }
    if (e.key === "F2" && chosen?.file) { e.preventDefault(); showDialog("rename", chosen); }
    if (e.key === "Delete" && chosen?.file) { e.preventDefault(); showDialog("trash", chosen); }
  }}>
    <aside className="finder-sidebar"><h2>Favorites</h2><nav aria-label="Favorites"><button aria-current={applications ? "page" : undefined} onClick={() => navigate("@applications")}><span>▦</span>Applications</button>{locations.filter(l => l.name !== "Computer").map(location => <button key={location.path + location.name} aria-current={path === location.path ? "page" : undefined} onClick={() => navigate(location.path)}><span>{location.name === "Home" ? "⌂" : "▱"}</span>{location.name}</button>)}</nav><h2>Locations</h2><nav><button aria-current={path === "/" ? "page" : undefined} onClick={() => navigate("/")}><span>▣</span>Computer</button></nav></aside>
    <section className="finder-body"><header className="finder-toolbar"><button aria-label="Go back" disabled={position === 0} onClick={() => travel(-1)}>‹</button><button aria-label="Go forward" disabled={position >= history.length - 1} onClick={() => travel(1)}>›</button><button aria-label="Enclosing folder" disabled={applications || !listing?.parent} onClick={() => listing?.parent && navigate(listing.parent)}>↑</button><h1>{title}</h1><div className="toolbar-spacer" /><button aria-label="Icon view" aria-pressed={view === "grid"} onClick={() => setView("grid")}>▦</button><button aria-label="List view" aria-pressed={view === "list"} onClick={() => setView("list")}>☰</button><button aria-label="Refresh" disabled={busy} onClick={() => setRefresh(n => n + 1)}>↻</button><button aria-label="New folder" disabled={busy || applications || !listing} onClick={() => showDialog("mkdir")}>＋</button><input ref={search} aria-label={applications ? "Search applications" : "Search this folder"} placeholder="Search" value={query} onChange={e => setQuery(e.target.value)} /></header>
      {notice && <p role="status" className="finder-notice">{notice}</p>}
      <div className={`file-collection ${view}`} aria-label={title} aria-busy={busy}>{!busy && items.map(item => <button key={item.id} className={`file-item${selected === item.id ? " selected" : ""}`} aria-label={item.name} aria-pressed={selected === item.id} draggable onDragStart={e => { if (item.file) e.dataTransfer.setData("text/uri-list", `file://${item.file.path.split("/").map(encodeURIComponent).join("/")}`); else if (item.app) e.dataTransfer.setData("application/x-meridian-app", item.app.id); e.dataTransfer.effectAllowed = "copy"; }} onClick={() => setSelected(item.id)} onDoubleClick={() => void open(item)} onContextMenu={e => { e.preventDefault(); setSelected(item.id); setContext({ item, x: e.clientX, y: e.clientY }); }}>
        {item.app ? <AppIcon id={item.app.id} name={item.app.title} icon={item.app.icon} /> : <FileIcon directory={!!item.file?.directory} />}<span className="file-name">{item.name}</span>{view === "list" && <><span className="file-kind">{item.app ? "Application" : item.file?.directory ? "Folder" : "File"}</span><span className="file-size">{item.file && !item.file.directory ? size(item.file.size) : "—"}</span><span className="file-date">{item.file?.modified ? new Date(item.file.modified * 1000).toLocaleDateString() : "—"}</span></>}
      </button>)}{busy && <p className="folder-empty">Loading…</p>}{!busy && !items.length && <p className="folder-empty">{query ? "No matching items" : "This folder is empty"}</p>}</div>
      <footer className="finder-footer"><span>{items.length} items{listing?.truncated && !applications ? " · Showing first 5,000" : ""}</span>{!applications && <form onSubmit={e => { e.preventDefault(); navigate(address); }}><input aria-label="Go to folder" value={address} onChange={e => setAddress(e.target.value)} /></form>}<label><input type="checkbox" checked={hidden} onChange={e => setHidden(e.target.checked)} />Hidden files</label></footer>
    </section>
    {context && <div role="menu" aria-label="Item actions" className="finder-context" style={{ left: Math.max(8, Math.min(context.x, window.innerWidth - 200)), top: Math.max(8, Math.min(context.y, window.innerHeight - 210)) }} onClick={e => e.stopPropagation()}><button role="menuitem" onClick={() => void open(context.item)}>Open</button><button role="menuitem" onClick={() => void addDesktop(context.item)}>Add to Desktop</button>{context.item.app ? <><button role="menuitem" onClick={() => { void request({ method: "dock.pin", params: { id: context.item.id, pinned: true } }).then(() => setNotice("Added to dock")).catch(report); setContext(null); }}>Add to Dock</button><button role="menuitem" disabled={isBuiltinApp(context.item.id)} onClick={() => { setUninstall(context.item.id); setContext(null); }}>Uninstall…</button></> : <>{context.item.file?.directory && <button role="menuitem" onClick={() => void openTerminal(context.item)}>Open in Terminal</button>}<button role="menuitem" onClick={() => showDialog("rename", context.item)}>Rename…</button><button role="menuitem" onClick={() => showDialog("trash", context.item)}>Move to Trash…</button></>}</div>}
    {dialog && <div className="finder-dialog-scrim"><form role="dialog" aria-modal="true" aria-label={dialog.action === "trash" ? "Move to Trash" : dialog.action === "mkdir" ? "New folder" : "Rename"} onSubmit={e => { e.preventDefault(); void mutate(); }}><h2>{dialog.action === "trash" ? `Move ${dialog.item?.name} to Trash?` : dialog.action === "mkdir" ? "New folder" : "Rename"}</h2>{dialog.action === "trash" ? <p>You can restore it from the system Trash.</p> : <input autoFocus aria-label="Name" value={name} disabled={busy} onChange={e => setName(e.target.value)} />}<div><button type="button" disabled={busy} onClick={() => setDialog(null)}>Cancel</button><button type="submit" disabled={busy || (dialog.action !== "trash" && !name.trim())}>{busy ? "Working…" : dialog.action === "trash" ? "Move to Trash" : "Save"}</button></div></form></div>}
    {uninstall && <UninstallDialog id={uninstall} close={() => { setUninstall(null); setRefresh(n => n + 1); }} />}
  </main>;
}
mount(<Finder />);
