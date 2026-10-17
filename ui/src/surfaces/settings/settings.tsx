import { useEffect, useRef, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { mount, errorMessage } from "../../lib/react";
import { useDesktop } from "../../lib/desktop";
import { useSystemSettings } from "../../lib/settings";
import { Displays, Wireless, Sharing } from "./devices";
import { Icon, type IconName } from "../../components/Icon";
import { SystemSlider } from "../../components/SystemSlider";

const sections: { name: string; icon: IconName; description: string }[] = [
  { name: "Home", icon: "home", description: "Your desktop at a glance." },
  { name: "Wallpaper", icon: "image", description: "Make your workspace feel like home." },
  { name: "Appearance", icon: "appearance", description: "Choose a comfortable look for your desktop and apps." },
  { name: "Desktop & Taskbar", icon: "monitor", description: "Keep your files and favorite apps close at hand." },
  { name: "Wi-Fi", icon: "wifi", description: "Find a network and manage your connection." },
  { name: "Bluetooth", icon: "bluetooth", description: "Connect headphones, keyboards, and other devices." },
  { name: "Displays", icon: "monitor", description: "Arrange your screens and adjust how things look." },
  { name: "Sound", icon: "sound", description: "Adjust the output volume." },
  { name: "Sharing", icon: "share", description: "A preview of future sharing preferences." },
];
const shortcuts = [["Applications", "Super + Space"], ["Control Center", "Super + C"], ["Lock desktop", "Super + L"], ["Open Finder", "Super + E"], ["Settings", "Super + I"], ["Switch windows", "Alt + Tab"]];
function Settings() {
  const [section, setSection] = useState("Home");
  const [search, setSearch] = useState("");
  const { desktop, setDesktop } = useDesktop();
  const { settings } = useSystemSettings();
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => { void hello().catch(console.error); return on("preferences.wallpaper", () => { setSection("Wallpaper"); setSearch(""); }); }, []);
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
  const categories = sections.filter(item => `${item.name} ${item.description}`.toLowerCase().includes(search.toLowerCase()));
  const active = sections.find(item => item.name === section)!;
  return <main className="settings-app">
    <aside className="settings-sidebar"><h1><span className="settings-brand" aria-hidden="true">◈</span>Settings</h1>
      <label className="settings-search"><Icon name="search" width="16" height="16" /><input aria-label="Search settings" placeholder="Find a setting" value={search} onKeyDown={event => { if (event.key === "Enter" && categories[0]) { event.preventDefault(); navigate(categories[0].name); } if (event.key === "Escape") setSearch(""); }} onChange={e => setSearch(e.target.value)} />{search && <button aria-label="Clear settings search" onClick={() => setSearch("")}><Icon name="close" width="14" height="14" /></button>}</label>
      <nav aria-label="Settings categories">{categories.map(({ name, icon }) => <button key={name} aria-label={name} aria-current={section === name ? "page" : undefined} onClick={() => navigate(name)}><Icon name={icon} />{name}{name === "Sharing" && <span className="preview-badge">Preview</span>}</button>)}</nav>
      {!categories.length && <p className="settings-no-results">No settings match “{search}”. Try “display” or “wallpaper”.</p>}
      <div className="sidebar-footnote">Meridian desktop<span>Made for your everyday.</span></div>
    </aside>
    <section className="settings-content" aria-labelledby="settings-title"><div className="settings-page-heading"><h2 id="settings-title" ref={heading} tabIndex={-1}>{section}</h2><p>{active.description}</p></div>
      {section === "Home" ? <>
        <div className="welcome-banner"><div><span className="home-eyebrow">YOUR WORKSPACE</span><h3>Welcome to Meridian</h3><p>A familiar desktop. A little more room to make it yours.</p><button className="primary" onClick={() => navigate("Appearance")}>Make it yours<Icon name="forward" width="16" height="16" /></button></div><div className="desktop-miniature" aria-hidden="true" style={desktop ? { backgroundImage: `url("${desktop.wallpaper_uri}")` } : undefined}><div className="mini-window"><span /><span /><span /></div><div className="mini-taskbar"><i /><i /><i /><i /></div></div></div>
        <h3 className="home-section-title">Start here</h3><div className="settings-quick-links">{["Wallpaper", "Wi-Fi", "Displays", "Sound"].map(name => { const item = sections.find(s => s.name === name)!; return <button key={name} onClick={() => navigate(name)}><span className="quick-link-icon"><Icon name={item.icon} /></span><span><strong>{name}</strong><small>{name === "Wi-Fi" ? settings.wifi === null ? "Connection settings" : settings.wifi ? "Wi-Fi is on" : "Wi-Fi is off" : name === "Sound" ? settings.volume === null ? "Volume controls" : `Volume · ${settings.volume}%` : name === "Displays" ? "Resolution and scaling" : "Choose your background"}</small></span><Icon name="forward" width="16" height="16" /></button>; })}</div>
        <div className="settings-card shortcut-card"><div className="shortcut-heading"><Icon name="keyboard" /><h3>A few helpful shortcuts</h3></div>{shortcuts.map(([name, keys]) => <div className="settings-row" key={name}><span>{name}</span><kbd>{keys}</kbd></div>)}<p className="section-caption card-caption">In a nested development window, use Ctrl + Alt + H for Applications and Ctrl + Alt + C for Control Center.</p></div>
      </> : section === "Displays" ? <Displays /> : section === "Wi-Fi" || section === "Bluetooth" ? <Wireless key={section} kind={section === "Wi-Fi" ? "wifi" : "bluetooth"} /> : section === "Sharing" ? <Sharing /> : section === "Wallpaper" ? <>
        <div className="settings-card">{desktop ? <img className="wallpaper-current" src={desktop.wallpaper_uri} alt="Current desktop wallpaper" /> : <div className="wallpaper-loading" role="status">Loading wallpaper…</div>}<div className="wallpaper-choices" aria-label="Wallpapers">{desktop?.wallpapers.map(choice => <button className="wallpaper-choice" key={choice.id} aria-pressed={desktop.wallpaper === choice.id} disabled={busy} onClick={() => void wallpaper(choice.id)}><img src={choice.uri} alt="" /><span>{choice.name}{desktop.wallpaper === choice.id && <Icon name="check" width="14" height="14" />}</span></button>)}</div><div className="wallpaper-custom"><button disabled={busy} onClick={() => void wallpaper()}><Icon name="image" width="16" height="16" />{busy ? "Updating…" : "Choose custom image…"}</button><small>PNG, JPEG or WebP · up to 20 MB</small></div></div><p className="section-caption">Changes are saved automatically. Original Meridian wallpapers are licensed under GPL-3.0-or-later.</p>
      </> : section === "Appearance" ? <>
        <div className="appearance-options" aria-label="Choose appearance">{(["light", "dark"] as const).map(mode => <button className={`appearance-choice ${mode}`} key={mode} aria-pressed={desktop?.appearance === mode} disabled={busy || !desktop} onClick={() => void appearance(mode)}><span className="appearance-preview" aria-hidden="true"><span className="appearance-preview-sidebar" /><span className="appearance-preview-content"><i /><i /><i /></span></span><span className="appearance-choice-label">{mode === "light" ? "Light" : "Dark"}<span className="choice-check">{desktop?.appearance === mode && <Icon name="check" width="14" height="14" />}</span></span></button>)}</div><p className="section-caption">Meridian windows update immediately. Applications with their own theme preferences may keep their setting.</p>
      </> : section === "Desktop & Taskbar" ? <>
        <div className="settings-card"><div className="settings-row"><Icon name="pin" /><div><strong>Your favorites, one click away</strong><small className="row-detail">Drag an app from Applications to the taskbar, or right-click it and choose Add to Dock. Drag taskbar icons to reorder them.</small></div></div><div className="settings-row"><Icon name="folder" /><div><strong>A desktop that holds your files</strong><small className="row-detail">Your Desktop folder and desktop show the same files. Dropping a file here moves it into that folder. Use Move to Trash to remove a file.</small></div></div><div className="settings-row"><Icon name="settings" /><div><strong>Quick controls on the right</strong><small className="row-detail">Open Control Center from the taskbar for Wi-Fi, Bluetooth, volume, brightness, and appearance.</small></div></div></div>
      </> : section === "Sound" ? <div className="settings-card"><div className="settings-row"><Icon name="sound" /><div><strong>Output volume</strong><small className="row-detail">Adjust the volume for your current output device.</small></div></div><SystemSlider name="volume" label="Volume" value={settings.volume} report={e => setError(errorMessage(e))} />{settings.volume === null && <p className="section-caption card-caption">Volume control is unavailable. Check that an audio device and WirePlumber are running.</p>}</div> : null}
      {error && <p className="settings-error" role="alert">{error}</p>}
    </section>
  </main>;
}
mount(<Settings />);
