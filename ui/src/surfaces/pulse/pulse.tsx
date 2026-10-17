import { useEffect, useState, type CSSProperties } from "react";
import { hello, on, request } from "../../lib/bridge";
import { errorMessage, mount } from "../../lib/react";
import type { PulseSnapshot } from "../../lib/generated/PulseSnapshot";
import type { ResourceSample } from "../../lib/generated/ResourceSample";

type Frame = { time: number; data: PulseSnapshot };
type Sort = "cpu" | "memory" | "name" | "pid";
const colors: Record<string, string> = { cpu: "#5284ed", memory: "#9470da", gpu: "#32a98b", storage: "#dc9650", network: "#458fbd" };
function bytes(value: number | null | undefined) {
  if (value == null) return "—";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let n = Math.max(0, value), i = 0;
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
  return `${n.toFixed(i && n < 100 ? 1 : 0)} ${units[i]}`;
}
function percent(value: number | null | undefined) { return value == null ? "—" : `${value.toFixed(1)}%`; }
function Glyph({ kind }: { kind: string }) {
  return <svg viewBox="0 0 24 24" width="21" height="21" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{kind === "cpu" ? <><rect x="6" y="6" width="12" height="12" rx="2" /><path d="M9 2v4m6-4v4M9 18v4m6-4v4M2 9h4m-4 6h4m12-6h4m-4 6h4" /><rect x="9" y="9" width="6" height="6" rx="1" /></> : kind === "memory" ? <><rect x="3" y="6" width="18" height="11" rx="2" /><path d="M7 10v3m5-3v3m5-3v3M7 17v3m5-3v3m5-3v3" /></> : kind === "gpu" ? <><rect x="3" y="5" width="18" height="13" rx="2" /><circle cx="11" cy="11.5" r="3.5" /><path d="M7 18v3m4-3v3m4-3v3m4-12v5" /></> : kind === "storage" ? <><rect x="4" y="3" width="16" height="18" rx="3" /><circle cx="12" cy="10" r="3" /><path d="m12 10 5 6M7 18h2" /></> : <><path d="M3 8a14 14 0 0 1 18 0M6 12a9 9 0 0 1 12 0m-9 4a4 4 0 0 1 6 0" /><circle cx="12" cy="20" r=".8" /></>}</svg>;
}
function Graph({ frames, resource }: { frames: Frame[]; resource: ResourceSample }) {
  const rates = resource.kind === "network" || resource.kind === "storage";
  const samples = frames.map(f => ({ time: f.time, r: f.data.resources.find(r => r.id === resource.id) }));
  const end = frames.at(-1)?.time ?? Date.now();
  const available = samples.some(s => rates ? s.r?.read_rate != null || s.r?.write_rate != null : s.r?.percent != null);
  const max = rates ? Math.max(1024, ...samples.flatMap(s => [s.r?.read_rate ?? 0, s.r?.write_rate ?? 0])) * 1.15 : 100;
  function line(key: "percent" | "read_rate" | "write_rate") {
    let drawing = false, previousTime = 0;
    return samples.map(s => {
      const value = s.r?.[key];
      if (value == null) { drawing = false; return ""; }
      const x = 600 - (end - s.time) / 60000 * 600;
      const y = 96 - Math.max(0, Math.min(max, value)) / max * 86;
      if (s.time - previousTime > 5000) drawing = false;
      previousTime = s.time;
      const point = `${drawing ? "L" : "M"}${x.toFixed(1)},${y.toFixed(1)}`;
      drawing = true;
      return point + "l0,0";
    }).join(" ");
  }
  const label = rates ? `${resource.name} read and write rates over the last 60 seconds` : `${resource.name} usage over the last 60 seconds`;
  return <div className="pulse-chart"><svg viewBox="0 0 600 106" preserveAspectRatio="none" role="img" aria-label={label}>
    {[10, 53, 96].map(y => <path key={y} className="chart-grid" d={`M0 ${y}H600`} />)}
    {[0, 150, 300, 450, 600].map(x => <path key={x} className="chart-grid" d={`M${x} 10V96`} />)}
    {available && <><path className="chart-line" d={line(rates ? "read_rate" : "percent")} />{rates && <path className="chart-line secondary" d={line("write_rate")} />}</>}
  </svg>{!available && <span className="chart-empty">{resource.kind === "gpu" ? "Usage unavailable" : "Collecting samples…"}</span>}<div className="chart-axis"><span>60 seconds ago</span><span>{rates ? `${bytes(max)}/s` : "100%"} · now</span></div></div>;
}
function Card({ resource: r, frames }: { resource: ResourceSample; frames: Frame[] }) {
  const rates = r.kind === "network" || r.kind === "storage";
  return <article title={r.detail} className={`resource-card${rates || r.total != null ? "" : " no-footer"}`} style={{ "--chart": colors[r.kind] } as CSSProperties}>
    <div className="resource-heading"><span className="resource-icon"><Glyph kind={r.kind} /></span><div><h2>{r.name}</h2></div><strong>{rates ? `${bytes((r.read_rate == null && r.write_rate == null) ? null : (r.read_rate ?? 0) + (r.write_rate ?? 0))}/s` : percent(r.percent)}</strong></div>

    <Graph frames={frames} resource={r} />
    {(rates || r.total != null) && <div className="resource-foot">{rates ? <><span><i className="legend" />{r.kind === "network" ? "Down" : "Read"} <b>{bytes(r.read_rate)}/s</b></span><span><i className="legend secondary" />{r.kind === "network" ? "Up" : "Write"} <b>{bytes(r.write_rate)}/s</b></span>{r.total != null && <span>{bytes(r.total)} capacity</span>}</> : r.total != null ? <><span><b>{bytes(r.used)}</b> used</span><span>{bytes(r.total)} total</span></> : null}</div>}
  </article>;
}
function Pulse() {
  const [tab, setTab] = useState<"overview" | "processes">("overview");
  const [frames, setFrames] = useState<Frame[]>([]);
  const [active, setActive] = useState(true);
  const [visible, setVisible] = useState(!document.hidden);
  const [error, setError] = useState("");
  const [query, setQuery] = useState(""), [sort, setSort] = useState<Sort>("cpu"), [ascending, setAscending] = useState(false);
  useEffect(() => {
    const off = on("pulse.visibility", e => setActive(e.data));
    const visibility = () => setVisible(!document.hidden);
    document.addEventListener("visibilitychange", visibility);
    return () => { off(); document.removeEventListener("visibilitychange", visibility); };
  }, []);
  useEffect(() => {
    if (!active || !visible) return;
    let alive = true, timer: ReturnType<typeof setTimeout> | undefined;
    async function poll() {
      try {
        const data = await request({ method: "pulse.snapshot" });
        if (!alive) return;
        const time = Date.now();
        setFrames(old => [...old.filter(f => time - f.time <= 60000), { time, data }].slice(-31));
        setError("");
      } catch (e) { if (alive) setError(errorMessage(e, "Could not read system activity.")); }
      if (alive) timer = setTimeout(() => void poll(), 2000);
    }
    void hello().then(() => { if (alive) void poll(); }).catch(e => { if (alive) setError(errorMessage(e)); });
    return () => { alive = false; clearTimeout(timer); };
  }, [active, visible]);
  const latest = frames.at(-1), data = latest?.data;
  const cpu = data?.resources.find(r => r.kind === "cpu"), memory = data?.resources.find(r => r.kind === "memory");
  const processes = (data?.processes ?? []).filter(p => `${p.name} ${p.pid}`.toLowerCase().includes(query.toLowerCase())).sort((a, b) => {
    const comparison = sort === "name" ? a.name.localeCompare(b.name) : (a[sort] ?? -1) - (b[sort] ?? -1);
    return (ascending ? comparison : -comparison) || a.pid - b.pid;
  });
  function order(key: Sort) { if (sort === key) setAscending(!ascending); else { setSort(key); setAscending(key === "name" || key === "pid"); } }
  return <main className="pulse-app">
    <div className="pulse-topline"><div className="pulse-tabs" role="tablist" aria-label="System monitor" onKeyDown={e => { if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) { e.preventDefault(); const next = e.key === "Home" ? "overview" : e.key === "End" ? "processes" : tab === "overview" ? "processes" : "overview"; setTab(next); document.getElementById(`tab-${next}`)?.focus(); } }}>{(["overview", "processes"] as const).map(t => <button key={t} id={`tab-${t}`} role="tab" aria-selected={tab === t} aria-controls={`panel-${t}`} tabIndex={tab === t ? 0 : -1} onClick={() => setTab(t)}>{t === "overview" ? "Overview" : "Processes"}{t === "processes" && data && <span>{data.processes.length}</span>}</button>)}</div></div>
    {error && <div className="pulse-error" role="alert">{error}</div>}
    {!data && !error && <p className="pulse-loading" role="status">Reading system activity…</p>}
    {tab === "overview" ? <section id="panel-overview" role="tabpanel" aria-labelledby="tab-overview" className="pulse-overview"><div className="resource-grid">{data?.resources.map(r => <Card key={r.id} resource={r} frames={frames} />)}</div></section> : <section id="panel-processes" role="tabpanel" aria-labelledby="tab-processes" className="pulse-processes"><div className="process-toolbar"><div><p>CPU {percent(cpu?.percent)} <span>·</span> Memory {percent(memory?.percent)}</p></div><input type="search" aria-label="Search processes" placeholder="Search name or PID…" value={query} onChange={e => setQuery(e.target.value)} /></div><div className="process-table-wrap"><table><thead><tr>{([['name', 'Process'], ['pid', 'PID'], ['cpu', 'CPU'], ['memory', 'Memory']] as [Sort, string][]).map(([key, title]) => <th key={key} title={key === "cpu" ? "Share of total system capacity" : key === "memory" ? "Resident RAM" : undefined} scope="col" aria-sort={sort === key ? ascending ? "ascending" : "descending" : "none"}><button onClick={() => order(key)}>{title}{sort === key ? ascending ? " ↑" : " ↓" : ""}</button></th>)}<th scope="col">State</th></tr></thead><tbody>{processes.map(p => <tr key={p.pid}><td title={p.name}><span className="process-dot" />{p.name}</td><td>{p.pid}</td><td>{percent(p.cpu)}</td><td>{bytes(p.memory)}</td><td>{({ R: "Running", S: "Sleeping", D: "Waiting", T: "Stopped", t: "Stopped", Z: "Zombie", I: "Idle" } as Record<string, string>)[p.state] ?? p.state}</td></tr>)}</tbody></table>{!processes.length && <p className="process-empty">{query ? "No matching processes." : "Waiting for processes…"}</p>}</div></section>}

  </main>;
}
mount(<Pulse />);
