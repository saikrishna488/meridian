import { SurfaceButton } from "../../meridian-ui";
import { ControlCenterButton } from "../../components/ControlCenterButton";
import { useDesktop } from "../../lib/desktop";
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
  const { desktop } = useDesktop();
  const [settings, setSettings] = useState<SettingsState>({ wifi: null, bluetooth: null, brightness: null, volume: null });
  const [notifications, setNotifications] = useState(false), [options, setOptions] = useState(false), [unread, setUnread] = useState(0);
  useEffect(() => {
    void hello().catch(report);
    let revision = 0, windowRevision = 0, settingsRevision = 0, alive = true;
    const offWindows = on("windows.changed", e => { windowRevision++; setWindows(e.data); });
    void request({ method: "windows.list" }).then(items => { if (alive && !windowRevision) setWindows(items ?? []); }).catch(report);
    const offSettings = on("settings.changed", e => { settingsRevision++; setSettings(e.data); });
    void request({ method: "settings.get" }).then(value => { if (alive && !settingsRevision && value) setSettings(value); }).catch(report);
    void request({ method: "notifications.get" }).then(s => { if (!revision && s) setUnread(s.items.filter(i => i.unread).length); }).catch(report);
    const off = on("popover.state", e => { setNotifications(e.data.notifications); setOptions(e.data.options); });
    const changed = on("notifications.changed", e => { revision++; setUnread(e.data.items.filter(i => i.unread).length); });
    return () => { alive = false; off(); changed(); offWindows(); offSettings(); };
  }, []);
  return <header className="topbar" aria-label="Desktop top bar">
    <span className="menu-brand" aria-label="Meridian"><MeridianLogo /></span>
    {focused && <SurfaceButton className="focused-app" aria-label={`${focused.name} menu`} aria-haspopup="menu"
      onClick={event => void request({ method: "menu.open", params: { menu: "Application", window_id: focused.id, x: Math.round(event.currentTarget.getBoundingClientRect().left) } }).catch(report)}>{focused.name}</SurfaceButton>}
    <div className="topbar-spacer" />
    <div className="system-tray">
      <ControlCenterButton settings={settings} expanded={options} onClick={() => void request({method:"popover.toggle",params:{popover:"options"}}).catch(report)} />
      <SurfaceButton id="notifications" title="Notifications" aria-label={unread ? `Notifications, ${unread} unread` : "Notifications"} aria-haspopup="dialog" aria-expanded={notifications} onClick={() => void request({ method: "popover.toggle", params: { popover: "notifications" } }).catch(report)}><Icon name="bell" width="20" height="20" />{unread > 0 && <span className="notification-dot" />}</SurfaceButton>
    </div>
    <SurfaceButton className="taskbar-calendar" aria-label="Open Calendar" title="Open GNOME Calendar" onClick={() => void request({ method: "search.activate", params: { provider: "apps", item_id: "org.gnome.Calendar.desktop" } }).catch(report)}><Clock compact format={desktop?.taskbar?.clock_format} /></SurfaceButton>
  </header>;
}
mount(<Topbar />);
