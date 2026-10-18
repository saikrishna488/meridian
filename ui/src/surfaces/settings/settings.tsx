import { SearchSettings } from "./search";
import { Select } from "../../components/Select";
import { useEffect, useRef, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { mount, errorMessage } from "../../lib/react";
import { useDesktop } from "../../lib/desktop";
import { useSystemSettings } from "../../lib/settings";
import { General } from "./general";
import { Startup } from "./startup";
import { Sound } from "./sound";
import { Power } from "./power";
import { DeviceManager } from "./device-manager";
import { MeridianLogo } from "../../components/MeridianLogo";
import { Users } from "./users";
import { Displays, Wireless } from "./devices";
import { Icon, type IconName } from "../../components/Icon";

const sections: { name: string; icon: IconName; description: string }[] = [
  { name: "General", icon: "info", description: "System information, Meridian version and updates." },
  { name: "Home", icon: "home", description: "Your desktop at a glance." },
  { name: "Wallpaper", icon: "image", description: "Make your workspace feel like home." },
  { name: "Appearance", icon: "appearance", description: "Choose a comfortable look for your desktop and apps." },
  { name: "Desktop & Taskbar", icon: "monitor", description: "Keep your files and favorite apps close at hand." },
  { name: "Startup Apps", icon: "grid", description: "Choose which applications open when you sign in." },
  { name: "Power & Lock", icon: "power", description: "Battery status and automatic screen locking." },
  { name: "Wi-Fi", icon: "wifi", description: "Find a network and manage your connection." },
  { name: "Bluetooth", icon: "bluetooth", description: "Connect headphones, keyboards, and other devices." },
  { name: "Displays", icon: "monitor", description: "Arrange your screens and adjust how things look." },
  { name: "Device Manager", icon: "settings", description: "Inspect and manage kernel drivers." },
  { name: "Users", icon: "home", description: "Manage local accounts and administrator access." },
  { name: "Search", icon: "search", description: "Choose launcher search providers." },
  { name: "Sound", icon: "sound", description: "Choose speakers and microphones, and set comfortable levels." },
];
const navigationGroups = [
  { name: "Workspace", sections: ["Home", "Appearance", "Wallpaper", "Desktop & Taskbar", "Startup Apps", "Search"] },
  { name: "Devices & connections", sections: ["Wi-Fi", "Bluetooth", "Displays", "Sound"] },
  { name: "System", sections: ["Power & Lock", "General", "Device Manager", "Users"] },
];
const shortcuts = [["Screenshot desktop", "Print Screen"], ["Screenshot area", "Super + Shift + S / Shift + Print Screen"],["Applications", "Super + Space"], ["Control Center", "Super + C"], ["Lock desktop", "Super + L"], ["Open Orbit", "Super + E"], ["Settings", "Super + I"], ["Switch windows", "Alt + Tab"], ["Minimize window", "Super + ↓"], ["Maximize / restore", "Super + ↑"], ["Snap left / right", "Super + ← / →"], ["Window menu", "Alt + Space"]];
const searchTerms: Record<string, string> = {
  "Device Manager": "hardware wifi adapter bluetooth printer storage disk driver usb pci",
  General: "about version update github system processor memory kernel computer",
  Users: "account password administrator login delete",
  Home: "shortcuts keyboard minimize maximize snap window controls",
  Appearance: "theme dark light color titlebar font typography",
  "Desktop & Taskbar": "dock pin favorites icons files",
  "Startup Apps": "login autostart launch boot sign in background applications",
  "Power & Lock": "battery power charging energy sleep screen lock idle timeout standby",
  "Wi-Fi": "wifi internet wireless network",
  Bluetooth: "pair headphones mouse keyboard",
  Displays: "monitor screen resolution scaling rotation brightness",
  Sound: "audio volume speaker mute microphone input output headphones recording",
};
function Settings() {
  const [section, setSection] = useState("Home");
  const [search, setSearch] = useState("");
  const { desktop, setDesktop } = useDesktop();
  const { settings } = useSystemSettings();
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    const offWallpaper = on("preferences.wallpaper", () => { setSection("Wallpaper"); setSearch(""); });
    const offPage = on("preferences.page", event => { const names = { home: "Home", sound: "Sound", power: "Power & Lock", "startup-apps": "Startup Apps", wifi: "Wi-Fi", bluetooth: "Bluetooth", displays: "Displays", appearance: "Appearance", wallpaper: "Wallpaper", desktop: "Desktop & Taskbar", general: "General", users: "Users", "device-manager": "Device Manager", search: "Search" }; setSection(names[event.data.page]); setSearch(""); setError(""); });
    void hello().catch(console.error);
    return () => { offWallpaper(); offPage(); };
  }, []);
  useEffect(() => { document.querySelector<HTMLElement>('.settings-sidebar [aria-current="page"]')?.scrollIntoView({ block: "nearest", inline: "nearest" }); }, [section, search]);
  function navigate(name: string) { setSection(name); setError(""); heading.current?.focus(); }
  async function wallpaper(id?: string) {
    if (busy) return; setBusy(true); setError("");
    try { setDesktop(await (id ? request({ method: "desktop.wallpaper_set", params: { id } }) : request({ method: "desktop.wallpaper_choose" }))); }
    catch (e) { setError(errorMessage(e)); } finally { setBusy(false); }
  }
  async function appearance(id: "light" | "dark") {
    if (busy || !desktop) return; setBusy(true); setError("");
    try { setDesktop(await request({ method: "desktop.appearance_set", params: { id } })); }
    catch (e) { setError(errorMessage(e)); } finally { setBusy(false); }
  }
  async function font(id: string) {
    if (busy) return; setBusy(true); setError("");
    try { setDesktop(await request({ method: "desktop.font_set", params: { id } })); }
    catch (e) { setError(errorMessage(e)); } finally { setBusy(false); }
  }
  const categories = sections.filter(item => `${item.name} ${item.description} ${searchTerms[item.name] ?? ""}`.toLowerCase().includes(search.trim().toLowerCase()));
  return <main className="settings-app">
    <aside className="settings-sidebar"><div className="settings-identity"><span className="settings-brand"><MeridianLogo width="24" height="24" /></span><div><h1>Settings</h1><small>Make Meridian yours</small></div></div>
      <label className="settings-search"><Icon name="search" width="16" height="16" /><input aria-label="Search settings" placeholder="Find a setting" value={search} onKeyDown={event => { if (event.key === "Enter" && categories[0]) { event.preventDefault(); navigate(categories[0].name); } if (event.key === "Escape") setSearch(""); }} onChange={e => setSearch(e.target.value)} />{search && <button aria-label="Clear settings search" onClick={() => setSearch("")}><Icon name="close" width="14" height="14" /></button>}</label>
      <nav aria-label="Settings categories">{navigationGroups.map(group => { const matching = group.sections.flatMap(name => categories.filter(item => item.name === name)); return matching.length > 0 && <div className="settings-nav-group" key={group.name}><h2>{group.name}</h2>{matching.map(({ name, icon }) => <button key={name} aria-label={name} aria-current={section === name ? "page" : undefined} onClick={() => navigate(name)}><Icon name={icon} /><span>{name}</span></button>)}</div>; })}</nav>
      {!categories.length && <p className="settings-no-results">No settings match “{search}”. Try “display” or “wallpaper”.</p>}

    </aside>
    <section className="settings-content" aria-labelledby="settings-title"><div className="settings-page-heading"><div className="settings-heading-copy"><span className="settings-breadcrumb">Settings <Icon name="forward" width="11" height="11" />{section}</span><h2 id="settings-title" ref={heading} tabIndex={-1}>{section}</h2><p>{sections.find(item => item.name === section)?.description}</p></div><button className="settings-theme-button" aria-label={`Switch to ${desktop?.appearance === "dark" ? "light" : "dark"} mode`} disabled={busy || !desktop} onClick={() => void appearance(desktop?.appearance === "dark" ? "light" : "dark")} title="Change appearance"><Icon name="appearance" width="18" height="18" /></button></div>
      {section === "Home" ? <>
        <div className="workspace-preview"><div className="workspace-copy"><span className="workspace-eyebrow">YOUR WORKSPACE</span><h3>Welcome to Meridian.</h3><p>A place for your files, your favorite apps, and a desktop that feels like home.</p><button onClick={() => navigate("Appearance")}><Icon name="appearance" width="16" height="16" />Personalize appearance<Icon name="forward" width="14" height="14" /></button></div><button className="workspace-wallpaper" aria-label="Change desktop wallpaper" onClick={() => navigate("Wallpaper")}>{desktop && <img src={desktop.wallpaper_uri} alt="" />}<span><Icon name="image" width="14" height="14" />Change wallpaper</span></button></div>
        <h3 className="home-section-title">Everyday essentials</h3>
        <div className="settings-quick-links">{["Wallpaper", "Wi-Fi", "Displays", "Sound", "Startup Apps", "General"].map(name => { const item = sections.find(s => s.name === name)!; return <button key={name} onClick={() => navigate(name)}><span className="quick-link-icon"><Icon name={item.icon} /></span><span><strong>{name}</strong><small>{name === "Wi-Fi" ? settings.wifi === null ? "Connection settings" : settings.wifi ? "Wi-Fi is on" : "Wi-Fi is off" : name === "Sound" ? settings.volume === null ? "Volume controls" : `Volume · ${settings.volume}%` : name === "Displays" ? "Resolution and scaling" : name === "Startup Apps" ? "Ready when you sign in" : name === "General" ? "System information and updates" : "Choose your background"}</small></span><Icon name="forward" width="16" height="16" /></button>; })}</div>
        <div className="settings-card shortcut-card"><div className="shortcut-heading"><Icon name="keyboard" /><h3>Keyboard shortcuts</h3></div>{shortcuts.map(([name, keys]) => <div className="settings-row" key={name}><span>{name}</span><kbd>{keys}</kbd></div>)}<p className="section-caption card-caption">In a nested development window, use Ctrl + Alt + H for Applications and Ctrl + Alt + C for Control Center.</p></div>
      </> : section === "Search" ? <SearchSettings /> : section === "Power & Lock" ? <Power /> : section === "Startup Apps" ? <Startup /> : section === "General" ? <General /> : section === "Device Manager" ? <DeviceManager navigate={navigate} /> : section === "Users" ? <Users /> : section === "Displays" ? <Displays /> : section === "Wi-Fi" || section === "Bluetooth" ? <Wireless key={section} kind={section === "Wi-Fi" ? "wifi" : "bluetooth"} /> : section === "Wallpaper" ? <>
        <div className="settings-card">{desktop ? <img className="wallpaper-current" src={desktop.wallpaper_uri} alt="Current desktop wallpaper" /> : <div className="wallpaper-loading" role="status">Loading wallpaper…</div>}<div className="wallpaper-choices" aria-label="Wallpapers">{desktop?.wallpapers.map(choice => <button className="wallpaper-choice" key={choice.id} aria-pressed={desktop.wallpaper === choice.id} disabled={busy} onClick={() => void wallpaper(choice.id)}><img src={choice.uri} alt="" /><span>{choice.name}{desktop.wallpaper === choice.id && <Icon name="check" width="14" height="14" />}</span></button>)}</div><div className="wallpaper-custom"><button disabled={busy} onClick={() => void wallpaper()}><Icon name="image" width="16" height="16" />{busy ? "Updating…" : "Choose custom image…"}</button><small>PNG, JPEG or WebP · up to 20 MB</small></div></div>
      </> : section === "Appearance" ? <>
        <div className="appearance-options" aria-label="Choose appearance">{(["light", "dark"] as const).map(mode => <button className={`appearance-choice ${mode}`} key={mode} aria-pressed={desktop?.appearance === mode} disabled={busy || !desktop} onClick={() => void appearance(mode)}><span className="appearance-preview" aria-hidden="true"><span className="appearance-preview-sidebar" /><span className="appearance-preview-content"><i /><i /><i /></span></span><span className="appearance-choice-label">{mode === "light" ? "Light" : "Dark"}<span className="choice-check">{desktop?.appearance === mode && <Icon name="check" width="14" height="14" />}</span></span></button>)}</div><div className="settings-card"><div className="settings-row"><label htmlFor="system-font">Font</label><Select id="system-font" aria-label="System font" value={desktop?.font ?? "Inter"} disabled={busy || !desktop} onChange={e => void font(e.target.value)}>{["Inter", "Noto Sans", "DejaVu Sans", "Liberation Sans"].map(name => <option key={name} value={name}>{name}</option>)}</Select></div></div>
      </> : section === "Desktop & Taskbar" ? <>
        <div className="settings-card"><div className="settings-row"><Icon name="pin" /><div><strong>Pinned apps</strong><small className="row-detail">Drag apps to the taskbar to pin them. Drag pinned icons to reorder.</small></div></div><div className="settings-row"><Icon name="folder" /><div><strong>Desktop files</strong><small className="row-detail">Files on your desktop are stored in the Desktop folder.</small></div></div><div className="settings-row"><Icon name="settings" /><div><strong>Control Center</strong><small className="row-detail">Wi-Fi, Bluetooth, volume and brightness.</small></div></div></div>
      </> : section === "Sound" ? <Sound /> : null}
      {error && <p className="settings-error" role="alert">{error}</p>}
    </section>
  </main>;
}
mount(<Settings />);
