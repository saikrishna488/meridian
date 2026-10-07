import { useEffect, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { mount } from "../../lib/react";
import { Dock } from "../../components/Dock";
import { Clock } from "../../components/Clock";
const report = (error: unknown) => console.error("meridian panel:", error);
function openMenu(menu: string, button: HTMLButtonElement) {
  void request({ method: "menu.open", params: { menu, x: Math.round(button.getBoundingClientRect().left) } }).catch(report);
}
function Panel() {
  const [topbar, setTopbar] = useState<boolean | null>(null);
  const [name, setName] = useState("Meridian");
  const [options, setOptions] = useState(false);
  useEffect(() => {
    const offWindows = on("windows.changed", e => setName(e.data.find(w => w.active)?.name ?? "Meridian"));
    const offMenus = on("popover.state", e => setOptions(e.data.options));
    let alive = true;
    void hello().then(info => {
      if (!alive) return;
      const isTop = info?.surface === "topbar";
      document.body.classList.toggle("topbar-surface", isTop); setTopbar(isTop);
      if (isTop) void request({ method: "windows.list" }).then(windows => { if (alive) setName(windows?.find(w => w.active)?.name ?? "Meridian"); }).catch(report);
    }).catch(report);
    return () => { alive = false; offWindows(); offMenus(); };
  }, []);
  if (topbar === null) return null;
  return topbar ? <header className="topbar" aria-label="Desktop menu bar">
    <button className="menu-brand" aria-label="Meridian menu" aria-haspopup="menu" onClick={event => openMenu("Meridian", event.currentTarget)}>◈</button>
    <strong className="active-app">{name}</strong>
    {["File", "Edit", "View", "Window", "Help"].map(label => <button key={label} aria-haspopup="menu" onClick={event => openMenu(label, event.currentTarget)}>{label}</button>)}
    <div className="menu-spacer" />
    <button id="options" aria-label="Control Center" aria-haspopup="dialog" aria-expanded={options}
      onClick={() => void request({ method: "popover.toggle", params: { popover: "options" } }).catch(report)}>
      <img width="22" height="22" src="meridian://ui/desktop-icons/meridian/control-center.svg" alt="" />
    </button>
    <Clock />
  </header> : <Dock />;
}
mount(<Panel />);
