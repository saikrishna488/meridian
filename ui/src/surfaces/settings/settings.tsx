import { Slider } from "../../components/ui/slider";
import { Button } from "../../components/ui/button";
import { SearchSettings } from "./search";
import { ApplicationsSettings } from "./applications";
import { Select } from "../../components/Select";
import { useEffect, useRef, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { mount, errorMessage } from "../../lib/react";
import { useDesktop } from "../../lib/desktop";
import { useSystemSettings } from "../../lib/settings";
import { getFontEffects, saveFontEffects, subscribeFontEffects, type FontEffects } from "../../lib/appearance";
import { General } from "./general";
import { InputSettings, ShortcutSettings, NotificationSettings, DateTimeSettings, PrinterSettings, PrivacySettings } from "./essentials";
import { Startup } from "./startup";
import { Sound } from "./sound";
import { Power } from "./power";
import { Storage } from "./storage";
import { ConnectedDevices } from "./connected-devices";
import { MeridianLogo } from "../../components/MeridianLogo";
import { Users } from "./users";
import { Displays, Wireless } from "./devices";
import { Icon, type IconName } from "../../components/Icon";

const sections: { name: string; icon: IconName; description: string }[] = [
  { name: "Keyboard", icon: "keyboard", description: "Key repeat and typing options." },
  { name: "Mouse & Touchpad", icon: "settings", description: "Pointer speed, scrolling, and touchpad gestures." },
  { name: "Keyboard Shortcuts", icon: "keyboard", description: "Review Meridian's keyboard shortcuts." },
  { name: "Date & Time", icon: "info", description: "Clock and time zone information." },
  { name: "Printers & Scanners", icon: "settings", description: "Printer and scanner integration status." },
  { name: "Notifications", icon: "info", description: "Manage notification interruptions." },
  { name: "Privacy & Security", icon: "settings", description: "Review security and permission availability." },
  { name: "General", icon: "info", description: "System information, Meridian version and updates." },
  { name: "Home", icon: "home", description: "Your desktop at a glance." },
  { name: "Wallpaper", icon: "image", description: "Make your workspace feel like home." },
  { name: "Appearance", icon: "appearance", description: "Choose a comfortable look for your desktop and apps." },
  { name: "Startup Apps", icon: "grid", description: "Choose which applications open when you sign in." },
  { name: "Applications", icon: "grid", description: "Manage installed apps and choose default handlers for files, links, and more." },
  { name: "Power & Lock", icon: "power", description: "Battery status and automatic screen locking." },
  { name: "Wi-Fi", icon: "wifi", description: "Find a network and manage your connection." },
  { name: "Bluetooth", icon: "bluetooth", description: "Connect headphones, keyboards, and other devices." },
  { name: "Displays", icon: "monitor", description: "Arrange your screens and adjust how things look." },
  { name: "Devices", icon: "grid", description: "Connected keyboards, mice, USB devices, Bluetooth accessories, printers and drives." },
  { name: "Storage", icon: "settings", description: "Inspect disks, filesystem usage, and removable volumes." },
  { name: "Users", icon: "home", description: "Manage local accounts and administrator access." },
  { name: "Search", icon: "search", description: "Choose launcher search providers." },
  { name: "Sound", icon: "sound", description: "Choose speakers and microphones, and set comfortable levels." },
];
const navigationGroups = [
  { name: "Workspace", sections: ["Home", "Appearance", "Wallpaper", "Startup Apps", "Applications", "Search", "Notifications"] },
  { name: "Devices & connections", sections: ["Wi-Fi", "Bluetooth", "Displays", "Sound", "Keyboard", "Mouse & Touchpad", "Devices", "Printers & Scanners"] },
  { name: "System", sections: ["Power & Lock", "General", "Storage", "Users", "Date & Time", "Keyboard Shortcuts", "Privacy & Security"] },
];
const shortcuts = [["Screenshot desktop", "Print Screen"], ["Screenshot area", "Super + Shift + S / Shift + Print Screen"],["Applications", "Super + Space"], ["Control Center", "Super + C"], ["Lock desktop", "Super + L"], ["Open Files", "Super + E"], ["Settings", "Super + I"], ["Switch windows", "Alt + Tab"], ["Minimize window", "Super + ↓"], ["Maximize / restore", "Super + ↑"], ["Snap left / right", "Super + ← / →"], ["Window menu", "Alt + Space"]];
const searchTerms: Record<string, string> = {
  "Keyboard": "keyboard typing repeat rate delay layout",
  "Mouse & Touchpad": "mouse pointer touchpad tap click scrolling acceleration",
  "Keyboard Shortcuts": "shortcuts hotkeys bindings",
  "Date & Time": "clock timezone time date ntp",
  "Printers & Scanners": "printer cups scanner sane",
  "Notifications": "notifications do not disturb dnd alerts",
  "Privacy & Security": "privacy security permissions microphone camera location lock",
  Devices: "peripheral connected usb wired wireless keyboard mouse touchpad printer dongle bluetooth drive",
  Storage: "storage disk volume filesystem mount usb removable capacity",
  General: "about version update github system processor memory kernel computer",
  Users: "account password administrator login delete",
  Home: "shortcuts keyboard minimize maximize snap window controls",
  Appearance: "dark color titlebar font typography",
  "Startup Apps": "login autostart launch boot sign in background applications",
  Applications: "installed apps default application web browser files file manager terminal email pdf image text software programs launch open uninstall remove manage",
  "Power & Lock": "battery power charging energy sleep screen lock idle timeout standby",
  "Wi-Fi": "wifi internet wireless network",
  Bluetooth: "pair headphones mouse keyboard",
  Displays: "monitor screen resolution scaling rotation brightness",
  Sound: "audio volume speaker mute microphone input output headphones recording",
};
function FontEffectsPanel() {
  const [effects, setEffects] = useState<FontEffects>(getFontEffects);
  useEffect(() => subscribeFontEffects(setEffects), []);
  function update(key: keyof FontEffects, value: number) {
    const next = { ...effects, [key]: value };
    setEffects(next);
    saveFontEffects(next);
  }
  const controls: { key: keyof FontEffects; label: string; max: number; step: number; unit: string }[] = [
    { key: "stroke", label: "Text stroke", max: 1, step: 0.1, unit: "px" },
    { key: "shadow", label: "Text shadow", max: 8, step: 0.5, unit: "px" },
    { key: "blur", label: "Text blur", max: 2, step: 0.1, unit: "px" },
  ];
  return <section className="settings-card font-effects" aria-labelledby="font-effects-title">
    <div className="font-effects-heading"><h3 id="font-effects-title">Font effects</h3><p>Fine tune text appearance across Meridian apps. Settings are saved automatically.</p></div>
    {controls.map(({ key, label, max, step, unit }) => <label className="font-effect-row" key={key}><span>{label}</span><Slider min={0} max={max} step={step} value={effects[key]} aria-label={label} onValueChange={value => update(key, value)} /><output>{effects[key] === 0 ? "Off" : `${effects[key]} ${unit}`}</output></label>)}
    <div className="font-effect-preview">The quick brown fox jumps over Meridian.</div>
  </section>;
}

function Settings() {
  const [section, setSection] = useState("Home");
  const [search, setSearch] = useState("");
  const { desktop, setDesktop } = useDesktop();
  const { settings } = useSystemSettings();
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    const offWallpaper = on("preferences.wallpaper", () => { setSection("Wallpaper"); setSearch(""); });
    const offPage = on("preferences.page", event => { const names: Record<string, string> = { home: "Home", sound: "Sound", power: "Power & Lock", "startup-apps": "Startup Apps", applications: "Applications", wifi: "Wi-Fi", bluetooth: "Bluetooth", displays: "Displays", appearance: "Appearance", wallpaper: "Wallpaper", desktop: "Home", general: "General", users: "Users", "device-manager": "Devices", devices: "Devices", storage: "Storage", search: "Search", keyboard: "Keyboard", mouse: "Mouse & Touchpad", notifications: "Notifications", privacy: "Privacy & Security", shortcuts: "Keyboard Shortcuts", printers: "Printers & Scanners", datetime: "Date & Time" }; setSection(names[event.data.page] ?? "Home"); setSearch(""); setError(""); });
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
  async function font(id: string) {
    if (busy) return; setBusy(true); setError("");
    try { setDesktop(await request({ method: "desktop.font_set", params: { id } })); }
    catch (e) { setError(errorMessage(e)); } finally { setBusy(false); }
  }
  const categories = sections.filter(item => `${item.name} ${item.description} ${searchTerms[item.name] ?? ""}`.toLowerCase().includes(search.trim().toLowerCase()));
  return <main className="settings-app">
    <aside className="settings-sidebar"><div className="settings-identity"><span className="settings-brand"><MeridianLogo width="24" height="24" /></span><div><h1>Settings</h1><small>Make Meridian yours</small></div></div>
      <label className="settings-search"><Icon name="search" width="16" height="16" /><input aria-label="Search settings" placeholder="Find a setting" value={search} onKeyDown={event => { if (event.key === "Enter" && categories[0]) { event.preventDefault(); navigate(categories[0].name); } if (event.key === "Escape") setSearch(""); }} onChange={e => setSearch(e.target.value)} />{search && <Button aria-label="Clear settings search" onClick={() => setSearch("")}><Icon name="close" width="14" height="14" /></Button>}</label>
      <nav aria-label="Settings categories">{navigationGroups.map(group => { const matching = group.sections.flatMap(name => categories.filter(item => item.name === name)); return matching.length > 0 && <div className="settings-nav-group" key={group.name}><h2>{group.name}</h2>{matching.map(({ name, icon }) => <Button key={name} aria-label={name} aria-current={section === name ? "page" : undefined} onClick={() => navigate(name)}><Icon name={icon} /><span>{name}</span></Button>)}</div>; })}</nav>
      {!categories.length && <p className="settings-no-results">No settings match “{search}”. Try “display” or “wallpaper”.</p>}

    </aside>
    <section className="settings-content" aria-labelledby="settings-title"><div className="settings-page-heading"><div className="settings-heading-copy"><span className="settings-breadcrumb">Settings <Icon name="forward" width="11" height="11" />{section}</span><h2 id="settings-title" ref={heading} tabIndex={-1}>{section}</h2><p>{sections.find(item => item.name === section)?.description}</p></div></div>
      {section === "Home" ? <>
        <div className="workspace-preview"><div className="workspace-copy"><span className="workspace-eyebrow">YOUR WORKSPACE</span><h3>Welcome to Meridian.</h3><p>A place for your files, your favorite apps, and a desktop that feels like home.</p><Button onClick={() => navigate("Appearance")}><Icon name="appearance" width="16" height="16" />Personalize appearance<Icon name="forward" width="14" height="14" /></Button></div><Button className="workspace-wallpaper" aria-label="Change desktop wallpaper" onClick={() => navigate("Wallpaper")}>{desktop && <img src={desktop.wallpaper_uri} alt="" />}<span><Icon name="image" width="14" height="14" />Change wallpaper</span></Button></div>
        <h3 className="home-section-title">Everyday essentials</h3>
        <div className="settings-quick-links">{["Wallpaper", "Wi-Fi", "Displays", "Sound", "Startup Apps", "General"].map(name => { const item = sections.find(s => s.name === name)!; return <Button key={name} onClick={() => navigate(name)}><span className="quick-link-icon"><Icon name={item.icon} /></span><span><strong>{name}</strong><small>{name === "Wi-Fi" ? settings.wifi === null ? "Connection settings" : settings.wifi ? "Wi-Fi is on" : "Wi-Fi is off" : name === "Sound" ? settings.volume === null ? "Volume controls" : `Volume · ${settings.volume}%` : name === "Displays" ? "Resolution and scaling" : name === "Startup Apps" ? "Ready when you sign in" : name === "General" ? "System information and updates" : "Choose your background"}</small></span><Icon name="forward" width="16" height="16" /></Button>; })}</div>
        <div className="settings-card shortcut-card"><div className="shortcut-heading"><Icon name="keyboard" /><h3>Keyboard shortcuts</h3></div>{shortcuts.map(([name, keys]) => <div className="settings-row" key={name}><span>{name}</span><kbd>{keys}</kbd></div>)}<p className="section-caption card-caption">In a nested development window, use Ctrl + Alt + H for Applications and Ctrl + Alt + C for Control Center.</p></div>
      </> : section === "Keyboard" ? <InputSettings kind="keyboard" /> : section === "Mouse & Touchpad" ? <InputSettings kind="mouse" /> : section === "Keyboard Shortcuts" ? <ShortcutSettings /> : section === "Notifications" ? <NotificationSettings /> : section === "Date & Time" ? <DateTimeSettings /> : section === "Printers & Scanners" ? <PrinterSettings /> : section === "Privacy & Security" ? <PrivacySettings /> : section === "Search" ? <SearchSettings /> : section === "Applications" ? <ApplicationsSettings /> : section === "Power & Lock" ? <Power /> : section === "Startup Apps" ? <Startup /> : section === "General" ? <General /> : section === "Storage" ? <Storage /> : section === "Devices" ? <ConnectedDevices /> : section === "Users" ? <Users /> : section === "Displays" ? <Displays /> : section === "Wi-Fi" || section === "Bluetooth" ? <Wireless key={section} kind={section === "Wi-Fi" ? "wifi" : "bluetooth"} /> : section === "Wallpaper" ? <>
        <div className="settings-card wallpaper-card"><div className="wallpaper-preview-heading"><h3>Current wallpaper</h3><p>Preview of your desktop background</p></div>{desktop ? <img className="wallpaper-current" src={desktop.wallpaper_uri} alt="Current desktop wallpaper" /> : <div className="wallpaper-loading" role="status">Loading wallpaper…</div>}<div className="wallpaper-list-heading"><h3>Default wallpapers</h3><p>Choose a background for your desktop</p></div><div className="wallpaper-choices" aria-label="Default wallpapers">{desktop?.wallpapers.map(choice => <Button className="wallpaper-choice" key={choice.id} aria-pressed={desktop.wallpaper === choice.id} disabled={busy} onClick={() => void wallpaper(choice.id)}><img src={choice.uri} alt="" /><span>{choice.name}</span>{desktop.wallpaper === choice.id && <Icon name="check" width="16" height="16" />}</Button>)}</div><div className="wallpaper-custom"><Button disabled={busy} onClick={() => void wallpaper()}><Icon name="image" width="16" height="16" />{busy ? "Updating…" : "Choose custom image…"}</Button><small>PNG, JPEG or WebP · up to 20 MB</small></div></div>
      </> : section === "Appearance" ? <>
        <div className="settings-card"><div className="settings-row"><strong>Dark mode</strong><small className="row-detail">Meridian always uses its dark, high-contrast interface.</small></div><div className="settings-row"><label htmlFor="dock-style">Dock style</label><Select id="dock-style" aria-label="Dock style" value={desktop?.dock_style ?? "floating"} disabled={busy || !desktop} onChange={e => { setError(""); void request({ method: "desktop.dock_style_set", params: { id: e.target.value } }).then(setDesktop).catch(err => setError(errorMessage(err))); }}><option value="floating">Floating</option><option value="full-width">Full width</option><option value="minimal">Minimal</option></Select></div><div className="settings-row"><label htmlFor="hover-background">Hover background</label><Select id="hover-background" value={desktop?.hover_background ?? "auto"} disabled={busy || !desktop} onChange={e => { setError(""); void request({ method: "desktop.hover_background_set", params: { value: e.target.value as import("../../lib/generated/HoverBackground").HoverBackground } }).then(setDesktop).catch(e => setError(errorMessage(e))); }}>{["auto", "green", "blue", "grey"].map(value => <option key={value} value={value}>{value[0]!.toUpperCase() + value.slice(1)}</option>)}</Select></div><div className="settings-row"><label htmlFor="system-font">Font</label><Select id="system-font" aria-label="System font" value={desktop?.font ?? "Inter"} disabled={busy || !desktop} onChange={e => void font(e.target.value)}>{["Inter", "Noto Sans", "DejaVu Sans", "Liberation Sans"].map(name => <option key={name} value={name}>{name}</option>)}</Select></div></div><FontEffectsPanel />
      </> : section === "Sound" ? <Sound /> : null}
      {error && <p className="settings-error" role="alert">{error}</p>}
    </section>
  </main>;
}
mount(<Settings />);
