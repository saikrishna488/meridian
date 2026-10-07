import { useEffect, useRef, useState } from "react";
import { on, request } from "../../lib/bridge";
import { ConfirmButton } from "../../components/ConfirmButton";
import { mount, usePopover, useWindowKey } from "../../lib/react";
import type { SystemInfo } from "../../lib/generated/SystemInfo";
import type { WindowInfo } from "../../lib/generated/WindowInfo";

function DesktopMenu() {
  const [busy, setBusy] = useState(false);
  const [label, setLabel] = useState("Meridian");
  const [x, setX] = useState(8);
  const [about, setAbout] = useState(false);
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [error, setError] = useState("");
  const first = useRef<HTMLButtonElement>(null);
  const report = (e: unknown) => setError(e instanceof Error ? e.message : "Couldn’t complete this action");
  const { visible, menu, dismiss } = usePopover(async () => { setAbout(false); setError(""); }, () => first.current?.focus(), report);
  useEffect(() => on("menu.opened", e => {
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
    <div className="desktop-scrim" onClick={() => void dismiss().catch(report)} />
    <section ref={menu} className={`desktop-menu${visible ? "" : " hidden"}${about ? " about" : ""}`}
      style={about ? undefined : { left: `min(${x}px, max(8px, calc(100vw - 280px)))` }}
      role={about ? "dialog" : "menu"} aria-modal={about || undefined} aria-label={about ? "About this Meridian" : `${label} menu`}>
      {about ? <>
        <button className="close-about" autoFocus aria-label="Close About window" onClick={() => void dismiss().catch(report)}>×</button>
        <div className="meridian-symbol" aria-hidden="true">◈</div><h1>Meridian</h1>
        <p className="version">Version {info?.version ?? "…"}</p>
        <p><a className="source-link" href="https://github.com/saikrishna488/meridian">Support / Visit source code</a></p>
        {info ? <dl>{[["Laptop", [info.manufacturer, info.model].filter(Boolean).join(" ") || "Unavailable"], ["Operating system", info.os], ["Processor", info.processor], ["Memory", info.memory]].map(([key, value]) => <div key={key}><dt>{key}</dt><dd>{value}</dd></div>)}</dl> : <p role="status">{error || "Loading laptop details…"}</p>}
      </> : <>
        {(label === "Meridian" || label === "Help") && <button ref={first} role="menuitem" onClick={() => void showAbout()}>About this Meridian…</button>}
        {label === "Meridian" && <><hr /><button role="menuitem" onClick={() => void request({ method: "preferences.open" }).then(dismiss).catch(report)}>Settings…</button><button role="menuitem" onClick={() => void request({ method: "finder.open" }).then(dismiss).catch(report)}>Finder…</button><button role="menuitem" onClick={() => void request({ method: "terminal.open" }).then(dismiss).catch(report)}>Terminal…</button><hr />
          <button id="sleep" role="menuitem" disabled={busy} onClick={() => void sessionAction("session.sleep")}>Sleep</button>
          <ConfirmButton id="restart" role="menuitem" className="session-menu-button" label="Restart" report={report} action={() => sessionAction("session.restart")} />
          <ConfirmButton id="shutdown" role="menuitem" className="session-menu-button" label="Shut Down" report={report} action={() => sessionAction("session.shutdown")} />
          <hr /><button id="lock" role="menuitem" disabled={busy} onClick={() => void sessionAction("session.lock")}>Lock</button>
          <ConfirmButton id="sign-out" role="menuitem" className="session-menu-button" label="Sign out" report={report} action={() => sessionAction("session.sign_out")} />
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
