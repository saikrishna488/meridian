import { useEffect, useRef, useState } from "react";
import { on, request } from "../../lib/bridge";
import { MeridianLogo } from "../../components/MeridianLogo";
import { ConfirmButton } from "../../components/ConfirmButton";
import { mount, useDialogFocus, usePopover, useWindowKey } from "../../lib/react";
import type { MenuOpen } from "../../lib/generated/MenuOpen";
import type { SystemInfo } from "../../lib/generated/SystemInfo";
import type { WindowInfo } from "../../lib/generated/WindowInfo";

function DesktopMenu() {
  const [busy, setBusy] = useState(false);
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
  async function sessionAction(method: "session.sign_out" | "session.lock" | "session.sleep" | "session.restart" | "session.shutdown") {
    if (busy) return;
    setBusy(true); setError("");
    try { await request({ method }); await dismiss(); }
    catch (e) { report(e); }
    finally { setBusy(false); }
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
        <button className="close-about" autoFocus aria-label="Close About window" onClick={() => void dismiss().catch(report)}>×</button>
        <div className="meridian-symbol"><MeridianLogo /></div><h1>Meridian</h1>
        <p className="version">Version {info?.version ?? "…"}</p>
        <p><a className="source-link" href="https://github.com/saikrishna488/meridian">Support / Visit source code</a></p>
        {info ? <dl>{[["Laptop", [info.manufacturer, info.model].filter(Boolean).join(" ") || "Unavailable"], ["Operating system", info.os], ["Processor", info.processor], ["Memory", info.memory]].map(([key, value]) => <div key={key}><dt>{key}</dt><dd>{value}</dd></div>)}</dl> : <p role="status">{error || "Loading laptop details…"}</p>}
      </> : <>
        {(label === "Meridian" || label === "Help") && <button ref={first} role="menuitem" onClick={() => void showAbout()}>About this Meridian…</button>}
        {label === "Meridian" && <><hr /><button role="menuitem" onClick={() => void request({ method: "preferences.open" }).then(dismiss).catch(report)}>Settings…</button><button role="menuitem" onClick={() => void request({ method: "finder.open" }).then(dismiss).catch(report)}>File Manager…</button><button role="menuitem" onClick={() => void request({ method: "terminal.open" }).then(dismiss).catch(report)}>Terminal…</button><button role="menuitem" onClick={() => void request({ method: "pulse.open" }).then(dismiss).catch(report)}>Pulse…</button><hr />
          <button id="sleep" role="menuitem" disabled={busy} onClick={() => void sessionAction("session.sleep")}>Sleep</button>
          <ConfirmButton id="restart" role="menuitem" className="session-menu-button" label="Restart" report={report} action={() => sessionAction("session.restart")} />
          <ConfirmButton id="shutdown" role="menuitem" className="session-menu-button" label="Shut Down" report={report} action={() => sessionAction("session.shutdown")} />
          <hr /><button id="lock" role="menuitem" disabled={busy} onClick={() => void sessionAction("session.lock")}>Lock</button>
          <ConfirmButton id="sign-out" role="menuitem" className="session-menu-button" label="Sign out" report={report} action={() => sessionAction("session.sign_out")} />
        </>}
        {label === "Application" && <button ref={first} role="menuitem" disabled={!focused} onClick={() => {
          if (!focused) return;
          void Promise.all(windows.filter(w => w.name === focused.name).map(w => request({ method: "windows.close", params: { id: w.id } }))).then(dismiss).catch(report);
        }}>Quit {focused?.name ?? "application"}</button>}
        {label === "DockItem" && <>
          <button ref={first} role="menuitem" disabled={!dockItem?.item_id} onClick={() => {
            if (!dockItem?.item_id) return;
            const running = dockItem.window_id && windows.some(window => window.id === dockItem.window_id);
            const action = running
              ? request({ method: "windows.activate", params: { id: dockItem.window_id! } })
              : request({ method: "search.activate", params: { provider: "apps", item_id: dockItem.item_id } });
            void action.then(dismiss).catch(report);
          }}>Open</button>
          <button role="menuitem" disabled={!dockItem?.item_id || ["meridian-settings.desktop", "meridian-pulse.desktop", "meridian-finder.desktop", "meridian-terminal.desktop"].includes(dockItem?.item_id ?? "")} onClick={() => {
            if (!dockItem?.item_id) return;
            void request({ method: "search.activate", params: { provider: "apps", item_id: dockItem.item_id } }).then(dismiss).catch(report);
          }}>{["meridian-settings.desktop", "meridian-pulse.desktop", "meridian-finder.desktop", "meridian-terminal.desktop"].includes(dockItem?.item_id ?? "") ? "New window unavailable" : "New window"}</button>
          <button role="menuitem" disabled={!dockItem?.item_id} onClick={() => {
            if (!dockItem?.item_id) return;
            void request({ method: "desktop.add_app", params: { id: dockItem.item_id } }).then(dismiss).catch(report);
          }}>Add to desktop</button>
          <button role="menuitem" disabled={!windows.some(w => w.name.toLowerCase() === dockItem?.item_name?.toLowerCase())} onClick={() => {
            if (!dockItem?.item_name) return;
            void Promise.all(windows.filter(w => w.name.toLowerCase() === dockItem.item_name?.toLowerCase()).map(w => request({ method: "windows.close", params: { id: w.id } }))).then(dismiss).catch(report);
          }}>Quit</button>
          <button role="menuitem" disabled={!dockItem?.item_id} onClick={() => {
            if (!dockItem?.item_id) return;
            void request({ method: "dock.pin", params: { id: dockItem.item_id, pinned: !dockItem.pinned } }).then(dismiss).catch(report);
          }}>{dockItem?.pinned ? "Remove from panel" : "Pin to panel"}</button>
        </>}
        {label === "File" && <button ref={first} role="menuitem" onClick={() => toggle("applications")}>Open application…</button>}
        {label === "Edit" && <><p className="menu-note">App editing commands are unavailable.</p>{["Undo", "Redo", "Cut", "Copy", "Paste", "Select All"].map(item => <button key={item} role="menuitem" disabled>{item}</button>)}</>}
        {label === "View" && <button ref={first} role="menuitem" onClick={() => toggle("options")}>Control Center…</button>}
        {label === "Window" && (windows.length ? windows.map((w, index) => <button ref={index === 0 ? first : undefined} key={w.id} role="menuitem" onClick={() => void request({ method: "windows.activate", params: { id: w.id } }).then(dismiss).catch(report)}>{w.active ? "✓ " : ""}{w.title || w.name}</button>) : <p className="menu-note">No open windows</p>)}
        {error && <p role="alert">{error}</p>}
      </>}
    </section>
  </>;
}
mount(<DesktopMenu />);
