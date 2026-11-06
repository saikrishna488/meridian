import { Alert, ConfirmDialog, Disclosure, ActionTile, SearchField } from "../../meridian-ui";
import { IconTile, type Tone } from "../../meridian-ui";
import { SettingsPageBoundary } from "./page-boundary";
import { Button } from "../../meridian-ui";
import { SearchSettings } from "./search";
import { ApplicationsSettings } from "./applications";
import { Select } from "../../meridian-ui";
import { useEffect, useRef, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { mount, errorMessage } from "../../lib/react";
import { useDesktop } from "../../lib/desktop";
import { ComfortSettings, FirstRunChecklist, RecoverySettings } from "./workflows";
import { SaveStatus } from "./save-status";
import { DesktopLayoutSettings } from "./desktop-layout";
import { TaskbarSettings } from "./taskbar";
import { UIKit } from "./ui-kit";
import { Engines } from "./engines";
import { General } from "./general";
import { InputSettings, ShortcutSettings, NotificationSettings, DateTimeSettings } from "./essentials";
import { Startup } from "./startup";
import { Sound } from "./sound";
import { Power } from "./power";
import { Storage } from "./storage";
import { MeridianLogo } from "../../components/MeridianLogo";
import { Users } from "./users";
import { Displays, Wireless } from "./devices";
import { Icon, type IconName } from "../../components/Icon";

const sections: { name: string; icon: IconName; description: string }[] = [
  { name: "Recovery", icon: "refresh", description: "Reload desktop components and review recent session errors." },
  { name: "UI Kit", icon: "grid", description: "Inspect components, themes and interaction states." },
  { name: "Accessibility", icon: "accessibility", description: "Make Meridian easier to see, hear and control." },
  { name: "Connections", icon: "wifi", description: "Wi-Fi networks and Bluetooth accessories." },
  { name: "System", icon: "settings", description: "Power, accounts and advanced system preferences." },
  { name: "Engines", icon: "monitor", description: "Turn the Font, GPU, CPU, and Audio engines on or off." },
  { name: "Keyboard", icon: "keyboard", description: "Key repeat and typing options." },
  { name: "Mouse & Touchpad", icon: "settings", description: "Pointer speed, scrolling, and touchpad gestures." },
  { name: "Date & Time", icon: "info", description: "Clock and time zone information." },
  { name: "Notifications", icon: "bell", description: "Manage notification interruptions." },
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
  { name: "Storage", icon: "settings", description: "Inspect disks, filesystem usage, and removable volumes." },
  { name: "Users", icon: "home", description: "Manage local accounts and administrator access." },
  { name: "Search", icon: "search", description: "Choose launcher search providers." },
  { name: "Sound", icon: "sound", description: "Choose speakers and microphones, and set comfortable levels." },
];
const navigationGroups = [
  { name: "Desktop", sections: ["Home", "Appearance", "Connections", "Sound", "Applications"] },
  { name: "Preferences", sections: ["Accessibility", "System"] },
];
const settingsHubs: Record<string, string[]> = {
  Accessibility: ["Displays", "Sound", "Keyboard", "Mouse & Touchpad", "Notifications"],
  Connections: ["Wi-Fi", "Bluetooth"],
  System: ["Recovery", "UI Kit", "Power & Lock", "Startup Apps", "Search", "Engines", "Storage", "Users", "Date & Time", "General"],
};
const parentPage: Record<string, string> = {
  Recovery: "System", "UI Kit": "System", Displays: "Accessibility", Keyboard: "Accessibility", "Mouse & Touchpad": "Accessibility", Notifications: "Accessibility",
  "Wi-Fi": "Connections", Bluetooth: "Connections", Wallpaper: "Appearance",
  "Power & Lock": "System", "Startup Apps": "System", Search: "System", Engines: "System", Storage: "System", Users: "System", "Date & Time": "System", General: "System",
};
const categoryTones: Record<string, Tone> = {
  Home:"white", Engines:"orange", Keyboard:"white", "Mouse & Touchpad":"white", "Date & Time":"red",
  Notifications:"red", General:"white", Wallpaper:"blue", Appearance:"white", "Startup Apps":"orange",
  Applications:"blue", "Power & Lock":"green", "Wi-Fi":"blue", Bluetooth:"blue", Displays:"blue",
  Storage:"orange", Users:"blue", Search:"white", Sound:"red",
};
const searchTerms: Record<string, string> = {
  Recovery: "repair restart reset failed crash errors recovery",
  Accessibility: "text size font bigger larger zoom high contrast reduce motion accessibility vision hearing input keyboard mouse touchpad display brightness notifications shortcuts",
  Connections: "wifi bluetooth internet wireless network",
  System: "power accounts storage date time startup search engines advanced",
  Engines: "rendering font graphics gpu cpu audio sound quality performance native",
  "Keyboard": "keyboard typing repeat rate delay layout shortcuts hotkeys bindings",
  "Mouse & Touchpad": "mouse pointer touchpad tap click scrolling acceleration",
  "Date & Time": "clock timezone time date ntp",
  "Notifications": "notifications do not disturb dnd alerts",
  Storage: "storage disk volume filesystem mount usb removable capacity",
  General: "about version update github system processor memory kernel computer",
  Users: "account password administrator login delete",
  Home: "overview desktop hardware storage engines",
  Appearance: "light dark theme dock style floating full width minimal",
  "Startup Apps": "login autostart launch boot sign in background applications",
  Applications: "installed apps default application web browser files file manager terminal email pdf image text software programs launch open uninstall remove manage",
  "Power & Lock": "battery power charging energy sleep timer screen lock idle timeout standby",
  "Wi-Fi": "wifi internet wireless network",
  Bluetooth: "pair headphones mouse keyboard",
  Displays: "monitor screen resolution scaling rotation brightness",
  Sound: "audio volume speaker mute microphone input output headphones recording",
};
const controlIndex = [
  ["Accessibility", "Text size", "bigger larger font zoom reading"],
  ["Accessibility", "High contrast controls", "visibility borders"],
  ["Accessibility", "Reduce motion", "animation movement"],
  ["Keyboard", "Keyboard layout", "language typing"],
  ["Keyboard", "Num Lock on startup", ""],
  ["Keyboard", "Key repeat speed", ""],
  ["Keyboard", "Repeat delay", ""],
  ["Mouse & Touchpad", "Pointer speed", ""],
  ["Mouse & Touchpad", "Pointer acceleration", ""],
  ["Mouse & Touchpad", "Natural scrolling", ""],
  ["Mouse & Touchpad", "Left-handed buttons", ""],
  ["Mouse & Touchpad", "Touchpad speed", ""],
  ["Mouse & Touchpad", "Tap to click", ""],
  ["Mouse & Touchpad", "Tap and drag", ""],
  ["Mouse & Touchpad", "Disable while typing", ""],
  ["Notifications", "Do Not Disturb", ""],
  ["Date & Time", "Time zone", ""],
  ["Date & Time", "Automatic time synchronization", ""],
  ["Power & Lock", "Automatically lock", "sleep timer screen lock"],
  ["Power & Lock", "Turn off the display", ""],
  ["Search", "System settings", ""],
  ["Search", "Calculator", ""],
  ["Engines", "Font engine", ""],
  ["Engines", "GPU engine", ""],
  ["Engines", "CPU engine", ""],
  ["Engines", "Audio engine", ""],
  ["Appearance", "Color theme", "light dark mode"],
  ["Appearance", "Dock style", "floating full width minimal"],
  ["Appearance", "App icon size", "taskbar small medium large"],
  ["Appearance", "Auto-hide", "taskbar hide dock"],
  ["Appearance", "Clock format", "12 hour 24 hour time"],
  ["Appearance", "Battery percentage", "charge percent"],
  ["Appearance", "Desktop icon size", "small medium large"],
  ["Appearance", "Desktop icon spacing", "compact comfortable spacious"],
  ["Appearance", "Snap to grid", "align desktop icons"],
  ["Appearance", "Show desktop icons", "hide shortcuts"],
  ["Appearance", "Window button placement", "left right close minimize maximize"],
  ["Appearance", "Desktop wallpaper", "background image"],
] as const;
function Settings() {
  const [resetAppearance, setResetAppearance] = useState(false), [resetWallpaper, setResetWallpaper] = useState(false);
  const [targetControl, setTargetControl] = useState<string | null>(null);
  const [section, setSection] = useState("Home");
  const currentPage = useRef("Home"), backStack = useRef<string[]>([]);
  const [canGoBack, setCanGoBack] = useState(false);
  const [search, setSearch] = useState("");
  const { desktop, setDesktop, error: desktopError, refresh: refreshDesktop } = useDesktop();
  const [pageRevision, setPageRevision] = useState(0);
  function retryPage() { setError(""); refreshDesktop(); setPageRevision(value => value + 1); }
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    const offWallpaper = on("preferences.wallpaper", () => { navigate("Wallpaper"); });
    const offPage = on("preferences.page", event => { const names: Record<string, string> = { accessibility: "Accessibility", recovery: "Recovery", engines: "Engines", home: "Home", sound: "Sound", power: "Power & Lock", "startup-apps": "Startup Apps", applications: "Applications", wifi: "Wi-Fi", bluetooth: "Bluetooth", displays: "Displays", appearance: "Appearance", wallpaper: "Wallpaper", desktop: "Home", general: "General", users: "Users", "device-manager": "System", devices: "System", storage: "Storage", search: "Search", keyboard: "Keyboard", mouse: "Mouse & Touchpad", notifications: "Notifications", privacy: "Power & Lock", shortcuts: "Keyboard", printers: "System", datetime: "Date & Time" }; navigate(names[event.data.page] ?? "Home"); });
    void hello().catch(console.error);
    return () => { offWallpaper(); offPage(); };
  }, []);
  useEffect(() => { document.querySelector<HTMLElement>('.settings-sidebar [aria-current="page"]')?.scrollIntoView({ block: "nearest", inline: "nearest" }); }, [section, search]);
  useEffect(() => {
    if (!targetControl) return;
    let timer: ReturnType<typeof setTimeout>;
    const find = () => {
      const row = [...document.querySelectorAll<HTMLElement>(".settings-content .settings-row")].find(row =>
        [...row.querySelectorAll("strong,label")].some(label => label.textContent?.trim() === targetControl));
      if (!row) return;
      observer.disconnect();
      row.classList.add("setting-search-hit"); row.scrollIntoView({block:"center", behavior:"auto"});
      row.tabIndex = -1;
      (row.querySelector<HTMLElement>("button:not(:disabled),input:not(:disabled),select:not(:disabled)") ?? row).focus({preventScroll:true});
      timer = setTimeout(() => { row.classList.remove("setting-search-hit"); setTargetControl(null); }, 3000);
    };
    const observer = new MutationObserver(find);
    observer.observe(document.querySelector(".settings-content")!, {childList:true, subtree:true, attributes:true, attributeFilter:["disabled"]});
    const frame = requestAnimationFrame(find);
    return () => { cancelAnimationFrame(frame); clearTimeout(timer); observer.disconnect(); document.querySelectorAll(".setting-search-hit").forEach(row => row.classList.remove("setting-search-hit")); };
  }, [section, targetControl]);
  function openControl(page: string, label: string) { navigate(page); setTargetControl(label); }
  function navigate(name: string, remember = true) {
    setTargetControl(null); setResetAppearance(false); setResetWallpaper(false);
    if (!sections.some(item => item.name === name)) name = "Home";
    if (remember && currentPage.current !== name) { backStack.current.push(currentPage.current); if (backStack.current.length > 32) backStack.current.shift(); }
    currentPage.current = name; setSection(name); setSearch(""); setError(""); setCanGoBack(backStack.current.length > 0);
    requestAnimationFrame(() => heading.current?.focus());
  }
  function goBack() { const previous = backStack.current.pop(); if (previous) navigate(previous, false); }
  async function wallpaper(id?: string) {
    if (busy) return; setBusy(true); setError("");
    try { setDesktop(await (id ? request({ method: "desktop.wallpaper_set", params: { id } }) : request({ method: "desktop.wallpaper_choose" }))); }
    catch (e) { setError(errorMessage(e)); } finally { setBusy(false); }
  }
  const appearancePending = useRef(false);
  async function changeAppearance(mode: string) {
    if (appearancePending.current) return;
    appearancePending.current = true;
    setBusy(true);
    setError("");
    try {
      setDesktop(await request({ method: "desktop.appearance_set", params: { id: mode } }));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      appearancePending.current = false;
      setBusy(false);
    }
  }
  const controlResults = search.trim() ? controlIndex.filter(parts => search.toLowerCase().trim().split(/\s+/).every(word => parts.join(" ").toLowerCase().includes(word))) : [];
  const categories = sections.filter(item => `${item.name} ${item.description} ${searchTerms[item.name] ?? ""}`.toLowerCase().includes(search.trim().toLowerCase()));
  return <main className="settings-app">
    <aside className="settings-sidebar"><div className="settings-identity"><span className="settings-brand" aria-label="Meridian"><MeridianLogo /></span><h1>Settings</h1></div>
      <SearchField className="settings-search" aria-label="Search settings" placeholder="Find a setting" value={search} onKeyDown={event => { if (event.key === "Enter") { if (controlResults[0]) { event.preventDefault(); openControl(controlResults[0][0], controlResults[0][1]); } else if (categories[0]) { event.preventDefault(); navigate(categories[0].name); } } if (event.key === "Escape") setSearch(""); }} onValueChange={setSearch} />
      {controlResults.length > 0 && <nav className="settings-control-results" aria-label="Matching controls">{controlResults.map(([page,label]) => <Button key={`${page}:${label}`} onClick={() => openControl(page,label)}><span>{label}<small>{page}</small></span></Button>)}</nav>}
      <nav aria-label="Settings categories">{navigationGroups.map(group => { const matching = (search.trim() ? (group.name === "Desktop" ? categories : []) : group.sections.flatMap(name => categories.filter(item => item.name === name))); return matching.length > 0 && <div className="settings-nav-group" key={group.name}><h2>{group.name}</h2>{matching.map(({ name, icon }) => <Button key={name} aria-label={name} aria-current={(section === name || (!search.trim() && parentPage[section] === name)) ? "page" : undefined} onClick={() => navigate(name)}><IconTile icon={icon} tone={categoryTones[name] ?? "blue"} /><span>{name}</span></Button>)}</div>; })}</nav>
      {!categories.length && !controlResults.length && <p className="settings-no-results">No settings match “{search}”. Try “display” or “wallpaper”. <Button onClick={()=>setSearch("")}>Show all settings</Button></p>}

    </aside>
    <section className="settings-content" aria-labelledby="settings-title"><nav className="settings-breadcrumb" aria-label="Settings path"><Button variant="ghost" size="icon" aria-label="Back to previous settings page" disabled={!canGoBack} onClick={goBack}><Icon name="back" width="14" height="14" /></Button><Button variant="ghost" size="small" aria-current={section === "Home" ? "page" : undefined} onClick={() => navigate("Home")}>Settings</Button>{parentPage[section] && <><Icon name="forward" width="11" height="11" /><Button variant="ghost" size="small" onClick={() => navigate(parentPage[section] ?? "Home")}>{parentPage[section]}</Button></>}{section !== "Home" && <><Icon name="forward" width="11" height="11" /><Button variant="ghost" size="small" aria-current="page" onClick={() => navigate(section)}>{section}</Button></>}</nav><div className="settings-page-heading"><IconTile icon={sections.find(item => item.name === section)?.icon ?? "settings"} tone={categoryTones[section] ?? "blue"} /><div className="settings-heading-copy"><h2 id="settings-title" ref={heading} tabIndex={-1}>{section}</h2><p>{sections.find(item => item.name === section)?.description}</p></div><Button variant="ghost" size="small" onClick={retryPage}>Reload page</Button></div>
      {desktopError && ["Home", "Wallpaper", "Appearance"].includes(section) && <Alert tone="error" action={<Button onClick={retryPage}>Retry</Button>}>{desktopError}</Alert>}
      <SaveStatus />
      <SettingsPageBoundary key={`${section}-${pageRevision}`} page={section} retry={retryPage}><div className="settings-page-body">
      {settingsHubs[section] ? <>{section === "Accessibility" && <ComfortSettings />}<div className="settings-hub-grid">{settingsHubs[section].map(name => {
        const item = sections.find(candidate => candidate.name === name)!;
        return <ActionTile className="settings-hub-card" key={name} title={name} description={item.description} icon={item.icon} onClick={() => navigate(name)} />;
      })}</div></> : section === "Home" ? <>
        <FirstRunChecklist navigate={navigate} />
        <h3 className="home-section-title">Your desktop</h3>
        <div className="settings-quick-links">{["Appearance", "Connections", "Accessibility", "System"].map(name => { const item = sections.find(s => s.name === name)!; return <Button key={name} onClick={() => navigate(name)}><IconTile icon={item.icon} tone={categoryTones[name] ?? "blue"} /><span><strong>{name}</strong><small>{item.description}</small></span><Icon name="forward" width="16" height="16" /></Button>; })}</div>

      </> : section === "Recovery" ? <RecoverySettings /> : section === "UI Kit" ? <UIKit /> : section === "Engines" ? <Engines /> : section === "Keyboard" ? <><InputSettings kind="keyboard" /><Disclosure title="Keyboard shortcuts"><ShortcutSettings /></Disclosure></> : section === "Mouse & Touchpad" ? <InputSettings kind="mouse" /> : section === "Notifications" ? <NotificationSettings /> : section === "Date & Time" ? <DateTimeSettings /> : section === "Search" ? <SearchSettings /> : section === "Applications" ? <ApplicationsSettings /> : section === "Power & Lock" ? <Power /> : section === "Startup Apps" ? <Startup /> : section === "General" ? <General /> : section === "Storage" ? <Storage /> : section === "Users" ? <Users /> : section === "Displays" ? <Displays /> : section === "Wi-Fi" || section === "Bluetooth" ? <Wireless key={section} kind={section === "Wi-Fi" ? "wifi" : "bluetooth"} /> : section === "Wallpaper" ? <>
        <div className="settings-card wallpaper-card"><div className="wallpaper-preview-heading"><h3>Current wallpaper</h3><p>Preview of your desktop background</p></div>{desktop ? <img className="wallpaper-current" src={desktop.wallpaper_uri} alt="Current desktop wallpaper" /> : <div className="wallpaper-loading" role="status">{desktopError ? "Wallpaper preferences are unavailable." : "Loading wallpaper…"}</div>}<div className="wallpaper-list-heading"><h3>Default wallpapers</h3><p>Choose a background for your desktop</p></div><div className="wallpaper-choices" aria-label="Default wallpapers">{desktop?.wallpapers.map(choice => <Button className="wallpaper-choice" key={choice.id} aria-pressed={desktop.wallpaper === choice.id} disabled={busy} onClick={() => void wallpaper(choice.id)}><img src={choice.uri} alt="" /><span>{choice.name}</span>{desktop.wallpaper === choice.id && <Icon name="check" width="16" height="16" />}</Button>)}</div><div className="wallpaper-custom"><Button disabled={busy} onClick={() => void wallpaper()}><Icon name="image" width="16" height="16" />{busy ? "Updating…" : "Choose custom image…"}</Button><small>PNG, JPEG or WebP · up to 20 MB</small></div></div>
        <Button disabled={busy || !desktop} onClick={() => setResetWallpaper(true)}>Restore wallpaper default…</Button>
        <ConfirmDialog open={resetWallpaper} title="Restore default wallpaper?" confirmLabel="Restore wallpaper" onClose={() => setResetWallpaper(false)} onConfirm={async () => { setDesktop(await request({method:"desktop.wallpaper_set",params:{id:"meridian-dusk"}})); }}>Replace the current background with Dusk. Your custom image file is preserved.</ConfirmDialog>
      </> : section === "Appearance" ? <>
        <DesktopLayoutSettings />
        <TaskbarSettings />
        <div className="settings-card"><div className="settings-row"><label htmlFor="appearance-mode">Color theme</label><Select id="appearance-mode" value={desktop?.appearance ?? "light"} disabled={busy || !desktop} onChange={event => void changeAppearance(event.target.value)}><option value="light">Light</option><option value="dark">Dark</option></Select></div></div>
        <div className="settings-card"><div className="settings-row"><span><strong>Desktop wallpaper</strong><p className="muted">Choose a background for your workspace.</p></span><Button onClick={() => navigate("Wallpaper")}>Choose wallpaper</Button></div></div>
        <div className="settings-card"><div className="settings-row"><span><strong>Comfort &amp; accessibility</strong><p className="muted">Display, sound, keyboard, pointer and interruption settings.</p></span><Button onClick={() => navigate("Accessibility")}>Open Accessibility</Button></div></div>
        <div className="settings-card"><div className="settings-row"><label htmlFor="dock-style">Dock style</label><Select id="dock-style" aria-label="Dock style" value={desktop?.dock_style ?? "floating"} disabled={busy || !desktop} onChange={e => { setError(""); void request({ method: "desktop.dock_style_set", params: { id: e.target.value } }).then(setDesktop).catch(err => setError(errorMessage(err))); }}><option value="floating">Floating</option><option value="full-width">Full width</option><option value="minimal">Minimal</option></Select></div></div>
        <Button disabled={busy || !desktop} onClick={() => setResetAppearance(true)}>Restore color & dock defaults…</Button>
        <ConfirmDialog open={resetAppearance} title="Restore color and dock defaults?" confirmLabel="Restore defaults" onClose={() => setResetAppearance(false)} onConfirm={async () => {
          setDesktop(await request({method:"desktop.appearance_set",params:{id:"light"}}));
          setDesktop(await request({method:"desktop.dock_style_set",params:{id:"floating"}}));
        }}>Use the Light theme and Floating dock. Wallpaper, pinned apps, taskbar preferences and desktop icon layout are preserved.</ConfirmDialog>
      </> : section === "Sound" ? <Sound /> : null}
      </div></SettingsPageBoundary>
      {error && <Alert tone="error">{error}</Alert>}
    </section>
  </main>;
}
mount(<Settings />);
