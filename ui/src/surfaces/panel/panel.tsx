import { useEffect, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { mount } from "../../lib/react";
import { Dock } from "../../components/Dock";
import { Clock } from "../../components/Clock";
const report = (error: unknown) => console.error("meridian panel:", error);
function Panel() {
  const [options, setOptions] = useState(false);
  useEffect(() => {
    void hello().catch(report);
    return on("popover.state", e => setOptions(e.data.options));
  }, []);
  return <footer className="taskbar" aria-label="Desktop taskbar">
    <button className="menu-brand" title="Meridian menu" aria-label="Meridian menu" aria-haspopup="menu"
      onClick={event => void request({ method: "menu.open", params: { menu: "Meridian", x: Math.round(event.currentTarget.getBoundingClientRect().left) } }).catch(report)}>
      <span aria-hidden="true">◈</span><span className="brand-label">Meridian</span>
    </button>
    <Dock />
    <div className="system-tray">
      <button id="options" title="Control Center (Super+C)" aria-label="Control Center" aria-haspopup="dialog" aria-expanded={options}
        onClick={() => void request({ method: "popover.toggle", params: { popover: "options" } }).catch(report)}>
        <img width="22" height="22" src="meridian://ui/desktop-icons/meridian/control-center.svg" alt="" />
      </button>
      <Clock compact />
    </div>
  </footer>;
}
mount(<Panel />);
