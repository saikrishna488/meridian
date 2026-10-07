import { useEffect, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { mount, errorMessage } from "../../lib/react";
import { useDesktop } from "../../lib/desktop";
import { Displays, Wireless, Sharing } from "./devices";
import { Toggle } from "../../components/Toggle";
const sections = [
  ["General", "⚙", "#88909d"], ["Wallpaper", "▧", "#6c9bca"], ["Appearance", "◐", "#386ceb"], ["Desktop & Dock", "▤", "#278cab"],
  ["Wi-Fi", "⌁", "#297bdf"], ["Bluetooth", "ᛒ", "#297bdf"], ["Displays", "▣", "#725eda"],
  ["Sharing", "⇄", "#498eae"], ["Sound", "♪", "#e55962"], ["Privacy & Security", "⌾", "#488778"],
];
function Settings() {
  useEffect(() => { void hello().catch(console.error); }, []);
  const [section, setSection] = useState("General");
  const [search, setSearch] = useState("");
  const { desktop, setDesktop } = useDesktop();
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  useEffect(() => on("preferences.wallpaper", () => setSection("Wallpaper")), []);
  async function wallpaper(id?: string) {
    if (busy) return; setBusy(true); setError("");
    try { setDesktop(await (id ? request({ method: "desktop.wallpaper_set", params: { id } }) : request({ method: "desktop.wallpaper_choose" }))); }
    catch (e) { setError(errorMessage(e)); } finally { setBusy(false); }
  }
  return <main className="settings-app">
    <aside className="settings-sidebar"><h1>Settings</h1><input aria-label="Search settings" placeholder="Search" value={search} onChange={e => setSearch(e.target.value)} />
      <div className="profile"><span className="profile-avatar">◈</span><div><strong>Meridian</strong><small>Your desktop, your way</small></div></div>
      <nav aria-label="Settings categories">{sections.filter(([name]) => name!.toLowerCase().includes(search.toLowerCase())).map(([name, symbol, color]) => <button key={name} aria-current={section === name ? "page" : undefined} onClick={() => setSection(name!)}><span className="category-icon" style={{ background: color }}>{symbol}</span>{name}</button>)}</nav>
    </aside>
    <section className="settings-content" aria-labelledby="settings-title"><h2 id="settings-title">{section}</h2>
      
      {section === "Displays" ? <Displays /> : section === "Wi-Fi" || section === "Bluetooth" ? <Wireless key={section} kind={section === "Wi-Fi" ? "wifi" : "bluetooth"} /> : section === "Sharing" ? <Sharing /> : section === "Wallpaper" ? <><div className="settings-card">{desktop && <img className="wallpaper-current" src={desktop.wallpaper_uri} alt="Current desktop wallpaper" />}<div className="wallpaper-choices" aria-label="Wallpapers">{desktop?.wallpapers.map(choice => <button className="wallpaper-choice" key={choice.id} aria-pressed={desktop.wallpaper === choice.id} disabled={busy} onClick={() => void wallpaper(choice.id)}><img src={choice.uri} alt="" /><span>{choice.name}</span></button>)}</div><div className="wallpaper-custom"><button disabled={busy} onClick={() => void wallpaper()}>{busy ? "Choosing…" : "Choose custom image…"}</button><small>PNG, JPEG or WebP · up to 20 MB</small></div>{error && <p className="settings-error" role="alert">{error}</p>}</div><p className="section-caption">Original Meridian wallpapers · GPL-3.0-or-later. Your choice is saved automatically.</p></> : section === "General" ? <><div className="settings-card"><div className="settings-row"><span>About Meridian</span><span className="muted">Desktop shell ◈</span></div><div className="settings-row"><span>Software Update</span><span className="muted">Coming soon</span></div><div className="settings-row"><span>Startup Applications</span><span className="muted">Coming soon</span></div></div><p className="section-caption">A simple home for your desktop preferences.</p></> : section === "Appearance" || section === "Desktop & Dock" ? <>
        <p className="section-caption">Installed app icons use a consistent rounded tile. Meridian’s Finder, Settings, Terminal, launcher, and Control Center icons are fixed.</p>
        <div className="settings-card"><div className="settings-row"><label htmlFor="appearance">Appearance</label><select id="appearance" value={desktop?.appearance ?? "light"} disabled={busy} onChange={event => { setBusy(true); setError(""); void request({ method: "desktop.appearance_set", params: { id: event.target.value } }).then(setDesktop).catch(e => setError(errorMessage(e))).finally(() => setBusy(false)); }}><option value="light">Light</option><option value="dark">Dark</option></select></div><div className="settings-row"><span>Accent color</span><div className="accent-swatches" aria-label="Blue accent"><i /><i /><i /><i /></div></div></div>
      </> : <div className="settings-card"><div className="settings-row"><span>{section}</span><Toggle label={section} checked={false} disabled onChange={() => {}} /></div><div className="settings-row"><span>Preferences</span><span className="muted">Coming soon</span></div></div>}
    </section>
  </main>;
}
mount(<Settings />);
