import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import { Icon } from "../../components/Icon";
import { formatSize } from "./Properties";
import type { TrashListing } from "../../lib/generated/TrashListing";

export function Trash({ back, navigate }: { back: () => void; navigate: (path: string) => void }) {
  const [listing, setListing] = useState<TrashListing | null>(null), [selected, setSelected] = useState("");
  const [query, setQuery] = useState(""), [busy, setBusy] = useState(false), [error, setError] = useState(""), [restored, setRestored] = useState("");
  const pending = useRef(false), alive = useRef(true), list = useRef<HTMLDivElement>(null);
  async function load() {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError("");
    try { const result = await request({ method: "files.trash_list" }); if (alive.current) { setListing(result); setSelected(old => result.entries.some(e => e.uri === old) ? old : ""); } }
    catch (e) { if (alive.current) setError(errorMessage(e)); }
    finally { pending.current = false; if (alive.current) setBusy(false); }
  }
  useEffect(() => { alive.current = true; void load(); return () => { alive.current = false; }; }, []);
  const entries = (listing?.entries ?? []).filter(e => `${e.name} ${e.original}`.toLowerCase().includes(query.toLowerCase()));
  const chosen = entries.find(e => e.uri === selected);
  async function restore() {
    if (!chosen || pending.current) return;
    pending.current = true; setBusy(true); setError(""); setRestored("");
    try {
      const path = await request({ method: "files.trash_restore", params: { uri: chosen.uri } });
      if (alive.current) { setRestored(path); setSelected(""); setListing(old => old && { ...old, entries: old.entries.filter(e => e.uri !== chosen.uri) }); }
    } catch (e) { if (alive.current) setError(errorMessage(e)); }
    finally { pending.current = false; if (alive.current) setBusy(false); }
  }
  return <section className="finder-body trash-browser" aria-label="Trash" onKeyDown={e => {
    if (e.key === "F5") { e.preventDefault(); void load(); }
    if (e.altKey && e.key === "ArrowLeft") { e.preventDefault(); back(); }
    if (e.target instanceof HTMLInputElement || busy || !entries.length) return;
    if (["ArrowUp", "ArrowDown", "Home", "End"].includes(e.key)) {
      e.preventDefault(); const index = entries.findIndex(e => e.uri === selected);
      const next = e.key === "Home" ? 0 : e.key === "End" ? entries.length - 1 : Math.max(0, Math.min(entries.length - 1, index + (e.key === "ArrowUp" ? -1 : 1)));
      setSelected(entries[next]!.uri); list.current?.querySelectorAll<HTMLButtonElement>(".trash-item")[next]?.focus();
    }
  }}><header className="finder-toolbar"><button aria-label="Leave Trash" onClick={back} title="Back to files"><Icon name="back" width="17" height="17" /></button><h1>Trash</h1><div className="toolbar-spacer" /><button aria-label="Refresh Trash" disabled={busy} onClick={() => void load()}><Icon name="refresh" width="17" height="17" /></button><button className="restore-button" disabled={busy || !chosen?.original} onClick={() => void restore()}><Icon name="refresh" width="15" height="15" />{busy && chosen ? "Restoring…" : "Restore"}</button></header>
    <div className="trash-intro"><div><span className="trash-intro-icon"><Icon name="trash" width="24" height="24" /></span><div><h2>Restore files from Trash</h2><p>Select an item to restore it to its original folder.</p></div></div><label className="finder-search"><Icon name="search" width="15" height="15" /><input aria-label="Search Trash" placeholder="Search Trash" value={query} onChange={e => setQuery(e.target.value)} /></label></div>
    {error && <p className="trash-error" role="alert">{error}</p>}
    {restored && <div className="trash-restored" role="status"><span>Restored {restored.split("/").at(-1)}.</span><button onClick={() => navigate(restored.slice(0, restored.lastIndexOf("/")) || "/")}>Show folder<Icon name="forward" width="12" height="12" /></button></div>}
    <div className="trash-list-heading" aria-hidden="true"><span>Name & original location</span><span>Deleted</span><span>Size</span></div>
    <div ref={list} className="trash-list" aria-busy={busy}>{entries.map(entry => <button className={`trash-item${selected === entry.uri ? " selected" : ""}`} key={entry.uri} aria-label={entry.name} aria-pressed={selected === entry.uri} disabled={busy} onClick={() => setSelected(entry.uri)}><span className="trash-file-icon"><Icon name={entry.directory ? "folder" : "file"} width="25" height="25" /></span><span className="trash-item-name"><strong>{entry.name}</strong><small title={entry.original}>{entry.original || "Original location unavailable"}</small></span><span className="trash-item-date">{entry.deleted && !Number.isNaN(Date.parse(entry.deleted)) ? new Date(entry.deleted).toLocaleDateString() : "—"}</span><span className="trash-item-size">{entry.directory ? "Folder" : formatSize(entry.size)}</span></button>)}
    {!entries.length && !busy && listing && <div className="folder-empty"><Icon name={query ? "search" : "trash"} width="42" height="42" /><h2>{query ? "No matching items" : "Trash is empty"}</h2><p>{query ? "Try another name or original location." : "Files you move to Trash will appear here."}</p>{query && <button onClick={() => setQuery("")}>Clear search</button>}</div>}{busy && !listing && <p className="folder-empty" role="status">Reading Trash…</p>}</div>
    <footer className="finder-footer"><span>{entries.length} {entries.length === 1 ? "item" : "items"}{listing?.truncated ? " · Showing first 5,000" : ""}</span><span className="footer-selection">{chosen?.name ?? ""}</span><span>Original files are never overwritten.</span></footer>
  </section>;
}
