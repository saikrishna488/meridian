import { SurfaceButton } from "../../meridian-ui";
import { useEffect, useRef, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { mount } from "../../lib/react";
import { Dock } from "../../components/Dock";
import { ControlCenterButton } from "../../components/ControlCenterButton";
import { useDesktop } from "../../lib/desktop";
import { MeridianLogo } from "../../components/MeridianLogo";
import { Icon } from "../../components/Icon";
import { Clock } from "../../components/Clock";
import type { SettingsState } from "../../lib/generated/SettingsState";

const report = (error: unknown) => console.error("meridian panel:", error);

function Panel() {
  const { desktop } = useDesktop();
  const [settings, setSettings] = useState<SettingsState>({ wifi: null, bluetooth: null, brightness: null, volume: null });
  const [desktopBusy, setDesktopBusy] = useState(false), [desktopError, setDesktopError] = useState("");
  const [notifications, setNotifications] = useState(false);
  const [options, setOptions] = useState(false);
  const [unread, setUnread] = useState(0);
  const [engaged, setEngaged] = useState(false), [launcher, setLauncher] = useState(false);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const hidden = !!desktop?.taskbar?.auto_hide && !engaged && !options && !notifications && !launcher;
  function reveal() { clearTimeout(hideTimer.current); setEngaged(true); }
  function scheduleHide() {
    clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => {
      if (!document.querySelector(".taskbar :focus-visible")) setEngaged(false);
    }, 700);
  }
  useEffect(() => {
    window.addEventListener("meridian-panel-leave", scheduleHide);
    return () => { clearTimeout(hideTimer.current); window.removeEventListener("meridian-panel-leave", scheduleHide); };
  }, []);

  useEffect(() => {
    void hello().catch(report);
    let revision = 0, settingsRevision = 0, alive = true;
    const offSettings = on("settings.changed", e => { settingsRevision++; setSettings(e.data); });
    void request({ method: "settings.get" }).then(value => { if (alive && !settingsRevision && value) setSettings(value); }).catch(report);
    void request({ method: "notifications.get" }).then(s => { if (!revision && s) setUnread(s.items.filter(i => i.unread).length); }).catch(report);
    const off = on("popover.state", e => { setNotifications(e.data.notifications); setOptions(e.data.options); setLauncher(e.data.applications); });
    const changed = on("notifications.changed", e => { revision++; setUnread(e.data.items.filter(i => i.unread).length); });
    return () => { alive = false; off(); changed(); offSettings(); };
  }, []);

  return <footer className="taskbar" data-hidden={hidden} data-icon-size={desktop?.taskbar?.icon_size ?? 32} aria-label="Desktop panel" onMouseEnter={reveal} onMouseLeave={scheduleHide} onFocusCapture={reveal} onBlurCapture={scheduleHide}>
    <div className="panel-left">
      <SurfaceButton aria-label="Show desktop or restore windows" title="Show desktop / restore windows" disabled={desktopBusy} onClick={() => { setDesktopBusy(true); setDesktopError(""); void request({method:"windows.show_desktop"}).catch(e => setDesktopError(String(e))).finally(() => setDesktopBusy(false)); }}><Icon name="monitor" width="18" height="18" /></SurfaceButton>
      {desktopError && <span role="alert" title={desktopError}>Desktop action failed</span>}
      <span className="menu-brand" aria-label="Meridian"><MeridianLogo /></span>

    </div>
    <div className="panel-center">
      <Dock hidden={hidden} />
    </div>
    <div className="panel-right">
      <div className="system-tray">
      <ControlCenterButton settings={settings} expanded={options} onClick={() => void request({method:"popover.toggle",params:{popover:"options"}}).catch(report)} />
      <SurfaceButton id="notifications" title="Notifications" aria-label={unread ? `Notifications, ${unread} unread` : "Notifications"} aria-haspopup="dialog" aria-expanded={notifications} onClick={() => void request({ method: "popover.toggle", params: { popover: "notifications" } }).catch(report)}><Icon name="bell" width="20" height="20" />{unread > 0 && <span className="notification-dot" />}</SurfaceButton>
      </div>
      <SurfaceButton className="taskbar-calendar" aria-label="Open Calendar" title="Open GNOME Calendar" onClick={() => void request({ method: "search.activate", params: { provider: "apps", item_id: "org.gnome.Calendar.desktop" } }).catch(report)}><Clock compact format={desktop?.taskbar?.clock_format} /></SurfaceButton>
    </div>
  </footer>;
}

mount(<Panel />);
