import { SurfaceButton } from "../../meridian-ui";
import { useEffect, useRef, useState } from "react";
import { on, request } from "../../lib/bridge";
import { MeridianLogo } from "../../components/MeridianLogo";
import { mount, useDialogFocus, usePopover, useWindowKey } from "../../lib/react";
import type { MenuOpen } from "../../lib/generated/MenuOpen";
import type { SystemInfo } from "../../lib/generated/SystemInfo";
import type { WindowInfo } from "../../lib/generated/WindowInfo";

function DesktopMenu() {
  const [label, setLabel] = useState("Meridian");
  const [x, setX] = useState(8);
  const [windowId, setWindowId] = useState<number | undefined>();
  const [dockItem, setDockItem] = useState<MenuOpen | null>(null);
  const [about, setAbout] = useState(false);
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const focused = windows.find(w => w.id === windowId);
  const [error, setError] = useState("");
  const first = useRef<HTMLButtonElement>(null);
  const report = (e: unknown) => setError(e instanceof Error ? e.message : "Couldn’t complete this action");
  const { visible, menu, dismiss } = usePopover(async () => { setAbout(false); setError(""); }, () => menu.current?.focus(), report);
  useEffect(() => on("menu.opened", e => {
    setWindowId(e.data.window_id);
    setDockItem(e.data.menu === "DockItem" ? e.data : null);
    setLabel(e.data.menu); setX(e.data.x); setAbout(false); setError("");
    void request({ method: "windows.list" }).then(setWindows).catch(report);
  }), []);
  useWindowKey(e => {
    if (e.key === "Escape") { e.preventDefault(); void dismiss().catch(report); }
    if (["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) {
      const buttons = Array.from(menu.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []);
      if (!buttons.length) return;
      e.preventDefault();
      const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
      buttons[e.key === "Home" ? 0 : e.key === "End" ? buttons.length - 1 : (index + (e.key === "ArrowUp" ? -1 : 1) + buttons.length) % buttons.length]?.focus();
    }
  });
  useDialogFocus(menu, visible && about, () => { void dismiss().catch(report); });
  async function showAbout() {
    setAbout(true); setInfo(null);
    try { setInfo(await request({ method: "system.info" })); } catch (e) { report(e); }
  }
  function toggle(popover: "applications" | "options") {
    void dismiss().then(() => request({ method: "popover.toggle", params: { popover } })).catch(report);
  }
  return <>
    <div hidden={!visible} className="desktop-scrim" onClick={() => void dismiss().catch(report)} />
    <section ref={menu} inert={!visible} aria-hidden={!visible} className={`desktop-menu${visible ? "" : " hidden"}${about ? " about" : ""}${label === "DockItem" ? " dock-item-menu" : ""}`} tabIndex={-1}
      style={about ? undefined : {
        left: `min(${x}px, max(8px, calc(100vw - ${label === "DockItem" ? 236 : 280}px)))`,
      }}
      role={about ? "dialog" : "menu"} aria-modal={about || undefined} aria-label={about ? "About this Meridian" : label === "DockItem" ? `${dockItem?.item_name ?? "Application"} actions` : `${label} menu`}>
      {about ? <>
        <SurfaceButton className="close-about" autoFocus aria-label="Close About window" onClick={() => void dismiss().catch(report)}>×</SurfaceButton>
        <h1 className="meridian-wordmark-heading" aria-label="Meridian"><MeridianLogo /></h1>
        <p className="version">Version {info?.version ?? "…"}</p>
        <p><a className="source-link" href="https://github.com/saikrishna488/meridian">Support / Visit source code</a></p>
        {info ? <dl>{[["Laptop", [info.manufacturer, info.model].filter(Boolean).join(" ") || "Unavailable"], ["Operating system", info.os], ["Processor", info.processor], ["Memory", info.memory]].map(([key, value]) => <div key={key}><dt>{key}</dt><dd>{value}</dd></div>)}</dl> : <p role="status">{error || "Loading laptop details…"}</p>}
      </> : <>
        {label === "Help" && <SurfaceButton ref={first} role="menuitem" onClick={() => void showAbout()}>About this Meridian…</SurfaceButton>}
        {label === "Application" && <SurfaceButton ref={first} role="menuitem" disabled={!focused} onClick={() => {
          if (!focused) return;
          void Promise.all(windows.filter(w => w.name === focused.name).map(w => request({ method: "windows.close", params: { id: w.id } }))).then(dismiss).catch(report);
        }}>Quit {focused?.name ?? "application"}</SurfaceButton>}
        {label === "DockItem" && <>
          <SurfaceButton ref={first} role="menuitem" disabled={!dockItem?.item_id} onClick={() => {
            if (!dockItem?.item_id) return;
            const running = dockItem.window_id && windows.some(window => window.id === dockItem.window_id);
            const action = running
              ? request({ method: "windows.activate", params: { id: dockItem.window_id! } })
              : request({ method: "search.activate", params: { provider: "apps", item_id: dockItem.item_id } });
            void action.then(dismiss).catch(report);
          }}>Open</SurfaceButton>
          <SurfaceButton role="menuitem" disabled={!dockItem?.item_id || ["meridian-settings.desktop", "meridian-pulse.desktop", "meridian-finder.desktop", "meridian-terminal.desktop"].includes(dockItem?.item_id ?? "")} onClick={() => {
            if (!dockItem?.item_id) return;
            void request({ method: "search.activate", params: { provider: "apps", item_id: dockItem.item_id } }).then(dismiss).catch(report);
          }}>{["meridian-settings.desktop", "meridian-pulse.desktop", "meridian-finder.desktop", "meridian-terminal.desktop"].includes(dockItem?.item_id ?? "") ? "New window unavailable" : "New window"}</SurfaceButton>
          <SurfaceButton role="menuitem" disabled={!dockItem?.item_id} onClick={() => {
            if (!dockItem?.item_id) return;
            void request({ method: "desktop.add_app", params: { id: dockItem.item_id } }).then(dismiss).catch(report);
          }}>Add to desktop</SurfaceButton>
          <SurfaceButton role="menuitem" disabled={!windows.some(w => w.name.toLowerCase() === dockItem?.item_name?.toLowerCase())} onClick={() => {
            if (!dockItem?.item_name) return;
            void Promise.all(windows.filter(w => w.name.toLowerCase() === dockItem.item_name?.toLowerCase()).map(w => request({ method: "windows.close", params: { id: w.id } }))).then(dismiss).catch(report);
          }}>Quit</SurfaceButton>
          <SurfaceButton role="menuitem" disabled={!dockItem?.item_id} onClick={() => {
            if (!dockItem?.item_id) return;
            void request({ method: "dock.pin", params: { id: dockItem.item_id, pinned: !dockItem.pinned } }).then(dismiss).catch(report);
          }}>{dockItem?.pinned ? "Remove from panel" : "Pin to panel"}</SurfaceButton>
        </>}
        {label === "File" && <SurfaceButton ref={first} role="menuitem" onClick={() => toggle("applications")}>Open application…</SurfaceButton>}
        {label === "Edit" && <><p className="menu-note">App editing commands are unavailable.</p>{["Undo", "Redo", "Cut", "Copy", "Paste", "Select All"].map(item => <SurfaceButton key={item} role="menuitem" disabled>{item}</SurfaceButton>)}</>}
        {label === "View" && <SurfaceButton ref={first} role="menuitem" onClick={() => toggle("options")}>Control Center…</SurfaceButton>}
        {label === "Window" && (windows.length ? windows.map((w, index) => <SurfaceButton ref={index === 0 ? first : undefined} key={w.id} role="menuitem" onClick={() => void request({ method: "windows.activate", params: { id: w.id } }).then(dismiss).catch(report)}>{w.active ? "✓ " : ""}{w.title || w.name}</SurfaceButton>) : <p className="menu-note">No open windows</p>)}
        {error && <p role="alert">{error}</p>}
      </>}
    </section>
  </>;
}
mount(<DesktopMenu />);
