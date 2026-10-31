import { useEffect, useMemo, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { errorMessage, mount } from "../../lib/react";
import type { PulseSnapshot } from "../../lib/generated/PulseSnapshot";
import type { ResourceSample } from "../../lib/generated/ResourceSample";

type Sort = "cpu" | "memory" | "name" | "pid";
type Section = "overview" | "cpu" | "memory" | "gpu" | "storage" | "network";
const sections: { id: Section; label: string }[] = [
  { id: "overview", label: "Overview" }, { id: "cpu", label: "CPU" },
  { id: "memory", label: "Memory" }, { id: "gpu", label: "GPU" },
  { id: "storage", label: "Storage" }, { id: "network", label: "Network" },
];
const stateNames: Record<string, string> = {
  R: "Running", S: "Sleeping", D: "Waiting", T: "Stopped",
  t: "Stopped", Z: "Zombie", I: "Idle",
};

function bytes(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let amount = Math.max(0, value), unit = 0;
  while (amount >= 1024 && unit < units.length - 1) { amount /= 1024; unit++; }
  return `${amount.toFixed(unit && amount < 100 ? 1 : 0)} ${units[unit]}`;
}

function percent(value: number | null | undefined): string {
  return value == null || !Number.isFinite(value) ? "—" : `${value.toFixed(1)}%`;
}

function resourceValue(resource: ResourceSample): string {
  if (resource.kind === "storage" || resource.kind === "network") {
    return resource.read_rate == null && resource.write_rate == null
      ? "—"
      : `${bytes((resource.read_rate ?? 0) + (resource.write_rate ?? 0))}/s`;
  }
  return percent(resource.percent);
}

function resourceDetail(resource: ResourceSample): string {
  if (resource.kind === "storage" || resource.kind === "network") {
    const read = resource.kind === "network" ? "Down" : "Read";
    const write = resource.kind === "network" ? "Up" : "Write";
    return `${read} ${bytes(resource.read_rate)}/s · ${write} ${bytes(resource.write_rate)}/s`;
  }
  if (resource.used != null && resource.total != null) {
    return `${bytes(resource.used)} of ${bytes(resource.total)}`;
  }
  return resource.percent == null ? resource.detail : "";
}

function ResourceRow({ resource }: { resource: ResourceSample }) {
  const label = resource.kind === "storage" ? `Disk · ${resource.name}`
    : resource.kind === "network" ? `Network · ${resource.name}` : resource.name;
  const detail = resourceDetail(resource);
  return <div className="pulse-resource-row" title={resource.detail}>
    <span className="pulse-resource-name">{label}</span>
    <strong className="pulse-resource-value">{resourceValue(resource)}</strong>
    {detail && <span className="pulse-resource-detail">{detail}</span>}
  </div>;
}

function Pulse() {
  const [data, setData] = useState<PulseSnapshot | null>(null);
  const [section, setSection] = useState<Section>("overview");
  const [active, setActive] = useState(true);
  const [visible, setVisible] = useState(!document.hidden);
  const [error, setError] = useState("");
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<Sort>("cpu");
  const [ascending, setAscending] = useState(false);

  useEffect(() => {
    const off = on("pulse.visibility", event => setActive(event.data));
    const visibility = () => setVisible(!document.hidden);
    document.addEventListener("visibilitychange", visibility);
    return () => { off(); document.removeEventListener("visibilitychange", visibility); };
  }, []);

  useEffect(() => {
    if (!active || !visible) return;
    let alive = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    async function poll() {
      try {
        const snapshot = await request({ method: "pulse.snapshot" });
        if (!alive) return;
        setData(snapshot);
        setError("");
      } catch (cause) {
        if (alive) setError(errorMessage(cause, "Could not read system activity."));
      }
      if (alive) timer = setTimeout(() => void poll(), 2000);
    }
    void hello().then(() => { if (alive) void poll(); })
      .catch(cause => { if (alive) setError(errorMessage(cause)); });
    return () => { alive = false; clearTimeout(timer); };
  }, [active, visible]);

  const resources = data?.resources ?? [];
  const cpu = resources.find(resource => resource.kind === "cpu");
  const memory = resources.find(resource => resource.kind === "memory");

  const processes = useMemo(() => (data?.processes ?? [])
    .filter(process => `${process.name} ${process.pid}`.toLowerCase().includes(query.trim().toLowerCase()))
    .sort((a, b) => {
      const comparison = sort === "name"
        ? a.name.localeCompare(b.name)
        : (a[sort] ?? -1) - (b[sort] ?? -1);
      return (ascending ? comparison : -comparison) || a.pid - b.pid;
    }), [data, query, sort, ascending]);

  function order(column: Sort) {
    if (sort === column) setAscending(current => !current);
    else { setSort(column); setAscending(column === "name" || column === "pid"); }
  }

  const columns: { key: Sort; label: string; detail?: string }[] = [
    { key: "name", label: "Process Name" },
    { key: "pid", label: "PID" },
    { key: "cpu", label: "CPU", detail: "Percentage of total logical CPU capacity" },
    { key: "memory", label: "Memory", detail: "Resident RAM in bytes" },
  ];

  const chosen = resources.filter(resource => resource.kind === section);
  const primary = chosen[0];
  const heading = sections.find(item => item.id === section)?.label ?? "Overview";
  const metric = section === "cpu" || section === "memory";
  const processDescription = section === "gpu" || section === "storage" || section === "network"
    ? "Process GPU, disk, and network attribution is not collected yet. CPU and RAM columns show system process usage."
    : section === "memory" ? "Processes ordered by resident memory" : "Processes ordered by system CPU share";

  function navigate(next: Section) {
    setSection(next);
    setQuery("");
    if (next === "memory") { setSort("memory"); setAscending(false); }
    else if (next === "cpu" || next === "overview") { setSort("cpu"); setAscending(false); }
  }

  return <main className="pulse-app">
    <aside className="pulse-sidebar" aria-label="Pulse navigation">
      <div className="pulse-brand"><h1>Pulse</h1><span>System Monitor</span></div>
      <nav aria-label="Resources">
        {sections.map(item => <button key={item.id} type="button"
          aria-current={section === item.id ? "page" : undefined}
          onClick={() => navigate(item.id)}>
          <span>{item.label}</span>
          {item.id !== "overview" && <small>{resourceValue(
            resources.find(resource => resource.kind === item.id)
            ?? { kind: item.id, percent: null, read_rate: null, write_rate: null } as ResourceSample
          )}</small>}
        </button>)}
      </nav>
      <div className="pulse-sidebar-foot">Updates every 2 seconds</div>
    </aside>

    <div className="pulse-main">
      <header className="pulse-page-header">
        <div><span className="pulse-eyebrow">SYSTEM ACTIVITY</span><h2>{heading}</h2></div>
        <span className="pulse-live">Live</span>
      </header>
      {error && <div className="pulse-error" role="alert">{error}</div>}
      {!data && !error && <p className="pulse-loading" role="status">Reading system activity…</p>}

      <div className="pulse-content" key={section}>
        {section === "overview" ? <section className="pulse-overview" aria-label="System overview">
          <div className="pulse-section-heading"><h3>System overview</h3><span>Current usage</span></div>
          <div className="pulse-overview-metrics">
            <div><span>CPU</span><strong>{percent(cpu?.percent)}</strong></div>
            <div><span>Memory</span><strong>{percent(memory?.percent)}</strong></div>
            <div><span>Processes</span><strong>{data?.processes.length ?? "—"}</strong></div>
          </div>
          <div className="pulse-resource-list">
            {resources.filter(resource => resource.kind !== "cpu" && resource.kind !== "memory")
              .map(resource => <ResourceRow key={resource.id} resource={resource} />)}
          </div>
        </section> : <section className="pulse-detail" aria-label={`${heading} resource details`}>
          <div className="pulse-section-heading"><h3>{heading} usage</h3><span>{chosen.length > 1 ? `${chosen.length} devices` : "Current"}</span></div>
          {metric ? <div className="pulse-detail-hero">
            <strong>{percent(primary?.percent)}</strong>
            <span>{primary ? resourceDetail(primary) || primary.detail : "Waiting for metrics…"}</span>
          </div> : chosen.length ? <div className="pulse-device-list">
            {chosen.map(resource => <ResourceRow key={resource.id} resource={resource} />)}
          </div> : <p className="pulse-no-device">No {heading.toLowerCase()} devices reported.</p>}
        </section>}

        <section className="pulse-processes" aria-labelledby="pulse-process-heading">
          <div className="process-toolbar">
            <div><h3 id="pulse-process-heading">{section === "overview" ? "Processes" : section === "cpu" ? "Processes using CPU" : section === "memory" ? "Processes using memory" : "Processes"}</h3>
              <p>{processes.length} shown · CPU {percent(cpu?.percent)} · Memory {percent(memory?.percent)}</p>
            </div>
            <input type="search" aria-label="Search processes" placeholder="Search name or PID"
              value={query} onChange={event => setQuery(event.target.value)} />
          </div>
          {section !== "overview" && <p className="pulse-process-caption">{processDescription}</p>}
          <div className="process-table-wrap">
            <table>
              <thead><tr>{columns.map(column => <th key={column.key} scope="col" title={column.detail}
                aria-sort={sort === column.key ? ascending ? "ascending" : "descending" : "none"}>
                <button onClick={() => order(column.key)}>{column.label}<span aria-hidden="true">{sort === column.key ? ascending ? " ↑" : " ↓" : ""}</span></button>
              </th>)}<th scope="col">RAM %</th><th scope="col">State</th></tr></thead>
              <tbody>{processes.map(process => <tr key={process.pid}>
                <td className="process-name" title={process.name}>{process.name}</td>
                <td>{process.pid}</td>
                <td>{percent(process.cpu)}</td>
                <td>{bytes(process.memory)}</td>
                <td>{memory?.total && memory.total > 0 ? percent(process.memory / memory.total * 100) : "—"}</td>
                <td>{stateNames[process.state] ?? process.state}</td>
              </tr>)}</tbody>
            </table>
            {!processes.length && <p className="process-empty">{query ? "No matching processes." : "Waiting for processes…"}</p>}
          </div>
        </section>
      </div>
    </div>
  </main>;
}

mount(<Pulse />);
