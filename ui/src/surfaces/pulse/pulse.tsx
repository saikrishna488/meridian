import { SearchField } from "../../meridian-ui";
import { useEffect, useMemo, useRef, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { errorMessage, mount } from "../../lib/react";
import type { PulseSnapshot } from "../../lib/generated/PulseSnapshot";
import type { ResourceSample } from "../../lib/generated/ResourceSample";
import type { ProcessSample } from "../../lib/generated/ProcessSample";

type Sort = "cpu" | "memory" | "name" | "pid";
type Section = "overview" | "processes" | "cpu" | "memory" | "gpu" | "storage" | "network";
type Frame = { time: number; resources: ResourceSample[] };
type Series = "percent" | "read_rate" | "write_rate";
const sections: { id: Section; label: string; mark: string }[] = [
  { id: "overview", label: "Overview", mark: "◈" }, { id: "processes", label: "Processes", mark: "≡" },
  { id: "cpu", label: "CPU", mark: "▥" }, { id: "memory", label: "Memory", mark: "▤" },
  { id: "gpu", label: "GPU", mark: "▧" }, { id: "storage", label: "Storage", mark: "▱" },
  { id: "network", label: "Network", mark: "⇅" },
];
const stateNames: Record<string, string> = { R: "Running", S: "Sleeping", D: "Waiting", T: "Stopped", t: "Stopped", Z: "Zombie", I: "Idle" };
const WINDOW = 120_000;
function bytes(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let amount = Math.max(0, value), unit = 0;
  while (amount >= 1024 && unit < units.length - 1) { amount /= 1024; unit++; }
  return `${amount.toFixed(unit && amount < 100 ? 1 : 0)} ${units[unit]}`;
}
function percent(value: number | null | undefined) { return value == null || !Number.isFinite(value) ? "—" : `${value.toFixed(1)}%`; }
function rate(value: number | null | undefined) { return value == null ? "—" : `${bytes(value)}/s`; }
function isRate(resource: ResourceSample) { return resource.kind === "storage" || resource.kind === "network"; }
function value(resource?: ResourceSample) {
  if (!resource) return "—";
  if (!isRate(resource)) return percent(resource.percent);
  // Do not turn a missing direction into a measured zero.
  return resource.read_rate == null || resource.write_rate == null ? "—" : rate(resource.read_rate + resource.write_rate);
}
function detail(resource?: ResourceSample) {
  if (!resource) return "No readings available";
  if (isRate(resource)) return `${resource.kind === "network" ? "Down" : "Read"} ${rate(resource.read_rate)} · ${resource.kind === "network" ? "Up" : "Write"} ${rate(resource.write_rate)}`;
  return resource.used != null && resource.total != null ? `${bytes(resource.used)} of ${bytes(resource.total)}` : resource.detail;
}
function Chart({ history, resource, compact = false }: { history: Frame[]; resource?: ResourceSample; compact?: boolean }) {
  const series: Series[] = resource && isRate(resource) ? ["read_rate", "write_rate"] : ["percent"];
  const end = history[history.length - 1]?.time ?? 0;
  const points = history.map(frame => ({ time: frame.time, sample: frame.resources.find(item => item.id === resource?.id) }));
  const readings = points.flatMap(point => series.map(key => point.sample?.[key]).filter((n): n is number => n != null && Number.isFinite(n)));
  const ceiling = series[0] === "percent" ? 100 : Math.max(1, ...readings);
  const height = compact ? 56 : 180;
  const paths = series.map(key => {
    let previous: number | undefined;
    return points.map(point => {
      const n = point.sample?.[key];
      if (n == null || !Number.isFinite(n)) { previous = undefined; return ""; }
      const command = previous == null || point.time - previous > 6000 ? "M" : "L";
      previous = point.time;
      return `${command}${((point.time - end + WINDOW) / WINDOW * 600).toFixed(1)},${(height - 4 - Math.min(1, Math.max(0, n / ceiling)) * (height - 8)).toFixed(1)}`;
    }).join(" ");
  });
  return <div className={`pm-chart ${compact ? "pm-chart-small" : ""}`}>
    {!compact && <div className="pm-chart-scale"><span>{series[0] === "percent" ? "100%" : rate(ceiling)}</span><span>0</span></div>}
    <svg viewBox={`0 0 600 ${height}`} preserveAspectRatio="none" role="img" aria-label={`${resource?.name ?? "Resource"} activity over the last two minutes; ${value(resource)} currently`}>
      {[.25, .5, .75].map(y => <path key={y} className="pm-grid-line" d={`M0 ${height * y}H600`} />)}
      {paths.map((path, i) => <path key={series[i]} className={`pm-chart-line pm-series-${i}`} d={path} />)}
    </svg>
    {readings.length < 2 && <span className="pm-chart-empty">{readings.length ? "Collecting history…" : "No readings available"}</span>}
    {!compact && <div className="pm-chart-time"><span>2 minutes ago</span><span>Latest sample</span></div>}
  </div>;
}
function Usage({ amount, label }: { amount: number | null; label: string }) {
  return <span className="pm-usage"><span aria-hidden="true" style={{ width: `${Math.max(0, Math.min(100, amount ?? 0))}%` }} /><span>{label}</span></span>;
}
function Pulse() {
  const [data, setData] = useState<PulseSnapshot | null>(null);
  const [history, setHistory] = useState<Frame[]>([]);
  const [updated, setUpdated] = useState<number | null>(null);
  const [section, setSection] = useState<Section>("overview");
  const [active, setActive] = useState(true), [visible, setVisible] = useState(!document.hidden);
  const [paused, setPaused] = useState(false), [error, setError] = useState("");
  const [saveError, setSaveError] = useState("");
  const [query, setQuery] = useState(""), [sort, setSort] = useState<Sort>("cpu"), [ascending, setAscending] = useState(false);
  const [viewReady, setViewReady] = useState(false), [deviceId, setDeviceId] = useState("");
  const [selected, setSelected] = useState<ProcessSample | null>(null);
  const detailsRef = useRef<HTMLElement>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    let alive = true;
    void request({ method: "pulse.view_get" }).then(view => {
      if (!alive) return;
      if (sections.some(item => item.id === view.section)) setSection(view.section as Section);
      if (["cpu", "memory", "name", "pid"].includes(view.sort)) setSort(view.sort as Sort);
      setAscending(view.ascending); setQuery(view.query); setViewReady(true);
    }).catch(() => { if (alive) setViewReady(true); });
    return () => { alive = false; };
  }, []);
  useEffect(() => {
    if (!viewReady) return;
    let alive = true;
    const timer = setTimeout(() => void request({ method: "pulse.view_set", params: { section, sort, ascending, query: query.slice(0, 128) } })
      .then(() => { if (alive) setSaveError(""); }).catch(cause => { if (alive) setSaveError(errorMessage(cause)); }), 200);
    return () => { alive = false; clearTimeout(timer); };
  }, [viewReady, section, sort, ascending, query]);
  useEffect(() => {
    const off = on("pulse.visibility", event => setActive(event.data));
    const visibility = () => setVisible(!document.hidden);
    document.addEventListener("visibilitychange", visibility);
    return () => { off(); document.removeEventListener("visibilitychange", visibility); };
  }, []);
  useEffect(() => {
    if (!active || !visible || paused) return;
    let alive = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    async function poll() {
      try {
        const snapshot = await request({ method: "pulse.snapshot" });
        if (!alive) return;
        const time = Date.now();
        setData(snapshot); setUpdated(time); setError("");
        setHistory(previous => [...previous.filter(frame => frame.time > time - WINDOW), { time, resources: snapshot.resources }].slice(-61));
      } catch (cause) { if (alive) setError(errorMessage(cause, "Could not read system activity.")); }
      if (alive) timer = setTimeout(() => void poll(), 2000);
    }
    void hello().then(() => { if (alive) void poll(); }).catch(cause => { if (alive) setError(errorMessage(cause)); });
    return () => { alive = false; clearTimeout(timer); };
  }, [active, visible, paused]);
  useEffect(() => { if (selected) detailsRef.current?.focus(); }, [selected]);
  const resources = data?.resources ?? [];
  const memory = resources.find(item => item.kind === "memory");
  const processes = useMemo(() => (data?.processes ?? [])
    .filter(process => `${process.name} ${process.pid}`.toLowerCase().includes(query.trim().toLowerCase()))
    .sort((a, b) => {
      const comparison = sort === "name" ? a.name.localeCompare(b.name) : (a[sort] ?? -1) - (b[sort] ?? -1);
      return (ascending ? comparison : -comparison) || a.pid - b.pid;
    }), [data, query, sort, ascending]);
  const chosen = resources.filter(item => item.kind === section);
  const primary = chosen.find(item => item.id === deviceId) ?? chosen[0];
  const heading = sections.find(item => item.id === section)?.label ?? "Overview";
  const currentProcess = selected ? data?.processes.find(item => item.pid === selected.pid && item.name === selected.name) : undefined;
  function navigate(next: Section) { setSection(next); setDeviceId(""); setSelected(null); }
  function inspect(process: ProcessSample, trigger: HTMLButtonElement) { triggerRef.current = trigger; setSelected(process); }
  function closeDetails() { setSelected(null); triggerRef.current?.focus(); }
  function order(column: Sort) {
    if (sort === column) setAscending(current => !current);
    else { setSort(column); setAscending(column === "name" || column === "pid"); }
  }
  function consumers(metric: "cpu" | "memory") {
    const top = [...(data?.processes ?? [])].filter(process => process[metric] != null).sort((a, b) => (b[metric] ?? 0) - (a[metric] ?? 0)).slice(0, 5);
    return <section className="pm-panel"><div className="pm-panel-heading"><h3>Top {metric === "cpu" ? "CPU" : "memory"} consumers</h3>
      <button className="pm-text-button" onClick={() => { setSort(metric); setAscending(false); setQuery(""); navigate("processes"); }}>View all →</button></div>
      {top.length ? <ol className="pm-top-list">{top.map(process => <li key={process.pid}><button onClick={event => inspect(process, event.currentTarget)}><span>{process.name}<small>PID {process.pid}</small></span><strong>{metric === "cpu" ? percent(process.cpu) : bytes(process.memory)}</strong></button></li>)}</ol> : <p className="pm-empty">{data ? "No processes reported." : "Waiting for processes…"}</p>}
    </section>;
  }
  return <main className="pulse-monitor">
    <aside className="pm-sidebar"><div className="pm-brand"><span aria-hidden="true">ϟ</span><div><h1>Pulse</h1><p>System activity</p></div></div>
      <nav aria-label="Pulse pages">{sections.map(item => <button key={item.id} aria-current={section === item.id ? "page" : undefined} onClick={() => navigate(item.id)}>
        <span className="pm-nav-mark" aria-hidden="true">{item.mark}</span><span>{item.label}</span>
        {!["overview", "processes"].includes(item.id) && <small>{value(resources.find(resource => resource.kind === item.id))}</small>}
      </button>)}</nav><p className="pm-sidebar-note">A closer look at your system.<br />Sampled every 2 seconds.</p>
    </aside>
    <div className="pm-main"><header className="pm-header"><div><h2>{heading}</h2><p>{section === "overview" ? "Your system, at a glance." : section === "processes" ? "Find what’s using your resources." : "Activity over the last two minutes."}</p></div>
      <div className="pm-live-controls"><span className="pm-status" data-live={!paused && !error && !!data && active && visible}>{paused ? "Paused" : error ? "Unavailable" : !data ? "Connecting" : !active || !visible ? "Inactive" : "Live"}</span><button className="pm-button" aria-pressed={paused} onClick={() => setPaused(current => !current)}>{paused ? "Resume" : "Pause"}</button></div></header>
      <div className="pm-scroll">
        {error && <p className="pm-alert" role="alert">{error} {data && "Showing the last successful sample."}</p>}
        {saveError && <p className="pm-alert" role="alert">Could not remember this view: {saveError}</p>}
        {!data && !error && <p className="pm-empty" role="status">Reading system activity…</p>}
        {section === "overview" ? <><div className="pm-cards">{sections.filter(item => !["overview", "processes"].includes(item.id)).map(item => {
          const devices = resources.filter(resource => resource.kind === item.id), resource = devices[0];
          return <button key={item.id} className="pm-resource-card" onClick={() => navigate(item.id)}><span className="pm-card-heading">{item.label}<span aria-hidden="true">↗</span></span><strong>{value(resource)}</strong><span className="pm-card-detail">{detail(resource)}</span><Chart history={history} resource={resource} compact /><span className="pm-card-foot">{resource?.name ?? "No device reported"}{devices.length > 1 ? ` · +${devices.length - 1} more` : ""}</span></button>;
        })}</div><div className="pm-consumers">{consumers("cpu")}{consumers("memory")}</div></>
        : section === "processes" ? <section className="pm-panel pm-process-panel" aria-label="Processes"><div className="pm-process-toolbar"><div><h3>All processes</h3><p>{processes.length} shown{data ? ` / ${data.processes.length} total` : ""}</p></div><SearchField aria-label="Search processes by name or PID" placeholder="Search name or PID" value={query} maxLength={128} onValueChange={setQuery} /></div>
          <div className="pm-table-scroll" tabIndex={0} role="region" aria-label="Process table, scroll horizontally for more columns"><table className="pm-table"><thead><tr>{([{ key: "name", label: "Process" }, { key: "pid", label: "PID" }, { key: "cpu", label: "CPU" }, { key: "memory", label: "Memory" }] as { key: Sort; label: string }[]).map(column => <th key={column.key} scope="col" aria-sort={sort === column.key ? ascending ? "ascending" : "descending" : "none"}><button onClick={() => order(column.key)}>{column.label}<span aria-hidden="true">{sort === column.key ? ascending ? " ↑" : " ↓" : ""}</span></button></th>)}<th scope="col">State</th></tr></thead>
          <tbody>{processes.map(process => <tr key={process.pid} data-selected={selected?.pid === process.pid}><td><button className="pm-process-name" onClick={event => inspect(process, event.currentTarget)}>{process.name}</button></td><td>{process.pid}</td><td><Usage amount={process.cpu} label={percent(process.cpu)} /></td><td><Usage amount={memory?.total ? process.memory / memory.total * 100 : null} label={bytes(process.memory)} /></td><td>{stateNames[process.state] ?? process.state}</td></tr>)}</tbody></table></div>
          {!processes.length && <p className="pm-empty">{query ? "No matching processes. Try another name or PID." : data ? "No processes reported." : "Waiting for processes…"}</p>}
          <p className="pm-table-note">CPU is a share of total machine capacity. Memory is resident RAM.</p></section>
        : <section className="pm-panel pm-resource-detail"><div className="pm-panel-heading"><div><h3>{primary?.name ?? heading}</h3><p>{chosen.length} {chosen.length === 1 ? "device" : "devices"} reported</p></div>{chosen.length > 1 && <label className="pm-device-label">Device<select value={primary?.id ?? ""} onChange={event => setDeviceId(event.target.value)}>{chosen.map(resource => <option key={resource.id} value={resource.id}>{resource.name}</option>)}</select></label>}</div>
          {primary ? <><div className="pm-resource-hero"><strong>{value(primary)}</strong><span>{isRate(primary) ? "Combined throughput" : "Current usage"}</span></div>
            {isRate(primary) && <div className="pm-legend"><span>━ {section === "network" ? "Download" : "Read"} {rate(primary.read_rate)}</span><span>┄ {section === "network" ? "Upload" : "Write"} {rate(primary.write_rate)}</span></div>}
            <Chart history={history} resource={primary} /><dl className="pm-resource-stats">{primary.used != null && <div><dt>Used</dt><dd>{bytes(primary.used)}</dd></div>}{primary.total != null && <div><dt>Total</dt><dd>{bytes(primary.total)}</dd></div>}<div><dt>Device</dt><dd>{primary.name}</dd></div></dl>
            {primary.detail && <p className="pm-resource-note">{primary.detail}</p>}
            {["cpu", "memory"].includes(section) ? <button className="pm-button" onClick={() => { setSort(section === "memory" ? "memory" : "cpu"); setAscending(false); setQuery(""); navigate("processes"); }}>Explore processes →</button> : <p className="pm-resource-note">Per-process {heading.toLowerCase()} activity isn’t available yet.</p>}
          </> : <p className="pm-empty">{data ? `No ${heading.toLowerCase()} devices reported.` : "Waiting for metrics…"}</p>}</section>}
        <footer className="pm-footer"><span>{updated ? `Last updated ${new Date(updated).toLocaleTimeString()}` : "Waiting for first sample"}</span><span>History is kept while Pulse is open · — unavailable</span></footer>
      </div>
    </div>
    {selected && <aside className="pm-inspector" ref={detailsRef} tabIndex={-1} aria-label="Process details" onKeyDown={event => { if (event.key === "Escape") { event.stopPropagation(); closeDetails(); } }}><div className="pm-panel-heading"><h3>Process details</h3><button className="pm-button" onClick={closeDetails} aria-label="Close process details">✕</button></div><h4>{selected.name}</h4><p className="pm-resource-note">PID {selected.pid}</p>{currentProcess ? <dl><div><dt>CPU</dt><dd>{percent(currentProcess.cpu)}</dd></div><div><dt>Resident memory</dt><dd>{bytes(currentProcess.memory)}</dd></div><div><dt>RAM share</dt><dd>{memory?.total ? percent(currentProcess.memory / memory.total * 100) : "—"}</dd></div><div><dt>State</dt><dd>{stateNames[currentProcess.state] ?? currentProcess.state}</dd></div></dl> : <p className="pm-empty">This process is no longer in the latest sample.</p>}<p className="pm-resource-note">{paused ? "Updates paused." : "Updates with each system sample."}</p></aside>}
  </main>;
}
mount(<Pulse />);
