import { useEffect, useRef, useState } from "react";
import type { ChangeEvent } from "react";
type LocalFile = { id: string; file: File; url: string };
function useLibrary() {
  const [items, setItems] = useState<LocalFile[]>([]);
  const urls = useRef(new Set<string>());
  useEffect(() => () => { urls.current.forEach(url => URL.revokeObjectURL(url)); }, []);
  function add(files: File[]) {
    const next = files.map(file => { const url = URL.createObjectURL(file); urls.current.add(url); return { id: crypto.randomUUID(), file, url }; });
    setItems(previous => [...previous, ...next]);
    return next;
  }
  function clear() { urls.current.forEach(url => URL.revokeObjectURL(url)); urls.current.clear(); setItems([]); }
  return { items, add, clear };
}
function OpenFiles({ accept, title, onFiles }: { accept: string; title: string; onFiles: (files: File[]) => void }) {
  const input = useRef<HTMLInputElement>(null);
  return <><input className="local-file-input" ref={input} type="file" multiple accept={accept} tabIndex={-1} onChange={(event: ChangeEvent<HTMLInputElement>) => { onFiles(Array.from(event.target.files ?? [])); event.target.value = ""; }} /><button className="suite-primary" onClick={() => input.current?.click()}>{title}</button></>;
}
export function Photos() {
  const { items, add, clear } = useLibrary();
  const [selected, setSelected] = useState<string | null>(null), [query, setQuery] = useState("");
  const [rotation, setRotation] = useState(0), [zoom, setZoom] = useState(1), [error, setError] = useState("");
  const filtered = items.filter(item => item.file.name.toLowerCase().includes(query.toLowerCase()));
  const index = filtered.findIndex(item => item.id === selected), current = filtered[index];
  const back = useRef<HTMLButtonElement>(null);
  useEffect(() => { setRotation(0); setZoom(1); setError(""); if (selected) back.current?.focus(); }, [selected]);
  function close() { const id = selected; setSelected(null); requestAnimationFrame(() => document.getElementById(`photo-${id}`)?.focus()); }
  function step(delta: number) { if (filtered.length) setSelected(filtered[(index + delta + filtered.length) % filtered.length]!.id); }
  return <><header className="suite-header"><div><span className="suite-kicker">YOUR MOMENTS</span><h1>Photos</h1><p className="muted">{items.length ? `${items.length} photos in this session` : "A little space for your favorite moments."}</p></div><OpenFiles accept="image/*" title="Open photos" onFiles={files => { add(files.filter(file => file.type.startsWith("image/") || /\.(jpe?g|png|webp|gif|avif|heic|bmp|svg)$/i.test(file.name))); }} /></header>
    {current ? <section className="photo-viewer" aria-label="Photo viewer" onKeyDown={event => { if (event.target instanceof HTMLInputElement) return; if (event.key === "ArrowLeft") { event.preventDefault(); step(-1); } if (event.key === "ArrowRight") { event.preventDefault(); step(1); } if (event.key === "Escape") close(); }}><div className="suite-toolbar"><button ref={back} onClick={close}>← Library</button><strong>{current.file.name}</strong><span>{index + 1} / {filtered.length}</span></div><div className="photo-stage"><img key={current.id} src={current.url} alt={current.file.name} style={{ transform: `rotate(${rotation}deg) scale(${zoom})` }} onError={() => setError("This image format cannot be displayed. Try a JPEG, PNG or WebP image.")} /></div>{error && <p role="alert">{error}</p>}<div className="suite-toolbar photo-controls"><button onClick={() => step(-1)} disabled={filtered.length < 2}>← Previous</button><button onClick={() => setRotation(value => (value + 90) % 360)}>Rotate ↻</button><label>Zoom <input aria-label="Photo zoom" type="range" min="1" max="3" step=".1" value={zoom} onChange={event => setZoom(Number(event.target.value))} /></label><button onClick={() => { setZoom(1); setRotation(0); }}>Fit</button><button onClick={() => step(1)} disabled={filtered.length < 2}>Next →</button></div><p className="muted">Rotation and zoom are for viewing; original files stay unchanged.</p></section>
    : <><div className="suite-toolbar"><input className="library-search" type="search" aria-label="Search photos" placeholder="Find a photo by name…" value={query} onChange={event => setQuery(event.target.value)} />{items.length > 0 && <button onClick={() => { clear(); setQuery(""); }}>Clear library</button>}</div>{filtered.length ? <div className="photo-grid">{filtered.map(item => <button key={item.id} id={`photo-${item.id}`} onClick={() => setSelected(item.id)}><img src={item.url} alt="" loading="lazy" /><span>{item.file.name}</span></button>)}</div> : <div className="suite-empty"><span aria-hidden="true">▧</span><h2>{query ? "No matching photos" : "Bring your photos in"}</h2><p>Open local images to browse, zoom and rotate.</p><p>Files stay on your device. The library lasts for this app session.</p></div>}</>}
  </>;
}
export function Media() {
  const { items, add, clear } = useLibrary();
  const [selected, setSelected] = useState<string | null>(null), [playing, setPlaying] = useState(false), [error, setError] = useState("");
  const [repeat, setRepeat] = useState(false), [query, setQuery] = useState("");
  const index = items.findIndex(item => item.id === selected), current = items[index];
  const media = useRef<HTMLVideoElement>(null), autoPlay = useRef(false);
  const video = current && (current.file.type.startsWith("video/") || /\.(mp4|mkv|webm|mov|ogv|avi)$/i.test(current.file.name));
  function choose(id: string, play = true) { autoPlay.current = play; setError(""); setPlaying(false); setSelected(id); }
  function step(delta: number) { const next = items[index + delta]; if (next) choose(next.id); }
  useEffect(() => {
    const pause = () => { if (document.hidden) { media.current?.pause(); autoPlay.current = false; } };
    document.addEventListener("visibilitychange", pause);
    return () => document.removeEventListener("visibilitychange", pause);
  }, []);
  function play() { void media.current?.play().catch(() => setError("Playback could not start. Press Play again or choose a supported file.")); }
  return <><header className="suite-header"><div><span className="suite-kicker">PRESS PLAY</span><h1>Music & Video</h1><p className="muted">Your songs. Your movies. One player.</p></div><OpenFiles accept="audio/*,video/*,.mkv,.flac,.ogg,.mp3,.mp4,.webm,.wav" title="Open media" onFiles={files => { const next = add(files); if (!current && next[0]) choose(next[0].id, false); }} /></header>
    <div className="player-layout"><section className="player-main" aria-label="Player"><div className={`media-stage ${video ? "is-video" : "is-audio"}`}>
      {!video && <div className={`record-art ${playing ? "is-playing" : ""}`} aria-hidden="true"><span>♫</span></div>}
      {current ? <video ref={media} key={current.id} src={current.url} controls playsInline preload="metadata" loop={repeat} aria-label={current.file.name} onPlay={() => setPlaying(true)} onPause={() => setPlaying(false)} onLoadedMetadata={() => { if (autoPlay.current) { autoPlay.current = false; play(); } }} onEnded={() => { setPlaying(false); if (!repeat) step(1); }} onError={() => { setPlaying(false); setError("This file could not be played. Format support depends on your installed media codecs."); }} /> : <p>Open a song or video to begin.</p>}
    </div><div className="now-playing"><span className="suite-kicker">{playing ? "NOW PLAYING" : "READY WHEN YOU ARE"}</span><h2>{current?.file.name ?? "Nothing queued yet"}</h2><p className="muted">{current ? `${video ? "Video" : "Audio"} · ${(current.file.size / 1024 / 1024).toFixed(1)} MB` : "Select local files to build a queue."}</p></div>
    <div className="suite-toolbar player-controls"><button disabled={index <= 0} onClick={() => step(-1)}>← Previous</button><button className="suite-primary" disabled={!current} onClick={() => { if (playing) media.current?.pause(); else play(); }}>{playing ? "Pause" : "Play"}</button><button disabled={index < 0 || index >= items.length - 1} onClick={() => step(1)}>Next →</button><button aria-pressed={repeat} onClick={() => setRepeat(value => !value)}>Repeat one</button></div>{error && <p role="alert">{error}</p>}<p className="muted">Playback pauses when this window is hidden. Your queue lasts for this app session.</p></section>
    <aside className="media-queue"><div className="suite-toolbar"><h2>Up next <small>{items.length}</small></h2><button disabled={!items.length} onClick={() => { media.current?.pause(); setSelected(null); setPlaying(false); setError(""); clear(); }}>Clear</button></div><input type="search" className="library-search" aria-label="Search queue" placeholder="Find in queue…" value={query} onChange={event => setQuery(event.target.value)} /><ol>{items.filter(item => item.file.name.toLowerCase().includes(query.toLowerCase())).map(item => <li key={item.id}><button aria-pressed={item.id === selected} onClick={() => { if (item.id === selected) play(); else choose(item.id); }}><span aria-hidden="true">{item.file.type.startsWith("video/") ? "▷" : "♫"}</span><span>{item.file.name}</span></button></li>)}</ol>{!items.length && <p className="muted">Add music and videos with Open media.</p>}{!!items.length && !items.some(item => item.file.name.toLowerCase().includes(query.toLowerCase())) && <p className="muted">No matching files.</p>}</aside></div>
  </>;
}
