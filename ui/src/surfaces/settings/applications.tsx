import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "../../components/ui/button";
import { AppIcon } from "../../components/AppIcon";
import { Icon } from "../../components/Icon";
import { UninstallDialog } from "../../components/UninstallDialog";
import { isBuiltinApp } from "../../lib/apps";
import { errorMessage } from "../../lib/react";
import { on, request } from "../../lib/bridge";
import type { SearchItem } from "../../lib/generated/SearchItem";

export function ApplicationsSettings() {
  const [apps, setApps] = useState<SearchItem[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [uninstall, setUninstall] = useState<string | null>(null);
  const serial = useRef(0);

  async function load() {
    const mine = ++serial.current;
    setLoading(true); setError("");
    try {
      const response = await request({ method: "search.query", params: { query: "", serial: mine } });
      if (mine !== serial.current) return;
      const applicationItems = response.sections
        .filter(section => section.provider === "apps" || section.title.toLowerCase() === "applications")
        .flatMap(section => section.items);
      setApps(Array.from(new Map(applicationItems.map(app => [app.id, app])).values()));
    } catch (e) {
      if (mine === serial.current) setError(errorMessage(e, "Couldn’t load applications"));
    } finally {
      if (mine === serial.current) setLoading(false);
    }
  }

  useEffect(() => {
    void load();
    return on("search.invalidated", () => { void load(); });
  }, []);

  const matching = useMemo(() => {
    const value = query.trim().toLowerCase();
    return value ? apps.filter(app => `${app.title} ${app.subtitle ?? ""}`.toLowerCase().includes(value)) : apps;
  }, [apps, query]);
  const software = apps.find(app => app.id === "org.gnome.Software.desktop");

  function open(app: SearchItem) {
    void request({ method: "search.activate", params: { provider: "apps", item_id: app.id } }).catch(e => setError(errorMessage(e, `Couldn’t open ${app.title}`)));
  }

  function openSoftware() {
    if (!software) return;
    void request({ method: "search.activate", params: { provider: "apps", item_id: software.id } })
      .catch(e => setError(errorMessage(e, `Couldn’t open ${software.title}`)));
  }

  return <>
    <section className="app-store-card" aria-label="App store">
      <span className="app-store-mark"><Icon name="grid" width="22" height="22" /></span>
      <div className="app-store-copy"><strong>Find your next app</strong><p>Discover and install apps with GNOME Software and Flathub.</p></div>
      <Button variant="primary" disabled={loading || !software} title={!loading && !software ? "GNOME Software is not installed" : undefined} onClick={openSoftware}>{loading ? "Finding Software…" : software ? "Open Software" : "Software is not installed"}<Icon name="forward" width="15" height="15" /></Button>
    </section>
    <div className="app-manager-toolbar">
      <label className="app-manager-search"><Icon name="search" width="16" height="16" /><input aria-label="Search installed applications" placeholder="Find an application" value={query} onChange={event => setQuery(event.target.value)} />{query && <Button aria-label="Clear application search" onClick={() => setQuery("")}><Icon name="close" width="14" height="14" /></Button>}</label>
      <Button aria-label="Refresh applications" disabled={loading} onClick={() => void load()}><Icon name="refresh" width="15" height="15" />Refresh</Button>
    </div>
    <div className="app-manager-list-heading"><div><h3>Installed applications</h3><p>{loading ? "Updating your library…" : `${matching.length} ${matching.length === 1 ? "application" : "applications"}${query ? " found" : " available"}`}</p></div><span className="app-manager-list-icon"><Icon name="list" width="17" height="17" /></span></div>
    {loading && !apps.length && <p className="device-empty" role="status">Loading installed applications…</p>}
    {error && <p className="settings-error" role="alert">{error}</p>}
    {!loading && !matching.length && <div className="settings-card app-manager-empty"><Icon name="grid" width="30" height="30" /><strong>{query ? "No matching applications" : "No applications installed"}</strong><p>{query ? "Try a different search." : "Installed applications will appear here."}</p></div>}
    {!!matching.length && <div className="settings-card app-manager-list">{matching.map(app => <div className="settings-row app-manager-row" key={app.id}>
      <AppIcon id={app.id} name={app.title} icon={app.icon} />
      <div className="app-manager-details"><strong>{app.title}</strong><small>{app.subtitle || app.id}</small></div>
      <div className="app-manager-actions"><Button onClick={() => open(app)}>Open</Button><Button disabled={isBuiltinApp(app.id)} onClick={() => setUninstall(app.id)}>Uninstall</Button></div>
    </div>)}</div>}
    {uninstall && <UninstallDialog id={uninstall} close={() => { setUninstall(null); void load(); }} />}
  </>;
}
