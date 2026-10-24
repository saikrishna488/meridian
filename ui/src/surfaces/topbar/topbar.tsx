import { usePowerStatus } from "../../lib/power";
import { BatteryIcon } from "../../components/BatteryIcon";
import { useEffect, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { mount } from "../../lib/react";
import type { WindowInfo } from "../../lib/generated/WindowInfo";
import { MeridianLogo } from "../../components/MeridianLogo";
import { Icon } from "../../components/Icon";
import { Clock } from "../../components/Clock";
import type { SettingsState } from "../../lib/generated/SettingsState";
const report = (error: unknown) => console.error("meridian top bar:", error);
function Topbar() {
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const focused = windows.find(window => window.active);
  const power = usePowerStatus();
  const battery = power?.batteries[0];
  const [settings, setSettings] = useState<SettingsState>({ wifi: null, bluetooth: null, brightness: null, volume: null });
  const [options, setOptions] = useState(false), [notifications, setNotifications] = useState(false), [unread, setUnread] = useState(0);
  useEffect(() => {
    void hello().catch(report);
    let revision = 0, windowRevision = 0, settingsRevision = 0, alive = true;
    const offWindows = on("windows.changed", e => { windowRevision++; setWindows(e.data); });
    void request({ method: "windows.list" }).then(items => { if (alive && !windowRevision) setWindows(items ?? []); }).catch(report);
    const offSettings = on("settings.changed", e => { settingsRevision++; setSettings(e.data); });
    void request({ method: "settings.get" }).then(value => { if (alive && !settingsRevision && value) setSettings(value); }).catch(report);
    void request({ method: "notifications.get" }).then(s => { if (!revision && s) setUnread(s.items.filter(i => i.unread).length); }).catch(report);
    const off = on("popover.state", e => { setOptions(e.data.options); setNotifications(e.data.notifications); });
    const changed = on("notifications.changed", e => { revision++; setUnread(e.data.items.filter(i => i.unread).length); });
    return () => { alive = false; off(); changed(); offWindows(); offSettings(); };
  }, []);
  return <header className="topbar" aria-label="Desktop top bar">
    <button className="menu-brand" title="Meridian menu" aria-label="Meridian menu" aria-haspopup="menu"
      onClick={event => void request({ method: "menu.open", params: { menu: "Meridian", x: Math.round(event.currentTarget.getBoundingClientRect().left) } }).catch(report)}>
      <MeridianLogo />
    </button>
    {focused && <button className="focused-app" aria-label={`${focused.name} menu`} aria-haspopup="menu"
      onClick={event => void request({ method: "menu.open", params: { menu: "Application", window_id: focused.id, x: Math.round(event.currentTarget.getBoundingClientRect().left) } }).catch(report)}>{focused.name}</button>}
    <div className="topbar-spacer" />
    <button className="taskbar-calendar" aria-label="Open Calendar" title="Open Calendar" onClick={() => void request({ method: "search.activate", params: { provider: "apps", item_id: "meridian-calendar.desktop" } }).catch(report)}><Clock compact /></button>
    <div className="system-tray">
      <button className={`tray-indicator${settings.wifi ? " is-active" : ""}`} aria-label={`Wi-Fi ${settings.wifi === null ? "unavailable" : settings.wifi ? "on" : "off"}. Open Control Center`} title="Wi-Fi · Control Center"
        onClick={() => void request({ method: "popover.toggle", params: { popover: "options" } }).catch(report)}><Icon name="wifi" width="17" height="17" /></button>
      <button className={`tray-indicator${settings.volume !== null ? " is-active" : ""}`} aria-label={`${settings.volume === null ? "Audio unavailable" : `Volume ${settings.volume} percent`}. Open Control Center`} title="Sound · Control Center"
        onClick={() => void request({ method: "popover.toggle", params: { popover: "options" } }).catch(report)}><Icon name="sound" width="17" height="17" /></button>
      {battery && <button className={`taskbar-battery${battery.percent !== null && battery.percent <= 15 && power?.on_ac === false ? " low" : ""}`} aria-label={`Battery ${battery.percent === null ? "level unavailable" : `${battery.percent}%`}, ${battery.status}. Open Power settings`} title={power?.batteries.map(b => `${b.name}: ${b.percent === null ? "Unknown" : `${b.percent}%`} · ${b.status}`).join("\n")} onClick={() => void request({ method: "preferences.page_open", params: { page: "power" } }).catch(report)}><BatteryIcon percent={battery.percent} charging={battery.status === "Charging"} />{power?.batteries.length === 1 && battery.percent !== null && <span>{battery.percent}%</span>}</button>}
      <button id="options" title="Control Center (Super+C)" aria-label="Control Center" aria-haspopup="dialog" aria-expanded={options}
        onClick={() => void request({ method: "popover.toggle", params: { popover: "options" } }).catch(report)}>
        <Icon name="settings" width="17" height="17" />
      </button>
      <button id="notifications" title="Notifications" aria-label={unread ? `Notifications, ${unread} unread` : "Notifications"} aria-haspopup="dialog" aria-expanded={notifications} onClick={() => void request({ method: "popover.toggle", params: { popover: "notifications" } }).catch(report)}><Icon name="bell" width="20" height="20" />{unread > 0 && <span className="notification-dot" />}</button>
    </div>
  </header>;
}
mount(<Topbar />);
