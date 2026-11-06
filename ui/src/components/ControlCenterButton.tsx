import { SurfaceButton } from "../meridian-ui";
import { Icon } from "./Icon";
import { BatteryIcon } from "./BatteryIcon";
import { usePowerStatus } from "../lib/power";
import { useDesktop } from "../lib/desktop";
import type { SettingsState } from "../lib/generated/SettingsState";
export function ControlCenterButton({ settings, expanded, onClick }: { settings: SettingsState; expanded: boolean; onClick: () => void }) {
  const power = usePowerStatus(), battery = power?.batteries[0];
  const { desktop } = useDesktop();
  const wifi = settings.wifi === null ? "unavailable" : settings.wifi ? "on" : "off";
  const sound = settings.volume === null ? "unavailable" : `${settings.volume}%`;
  const charge = battery ? `, battery ${battery.percent === null ? "unknown" : `${battery.percent}%`}, ${battery.status}` : "";
  return <SurfaceButton className="control-center-trigger" title="Control Center" aria-label={`Control Center. Wi-Fi ${wifi}, volume ${sound}${charge}`} aria-haspopup="dialog" aria-expanded={expanded} onClick={onClick}>
    <Icon name="wifi" /><Icon name={settings.volume === 0 ? "muted" : "sound"} />
    {battery && <span className={battery.percent !== null && battery.percent <= 15 && power?.on_ac === false ? "low" : ""}><BatteryIcon percent={battery.percent} charging={battery.status === "Charging"} />{desktop?.taskbar?.battery_percentage !== false && battery.percent !== null && <span className="tray-battery-percent">{battery.percent}%</span>}</span>}
  </SurfaceButton>;
}
