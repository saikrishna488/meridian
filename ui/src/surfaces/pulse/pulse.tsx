import { useEffect, useMemo, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { errorMessage, mount } from "../../lib/react";
import type { PulseSnapshot } from "../../lib/generated/PulseSnapshot";
import type { ResourceSample } from "../../lib/generated/ResourceSample";

type Sort = "cpu" | "memory" | "name" | "pid";
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

  return <main className="pulse-app">
    <header className="pulse-header">
      <div><h1>Pulse</h1><p>System activity</p></div>
      <span className="pulse-refresh-note">Updates every 2 seconds</span>
    </header>
    {error && <div className="pulse-error" role="alert">{error}</div>}
    {!data && !error && <p className="pulse-loading" role="status">Reading system activity…</p>}

    <section className="pulse-summary" aria-label="System resource usage">
      <div className="pulse-summary-heading"><h2>System</h2><span>Current usage</span></div>
      <div className="pulse-summary-primary">
        <div><span>CPU</span><strong>{percent(cpu?.percent)}</strong></div>
        <div><span>Memory</span><strong>{percent(memory?.percent)}</strong></div>
        <div><span>Processes</span><strong>{data?.processes.length ?? "—"}</strong></div>
      </div>
      <div className="pulse-resource-list">
        {resources.filter(resource => resource.kind !== "cpu" && resource.kind !== "memory")
          .map(resource => <ResourceRow key={resource.id} resource={resource} />)}
        {memory && <div className="pulse-memory-detail">{resourceDetail(memory)}</div>}
      </div>
    </section>

    <section className="pulse-processes" aria-labelledby="pulse-process-heading">
      <div className="process-toolbar">
        <div><h2 id="pulse-process-heading">Processes</h2><p>{processes.length} shown · CPU {percent(cpu?.percent)} · RAM {percent(memory?.percent)}</p></div>
        <input type="search" aria-label="Search processes" placeholder="Search process or PID" value={query}
          onChange={event => setQuery(event.target.value)} />
      </div>
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
  </main>;
}

mount(<Pulse />);
