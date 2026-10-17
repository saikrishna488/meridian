import { Trash } from "./Trash";
import { Properties, formatSize } from "./Properties";
import { Thumbnail, imageFile } from "./Thumbnail";
import { Select } from "../../components/Select";
import { useEffect, useRef, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { errorMessage, mount, useDialogFocus, useMenuFocus } from "../../lib/react";
import { Icon } from "../../components/Icon";
import { AppIcon } from "../../components/AppIcon";
import { UninstallDialog } from "../../components/UninstallDialog";
import { isBuiltinApp } from "../../lib/apps";
import type { FileEntry } from "../../lib/generated/FileEntry";
import type { FileListing } from "../../lib/generated/FileListing";
import type { FileLocation } from "../../lib/generated/FileLocation";
import type { SearchItem } from "../../lib/generated/SearchItem";

function fileKind(name: string) { const kinds: Record<string, string> = { pdf: "PDF document", txt: "Text document", md: "Markdown", zip: "Archive", rs: "Rust source", ts: "TypeScript", py: "Python source" }; return kinds[name.split(".").at(-1)?.toLowerCase() ?? ""] ?? "File"; }

type Item = { id: string; name: string; file?: FileEntry; app?: SearchItem };
function FileIcon({ directory, name = "" }: { directory: boolean; name?: string }) {
  const extension = name.includes(".") ? name.split(".").at(-1)!.toUpperCase() : "";
  const tag = extension.length <= 5 ? extension : "";
  const color = extension === "PDF" ? "#d8455e" : ["ZIP", "TAR", "GZ", "7Z"].includes(extension) ? "#a77021" : ["JS", "TS", "TSX", "RS", "PY", "MD"].includes(extension) ? "#7763be" : "#5d7fa8";
  return <svg className="file-art" viewBox="0 0 72 64" aria-hidden="true">{directory ? <><path d="M6 14h24l6 6h30v38H6Z" fill="#6ab7ec" /><path d="M6 26h60v28a5 5 0 0 1-5 5H11a5 5 0 0 1-5-5Z" fill="#83c8f3" /><path d="M7 26h58" stroke="#bce5ff" /><g fill="none" stroke="#246b9d" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">{name === "Downloads" ? <path d="M36 31v18m-7-7 7 7 7-7M26 51h20" /> : name === "Documents" ? <path d="M29 31h14v21H29Zm4 6h6m-6 5h6m-6 5h6" /> : name === "Pictures" ? <><rect x="25" y="32" width="22" height="18" rx="2" /><path d="m26 48 7-7 5 4 4-4 4 6" /><circle cx="41" cy="37" r="1" /></> : name === "Music" ? <><path d="M33 47V33l12-2v13M33 36l12-2" /><ellipse cx="30" cy="48" rx="3" ry="2" /><ellipse cx="42" cy="45" rx="3" ry="2" /></> : name === "Videos" ? <><rect x="25" y="33" width="22" height="17" rx="2" /><path d="m34 38 6 4-6 4Z" /></> : name === "Desktop" ? <path d="M25 32h22v15H25Zm11 15v5m-6 0h12" /> : name === "Home" ? <path d="m25 40 11-9 11 9m-19-2v13h16V38m-11 13v-8h6v8" /> : null}</g></> : <><path d="M16 5h28l13 13v42H16Z" fill="#fff" stroke="#c8cdd5" /><path d="M44 5v14h13M24 28h25M24 34h19" fill="none" stroke="#c8cdd5" />{tag && <><rect x="22" y="40" width="29" height="13" rx="3" fill={color} /><text x="36.5" y="49.4" fill="white" fontSize="7.5" fontWeight="600" textAnchor="middle" fontFamily="var(--font-ui)">{tag}</text></>}</>}</svg>;
}

function Orbit() {
  const [locations, setLocations] = useState<FileLocation[]>([]);
  const [history, setHistory] = useState<string[]>([]), [position, setPosition] = useState(0);
  const [listing, setListing] = useState<FileListing | null>(null), [apps, setApps] = useState<SearchItem[]>([]);
  const [query, setQuery] = useState(""), [view, setView] = useState<"grid" | "list">(() => { try { return localStorage.getItem("meridian.finder-view") === "list" ? "list" : "grid"; } catch { return "grid"; } });
  const [sort, setSort] = useState(() => { try { const value = localStorage.getItem("meridian.finder-sort"); return ["name", "modified", "size"].includes(value ?? "") ? value! : "name"; } catch { return "name"; } });
  const [descending, setDescending] = useState(() => { try { return localStorage.getItem("meridian.finder-descending") === "true"; } catch { return false; } });
  const [properties, setProperties] = useState<string | null>(null);
  const collection = useRef<HTMLDivElement>(null), addressInput = useRef<HTMLInputElement>(null), contextMenu = useRef<HTMLDivElement>(null), fileDialog = useRef<HTMLFormElement>(null);
  useEffect(() => { try { localStorage.setItem("meridian.finder-view", view); } catch {} }, [view]);
  const [hidden, setHidden] = useState(() => { try { return localStorage.getItem("meridian.finder-hidden") === "true"; } catch { return false; } }), [selected, setSelected] = useState<string | null>(null);
  const [loading, setLoading] = useState(false), [mutating, setMutating] = useState(false);
  const busy = loading || mutating;
  const [notice, setNotice] = useState(""), [refresh, setRefresh] = useState(0);
  const [context, setContext] = useState<{ item?: Item; x: number; y: number } | null>(null);
  const [dialog, setDialog] = useState<{ action: "mkdir" | "rename" | "trash" | "copy" | "move"; item?: Item } | null>(null), [name, setName] = useState("");
  const [uninstall, setUninstall] = useState<string | null>(null), [address, setAddress] = useState("");
  const [clipboard, setClipboard] = useState<{ path: string; move: boolean } | null>(null);
  const [bookmarks, setBookmarks] = useState<string[]>([]);
  const latestDirectory = useRef<string | null>(null), lastRequestedPath = useRef("");
  const serial = useRef(0), search = useRef<HTMLInputElement>(null);
  useEffect(() => { try { localStorage.setItem("meridian.finder-sort", sort); localStorage.setItem("meridian.finder-descending", String(descending)); localStorage.setItem("meridian.finder-hidden", String(hidden)); } catch {} }, [sort, descending, hidden]);
  const path = history[position] ?? "", applications = path === "@applications", trashed = path === "@trash";
  const title = applications ? "Applications" : locations.find(l => l.path === path)?.name ?? path.split('/').filter(Boolean).at(-1) ?? "Computer";
  const report = (e: unknown) => setNotice(errorMessage(e));
  useEffect(() => {
    let alive = true;
    void hello().then(() => request({ method: "files.locations" })).then(result => { if (alive) { setLocations(result); setHistory(old => old.length ? old : [result[0]?.path ?? "/"]); } }).catch(report);
    void hello().then(() => request({ method: "files.bookmarks" })).then(result => { if (alive) setBookmarks((result ?? []).map(l => l.path)); }).catch(report);
    const off = on("search.invalidated", () => setRefresh(n => n + 1));
    const offLocation = on("finder.location", e => { setHistory([e.data.path]); setPosition(0); setQuery(""); });
    const offFiles = on("files.changed", e => { if (e.data.path === latestDirectory.current) setRefresh(n => n + 1); });
    return () => { alive = false; off(); offLocation(); offFiles(); };
  }, []);
  useEffect(() => {
    latestDirectory.current = null;
    if (!path || trashed) return;
    const mine = ++serial.current; setLoading(true); setNotice(""); setContext(null); if (lastRequestedPath.current !== path) setSelected(null); lastRequestedPath.current = path; setAddress(path);
    const load = applications ? request({ method: "search.query", params: { query: "", serial: mine } }).then(r => { if (mine === serial.current) setApps(r.sections.filter(s => s.provider === "apps").flatMap(s => s.items)); }) : request({ method: "files.list", params: { path, hidden } }).then(result => { if (mine === serial.current) { latestDirectory.current = result.path; setListing(result); } });
    void load.catch(e => { if (mine === serial.current) { setListing(null); setApps([]); report(e); } }).finally(() => { if (mine === serial.current) setLoading(false); });
    return () => { serial.current++; };
  }, [path, hidden, refresh]);
  const items: Item[] = (applications ? apps.map(app => ({ id: app.id, name: app.title, app })) : (listing?.entries ?? []).map(file => ({ id: file.path, name: file.name, file }))).filter(item => item.name.toLowerCase().includes(query.toLowerCase()));
  function compareNames(a: Item, b: Item) { return a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" }); }
  items.sort((a, b) => Number(!!b.file?.directory) - Number(!!a.file?.directory) || (descending ? -1 : 1) * ((sort === "modified" ? (a.file?.modified ?? 0) - (b.file?.modified ?? 0) : sort === "size" ? (a.file?.size ?? 0) - (b.file?.size ?? 0) : compareNames(a, b)) || compareNames(a, b)));
  const breadcrumbs = applications ? [] : (listing?.path ?? path).split("/").filter(Boolean).map((name, i, parts) => ({ name, path: "/" + parts.slice(0, i + 1).join("/") }));
  function chooseSort(value: string) { setSort(value); setDescending(value !== "name"); }
  function showProperties(target: string) { setContext(null); setProperties(target); }
  useMenuFocus(contextMenu, !!context, () => setContext(null));
  useDialogFocus(fileDialog, !!dialog, () => { if (!busy) setDialog(null); });
  const chosen = items.find(item => item.id === selected);
  function navigate(next: string) { if (!next || next === path) return; setHistory(old => [...old.slice(0, position + 1), next]); setPosition(position + 1); setQuery(""); }
  function travel(delta: number) { setPosition(n => Math.max(0, Math.min(history.length - 1, n + delta))); setQuery(""); }
  async function open(item: Item) {
    setContext(null);
    try { if (item.file?.directory) navigate(item.file.path); else if (item.file) await request({ method: "files.open", params: { path: item.file.path } }); else if (item.app) await request({ method: "search.activate", params: { provider: "apps", item_id: item.app.id } }); } catch (e) { report(e); }
  }
  async function addDesktop(item: Item) {
    setContext(null);
    try { if (item.file) await request({ method: "desktop.add_file", params: { path: item.file.path } }); else if (item.app) await request({ method: "desktop.add_app", params: { id: item.app.id } }); setNotice(item.file ? "Moved to Desktop" : "Added to desktop"); setRefresh(n => n + 1); } catch (e) { report(e); }
  }
  async function openTerminal(item?: Item) {
    setContext(null);
    const directory = item?.file?.directory ? item.file.path : listing?.path;
    if (applications || !directory || busy) return;
    try { await request({ method: "terminal.open_at", params: { path: directory } }); setNotice(`Opened Terminal in ${directory}`); }
    catch (e) { report(e); }
  }
  function showDialog(action: "mkdir" | "rename" | "trash" | "copy" | "move", item?: Item) { setContext(null); setName(action === "rename" ? item?.name ?? "" : action === "copy" || action === "move" ? listing?.path ?? path : "Untitled folder"); setDialog({ action, item }); }
  async function mutate() {
    if (!dialog || busy) return; setMutating(true); setNotice("");
    try {
      if ((dialog.action === "copy" || dialog.action === "move") && dialog.item?.file) await request({ method: "files.transfer", params: { path: dialog.item.file.path, destination: name, move_file: dialog.action === "move" } });
      else if (dialog.action === "mkdir") await request({ method: "files.mkdir", params: { path: listing?.path ?? path, name } });
      else if (dialog.action === "rename" && dialog.item?.file) await request({ method: "files.rename", params: { path: dialog.item.file.path, name } });
      else if (dialog.action === "trash" && dialog.item?.file) await request({ method: "files.trash", params: { path: dialog.item.file.path } });
      setDialog(null); setRefresh(n => n + 1);
    } catch (e) { report(e); } finally { setMutating(false); }
  }
  async function bookmark(target: string) { setContext(null); try { setBookmarks((await request({ method: "files.bookmark_set", params: { path: target } })).map(l => l.path)); } catch (e) { report(e); } }
  function copy(item: Item, move: boolean) { if (item.file) { setClipboard({ path: item.file.path, move }); setNotice(move ? `Ready to move ${item.name}` : `Copied ${item.name}`); } setContext(null); }
  async function paste() {
    if (!clipboard || applications || !listing || busy) return;
    setContext(null); setMutating(true);
    try { await request({ method: "files.transfer", params: { path: clipboard.path, destination: listing.path, move_file: clipboard.move } }); if (clipboard.move) setClipboard(null); setRefresh(n => n + 1); }
    catch (e) { report(e); } finally { setMutating(false); }
  }
  async function wallpaper(item: Item) { setContext(null); if (!item.file) return; try { await request({ method: "files.wallpaper", params: { path: item.file.path } }); setNotice("Wallpaper updated"); } catch (e) { report(e); } }
  return <main className="finder-app" onClick={() => setContext(null)} onKeyDown={e => {
    if (dialog || uninstall || properties || trashed) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") { e.preventDefault(); search.current?.focus(); }
    if (e.ctrlKey && e.key.toLowerCase() === "h") { e.preventDefault(); setHidden(!hidden); }
    if (e.key === "Escape") { setContext(null); setQuery(""); }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "l") { e.preventDefault(); addressInput.current?.focus(); addressInput.current?.select(); return; }
    if (e.altKey && e.key === "ArrowLeft") { e.preventDefault(); travel(-1); return; }
    if (e.altKey && e.key === "ArrowRight") { e.preventDefault(); travel(1); return; }
    if (e.key === "F5") { e.preventDefault(); setRefresh(n => n + 1); return; }
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement || context) return;
    if ((e.ctrlKey || e.metaKey) && ["c", "x", "v", "d"].includes(e.key.toLowerCase())) {
      e.preventDefault();
      if (e.key.toLowerCase() === "v") void paste();
      else if (e.key.toLowerCase() === "d" && !applications && listing) bookmark(listing.path);
      else if (chosen?.file) copy(chosen, e.key.toLowerCase() === "x");
      return;
    }
    if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(e.key) && items.length) {
      e.preventDefault();
      const index = items.findIndex(item => item.id === selected);
      const columns = view === "list" ? 1 : Math.max(1, getComputedStyle(collection.current!).gridTemplateColumns.split(" ").length);
      const offset = e.key === "ArrowLeft" ? -1 : e.key === "ArrowRight" ? 1 : e.key === "ArrowUp" ? -columns : columns;
      const next = e.key === "Home" || index < 0 && e.key !== "End" ? 0 : e.key === "End" ? items.length - 1 : Math.max(0, Math.min(items.length - 1, index + offset));
      setSelected(items[next]!.id); collection.current?.querySelectorAll<HTMLButtonElement>(".file-item")[next]?.focus(); return;
    }
    if ((e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) && (chosen || (!applications && listing && !busy))) {
      e.preventDefault(); const bounds = (document.activeElement as HTMLElement).getBoundingClientRect();
      setContext({ item: chosen, x: bounds.left, y: bounds.bottom }); return;
    }
    if (e.altKey && e.key === "Enter" && chosen?.file) { e.preventDefault(); showProperties(chosen.file.path); return; }
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "n" && !applications && !busy) { e.preventDefault(); showDialog("mkdir"); return; }
    if (e.altKey && e.key === "ArrowUp" && listing?.parent && !applications) { e.preventDefault(); navigate(listing.parent); return; }
    if (e.key === "Enter" && chosen) { e.preventDefault(); void open(chosen); }
    if (e.key === "F2" && chosen?.file) { e.preventDefault(); showDialog("rename", chosen); }
    if (e.key === "Delete" && chosen?.file) { e.preventDefault(); showDialog("trash", chosen); }
  }}>
    <aside className="finder-sidebar"><div className="orbit-brand"><span><Icon name="folder" width="22" height="22" /></span><strong>Orbit</strong><small>Files, in order.</small></div><h2>Favorites</h2><nav aria-label="Favorites"><button title="Applications" aria-current={applications ? "page" : undefined} onClick={() => navigate("@applications")}><Icon name="grid" />Applications</button>{locations.filter(l => l.name !== "Computer").map(location => <button key={location.path + location.name} title={location.name} aria-current={path === location.path ? "page" : undefined} onClick={() => navigate(location.path)}><FileIcon directory name={location.name} />{location.name}</button>)}</nav>{bookmarks.length > 0 && <><h2>Bookmarks</h2><nav aria-label="Bookmarks">{bookmarks.map(target => <div className="bookmark-row" key={target}><button title={target} aria-current={path === target ? "page" : undefined} onClick={() => navigate(target)}><Icon name="folder" />{target.split("/").filter(Boolean).at(-1) ?? "/"}</button><button aria-label={`Remove bookmark ${target}`} onClick={() => bookmark(target)}><Icon name="close" width="12" height="12" /></button></div>)}</nav></>}<h2>Locations</h2><nav><button title="Trash" aria-current={trashed ? "page" : undefined} onClick={() => navigate("@trash")}><Icon name="trash" />Trash</button><button title="Computer" aria-current={path === "/" ? "page" : undefined} onClick={() => navigate("/")}><Icon name="monitor" />Computer</button></nav></aside>
    {trashed ? <Trash back={() => position > 0 ? travel(-1) : navigate(locations[0]?.path ?? "/")} navigate={navigate} /> : <section className="finder-body"><header className="finder-toolbar"><button aria-label="Go back" disabled={position === 0} onClick={() => travel(-1)} title="Back (Alt+Left)"><Icon name="back" width="17" height="17" /></button><button aria-label="Go forward" disabled={position >= history.length - 1} onClick={() => travel(1)} title="Forward (Alt+Right)"><Icon name="forward" width="17" height="17" /></button><button aria-label="Enclosing folder" disabled={applications || !listing?.parent} onClick={() => listing?.parent && navigate(listing.parent)} title="Enclosing folder"><Icon name="up" width="17" height="17" /></button><h1>{title}</h1><button aria-label={bookmarks.includes(listing?.path ?? path) ? "Remove bookmark" : "Bookmark folder"} title="Bookmark folder (Ctrl+D)" disabled={applications || !listing || busy} onClick={() => bookmark(listing?.path ?? path)}><Icon name="pin" width="16" height="16" /></button><div className="toolbar-spacer" /><button aria-label="Icon view" aria-pressed={view === "grid"} onClick={() => setView("grid")} title="Icon view"><Icon name="grid" width="17" height="17" /></button><button aria-label="List view" aria-pressed={view === "list"} onClick={() => setView("list")} title="List view"><Icon name="list" width="17" height="17" /></button><button aria-label="File properties" disabled={applications || !chosen?.file || busy} onClick={() => chosen?.file && showProperties(chosen.file.path)} title="Properties (Alt+Enter)"><Icon name="info" width="17" height="17" /></button><button aria-label="Refresh" disabled={busy} onClick={() => setRefresh(n => n + 1)} title="Refresh (F5)"><Icon name="refresh" width="17" height="17" /></button><button aria-label="New folder" disabled={busy || applications || !listing} onClick={() => showDialog("mkdir")} title="New folder"><Icon name="plus" width="19" height="19" /></button><label className="finder-search"><Icon name="search" width="15" height="15" /><input ref={search} aria-label={applications ? "Search applications" : "Search this folder"} placeholder="Search" value={query} onChange={e => setQuery(e.target.value)} /></label></header>
      <div className="finder-locationbar"><form onSubmit={e => { e.preventDefault(); navigate(address); }}><Icon name={applications ? "grid" : "folder"} width="16" height="16" /><input ref={addressInput} aria-label="Go to folder" title="Enter a folder path (Ctrl+L)" value={applications ? "Applications" : address} disabled={applications} onChange={e => setAddress(e.target.value)} /></form><label>Sort by<Select aria-label="Sort files by" value={sort} onChange={e => chooseSort(e.target.value)}><option value="name">Name</option><option value="modified">Modified</option><option value="size">Size</option></Select><button className="sort-direction" aria-label={descending ? "Sort ascending" : "Sort descending"} title={descending ? "Descending" : "Ascending"} onClick={() => setDescending(value => !value)}><Icon name={descending ? "down" : "up"} width="15" height="15" /></button></label></div>
      {!applications && <nav className="finder-breadcrumbs" aria-label="Folder path"><button onClick={() => navigate("/")} title="Computer"><Icon name="monitor" width="14" height="14" /></button>{breadcrumbs.map((crumb, index) => <span key={crumb.path}><Icon name="forward" width="11" height="11" /><button aria-current={index === breadcrumbs.length - 1 ? "location" : undefined} title={crumb.path} onClick={() => navigate(crumb.path)}>{crumb.name}</button></span>)}</nav>}
      {view === "list" && <div className="finder-list-heading" aria-hidden="true"><span /><span>Name</span><span>Kind</span><span>Size</span><span>Modified</span></div>}
      {notice && <p role="status" className="finder-notice">{notice}</p>}
      <div ref={collection} className={`file-collection ${view}`} aria-label={title} aria-busy={busy} tabIndex={0} onContextMenu={e => { e.preventDefault(); if (!applications && listing && !busy) { setSelected(null); setContext({ x: e.clientX, y: e.clientY }); } }}>{!busy && items.map(item => <button key={item.id} className={`file-item${selected === item.id ? " selected" : ""}`} aria-label={item.name} aria-pressed={selected === item.id} draggable onDragStart={e => { if (item.file) e.dataTransfer.setData("text/uri-list", `file://${item.file.path.split("/").map(encodeURIComponent).join("/")}`); else if (item.app) e.dataTransfer.setData("application/x-meridian-app", item.app.id); e.dataTransfer.effectAllowed = "copyMove"; }} onClick={() => setSelected(item.id)} onDoubleClick={() => void open(item)} onContextMenu={e => { e.preventDefault(); e.stopPropagation(); setSelected(item.id); setContext({ item, x: e.clientX, y: e.clientY }); }}>
        {item.app ? <AppIcon id={item.app.id} name={item.app.title} icon={item.app.icon} /> : imageFile(item.file) ? <Thumbnail file={item.file!} fallback={<FileIcon directory={false} name={item.name} />} /> : <FileIcon directory={!!item.file?.directory} name={item.name} />}<span className="file-name">{item.name}</span>{view === "list" && <><span className="file-kind">{item.app ? "Application" : item.file?.symlink ? "Symbolic link" : item.file?.directory ? "Folder" : fileKind(item.name)}</span><span className="file-size">{item.file && !item.file.directory ? formatSize(item.file.size) : "—"}</span><span className="file-date">{item.file?.modified ? new Date(item.file.modified * 1000).toLocaleDateString() : "—"}</span></>}
      </button>)}{busy && <div className="folder-empty" role="status"><span className="finder-spinner" /><p>Loading this folder…</p></div>}{!busy && !items.length && <div className="folder-empty"><Icon name={query ? "search" : "folder"} width="44" height="44" /><h2>{query ? "No matching items" : notice && !listing && !applications ? "This folder is unavailable" : "This folder is empty"}</h2>{query && <p>Try another name.</p>}{query ? <button onClick={() => setQuery("")}>Clear search</button> : !applications && listing ? <button onClick={() => showDialog("mkdir")}><Icon name="plus" width="16" height="16" />New folder</button> : null}</div>}</div>
      <footer className="finder-footer"><span>{items.length} items{listing?.truncated && !applications ? " · Showing first 5,000" : ""}</span><span className="footer-selection">{chosen ? `${chosen.name}${chosen.file && !chosen.file.directory ? ` · ${formatSize(chosen.file.size)}` : ""}` : query ? "Filtered results" : ""}</span><label><input type="checkbox" checked={hidden} onChange={e => setHidden(e.target.checked)} />Hidden files</label></footer>
    </section>}
    {context && <div ref={contextMenu} role="menu" aria-label={context.item ? "Item actions" : "Folder actions"} className="finder-context" style={{ left: Math.max(8, Math.min(context.x, window.innerWidth - 210)), top: Math.max(8, Math.min(context.y, window.innerHeight - 420)) }} onClick={e => e.stopPropagation()}>
      {context.item ? <>
        <button role="menuitem" onClick={() => void open(context.item!)}>Open</button>
        {context.item.app ? <>
          <button role="menuitem" onClick={() => void addDesktop(context.item!)}>Add to Desktop</button>
          <button role="menuitem" onClick={() => { void request({ method: "dock.pin", params: { id: context.item!.id, pinned: true } }).catch(report); setContext(null); }}>Add to Dock</button>
          <button role="menuitem" disabled={isBuiltinApp(context.item.id)} onClick={() => { setUninstall(context.item!.id); setContext(null); }}>Uninstall…</button>
        </> : <>
          <button role="menuitem" onClick={() => void openTerminal(context.item)}>Open in Terminal</button>
          {imageFile(context.item.file) && /\.(png|jpe?g|webp)$/i.test(context.item.name) && <button role="menuitem" onClick={() => void wallpaper(context.item!)}>Set as Wallpaper</button>}
          {context.item.file?.directory && <button role="menuitem" onClick={() => bookmark(context.item!.file!.path)}>{bookmarks.includes(context.item.file.path) ? "Remove Bookmark" : "Bookmark"}</button>}
          <hr /><button role="menuitem" onClick={() => copy(context.item!, false)}>Copy</button><button role="menuitem" onClick={() => copy(context.item!, true)}>Cut</button>
          <button role="menuitem" onClick={() => showDialog("copy", context.item)}>Copy To…</button><button role="menuitem" onClick={() => showDialog("move", context.item)}>Move To…</button>
          <button role="menuitem" onClick={() => void addDesktop(context.item!)}>Move to Desktop</button>
          <hr /><button role="menuitem" onClick={() => showProperties(context.item!.file!.path)}>Properties<span className="menu-shortcut">Alt+Enter</span></button><button role="menuitem" onClick={() => showDialog("rename", context.item)}>Rename…</button><button role="menuitem" onClick={() => showDialog("trash", context.item)}>Delete…</button>
        </>}
      </> : <><button role="menuitem" onClick={() => void openTerminal()}>Open Terminal Here</button><button role="menuitem" disabled={!clipboard || busy} onClick={() => void paste()}>Paste</button><button role="menuitem" onClick={() => bookmark(listing?.path ?? path)}>{bookmarks.includes(listing?.path ?? path) ? "Remove Bookmark" : "Bookmark Folder"}</button><button role="menuitem" onClick={() => showDialog("mkdir")}>New Folder…</button><hr /><button role="menuitem" onClick={() => showProperties(listing?.path ?? path)}>Folder Properties</button></>}
    </div>}
    {dialog && <div className="finder-dialog-scrim"><form ref={fileDialog} role="dialog" aria-modal="true" aria-label={dialog.action === "trash" ? "Delete" : dialog.action === "mkdir" ? "New folder" : dialog.action === "copy" ? "Copy To" : dialog.action === "move" ? "Move To" : "Rename"} onSubmit={e => { e.preventDefault(); void mutate(); }}>
      <h2>{dialog.action === "trash" ? `Delete ${dialog.item?.name}?` : dialog.action === "mkdir" ? "New folder" : dialog.action === "copy" ? `Copy ${dialog.item?.name}` : dialog.action === "move" ? `Move ${dialog.item?.name}` : "Rename"}</h2>
      {dialog.action === "trash" ? <p>The item will be moved to Trash.</p> : <label>{dialog.action === "copy" || dialog.action === "move" ? "Destination folder" : "Name"}<input data-autofocus aria-label={dialog.action === "copy" || dialog.action === "move" ? "Destination folder" : "Name"} value={name} disabled={busy} onChange={e => setName(e.target.value)} />{(dialog.action === "copy" || dialog.action === "move") && <button type="button" disabled={busy} onClick={() => void request({ method: "files.choose_folder" }).then(path => { if (path) setName(path); }).catch(report)}>Choose Folder…</button>}</label>}
      <div><button type="button" disabled={busy} onClick={() => setDialog(null)}>Cancel</button><button type="submit" disabled={busy || (dialog.action !== "trash" && !name.trim())}>{busy ? "Working…" : dialog.action === "trash" ? "Delete" : dialog.action === "copy" ? "Copy" : dialog.action === "move" ? "Move" : "Save"}</button></div>
    </form></div>}
    {properties && <Properties path={properties} close={() => setProperties(null)} />}
    {uninstall && <UninstallDialog id={uninstall} close={() => { setUninstall(null); setRefresh(n => n + 1); }} />}
  </main>;
}
mount(<Orbit />);
