import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "../../components/ui/button";
import { Select } from "../../components/Select";
import type { DefaultAppSetting } from "../../lib/generated/DefaultAppSetting";
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
  const [defaults, setDefaults] = useState<DefaultAppSetting[]>([]);
  const [defaultsBusy, setDefaultsBusy] = useState(false);
  const [defaultsLoading, setDefaultsLoading] = useState(true);
  const [defaultsMessage, setDefaultsMessage] = useState("");
  const [defaultsError, setDefaultsError] = useState("");

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
      setApps(Array.from(new Map(applicationItems.filter(app => app.id !== "meridian-terminal.desktop").map(app => [app.id, app])).values()));
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
  useEffect(() => {
    let alive = true;
    void request({ method: "settings.default_apps" })
      .then(value => { if (alive) setDefaults(Array.isArray(value) ? value : []); })
      .catch(e => { if (alive) setDefaultsError(errorMessage(e, "Couldn’t load default applications")); })
      .finally(() => { if (alive) setDefaultsLoading(false); });
    return () => { alive = false; };
  }, []);

  async function changeDefault(purpose: string, app_id: string) {
    if (!app_id || defaultsBusy) return;
    setDefaultsBusy(true); setDefaultsError(""); setDefaultsMessage("");
    try {
      const result = await request({ method: "settings.default_app_set", params: { purpose, app_id } });
      setDefaults(result);
      setDefaultsMessage("Default application updated.");
    } catch (e) {
      setDefaultsError(errorMessage(e, "Couldn’t change default application"));
    } finally {
      setDefaultsBusy(false);
    }
  }

  const purposes = [
    { id: "files", label: "Files", detail: "Open folders and file locations" },
    { id: "browser", label: "Web browser", detail: "Open HTTPS links" },
    { id: "terminal", label: "Terminal", detail: "Open a terminal from Meridian" },
    { id: "mail", label: "Email", detail: "Open email links" },
    { id: "pdf", label: "PDF documents", detail: "Open PDF files" },
    { id: "images", label: "Images", detail: "Open PNG images" },
    { id: "text", label: "Text files", detail: "Open plain text files" },
  ] as const;
  const defaultOptions = (purpose: string) => {
    const builtins = purpose === "files" ? [{ id: "meridian-finder.desktop", title: "Meridian Files" }]
      : [];
    const listed = apps.filter(app => !isBuiltinApp(app.id) || (purpose === "files" && app.id === "meridian-finder.desktop"));
    const options = [...builtins, ...listed];
    return Array.from(new Map(options.map(option => [option.id, option.title])).entries());
  };


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
    <section className="settings-card default-apps-section" aria-labelledby="default-apps-title">
      <div className="default-apps-heading"><h3 id="default-apps-title">Default apps</h3><p>Choose what Meridian and Linux use for common tasks. Installed applications are available to select.</p></div>
      {purposes.map(purpose => {
        const current = defaults.find(item => item.purpose === purpose.id)?.app_id ?? "";
        const options = defaultOptions(purpose.id);
        if (current && !options.some(([id]) => id === current)) options.unshift([current, current]);
        return <div className="settings-row default-apps-row" key={purpose.id}>
          <div><strong>{purpose.label}</strong><small className="row-detail">{purpose.detail}</small></div>
          <Select aria-label={`Default app for ${purpose.label}`} value={current} disabled={defaultsBusy || defaultsLoading || loading}
            onChange={event => { void changeDefault(purpose.id, event.target.value); }}>
            <option value="" disabled>{defaultsLoading ? "Loading…" : "No default selected"}</option>
            {options.map(([id, name]) => <option value={id} key={id}>{name}</option>)}
          </Select>
        </div>;
      })}
      {defaultsError && <p className="settings-error" role="alert">{defaultsError}</p>}
      {defaultsMessage && <p className="startup-message" role="status">{defaultsMessage}</p>}
      <p className="section-caption card-caption">Linux default associations are managed using GIO. Files and Terminal are Meridian-specific choices. Some apps cannot handle every file type.</p>
    </section>
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
