import { useEffect, useRef, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { errorMessage, mount } from "../../lib/react";
import { useAppearance } from "../../lib/appearance";
import { Terminal as Emulator } from "../../vendor/xterm";
import { FitAddon } from "../../vendor/addon-fit";
import { terminalTheme } from "./theme";

function colors() { return terminalTheme("light"); }
function Session({ active, fontSize, cwd, appearance, onExit }: { active: boolean; fontSize: number; cwd?: string; appearance: string; onExit: () => void }) {
  const container = useRef<HTMLDivElement>(null);
  const emulator = useRef<Emulator | null>(null);
  const fit = useRef<FitAddon | null>(null);
  const activeRef = useRef(active); activeRef.current = active;
  const refit = useRef<(() => void) | null>(null);
  const [error, setError] = useState("");
  const [shell, setShell] = useState("Starting shell…");
  const exit = useRef(onExit); exit.current = onExit;
  useEffect(() => {
    let alive = true, id: number | null = null, timer: ReturnType<typeof setTimeout> | undefined;
    let observer: ResizeObserver | undefined;
    let frame = 0;
    const repaint = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const element = container.current, term = emulator.current;
        if (!alive || !activeRef.current || !element?.clientWidth || !element.clientHeight || !term) return;
        const size = fit.current?.proposeDimensions();
        if (size && Number.isFinite(size.cols) && Number.isFinite(size.rows)) term.resize(Math.max(2, Math.min(500, size.cols)), Math.max(1, Math.min(250, size.rows)));
        term.refresh(0, term.rows - 1);
        term.focus();
      });
    };
    refit.current = repaint;
    window.addEventListener("resize", repaint);
    document.addEventListener("fullscreenchange", repaint);
    window.addEventListener("focus", repaint);
    const theme = () => { if (emulator.current) emulator.current.options.theme = colors(); repaint(); };
    window.addEventListener("meridian-theme", theme);
    const fail = (e: unknown) => { if (alive) setError(errorMessage(e)); };
    async function start() {
      try {
        const session = cwd ? await request({ method: "terminal.start_at", params: { path: cwd } }) : await request({ method: "terminal.start" });
        id = session.id;
        if (!alive) { await request({ method: "terminal.close", params: { id } }); return; }
        setShell(session.shell);
        const term = new Emulator({ fontSize, fontFamily: '"DejaVu Sans Mono", "Cascadia Code", monospace', cursorBlink: true, cursorStyle: "bar", letterSpacing: 0, lineHeight: 1, scrollback: 5000, allowProposedApi: false, minimumContrastRatio: 4.5, theme: colors() });
        const addon = new FitAddon(); emulator.current = term; fit.current = addon;
        term.loadAddon(addon); term.open(container.current!);
        let inputQueue = Promise.resolve();
        term.onData(data => { inputQueue = inputQueue.then(async () => { if (alive && id !== null) await request({ method: "terminal.write", params: { id, data } }); }).catch(fail); });
        term.onResize(({ cols, rows }) => { inputQueue = inputQueue.then(async () => { if (alive && id !== null && cols >= 2 && rows >= 1) await request({ method: "terminal.resize", params: { id, cols: Math.min(500, cols), rows: Math.min(250, rows) } }); }).catch(fail); });
        // Let the shell receive Ctrl+C; copy a selection with Ctrl+Shift+C.
        term.attachCustomKeyEventHandler(event => {
          if (event.type === "keydown" && event.ctrlKey && event.shiftKey && event.key.toLowerCase() === "c") {
            const text = term.getSelection(); if (text) void request({ method: "terminal.copy", params: { data: text } }).catch(fail); return false;
          }
          if (event.type === "keydown" && event.ctrlKey && event.shiftKey && event.key.toLowerCase() === "v") {
            void request({ method: "terminal.paste" }).then(text => { if (alive && text) term.paste(text); }).catch(fail); return false;
          }
          return true;
        });
        observer = new ResizeObserver(repaint);
        observer.observe(container.current!);
        if (document.fonts) await document.fonts.ready;
        if (!alive) return;
        repaint();
        async function poll() {
          if (!alive || id === null) return;
          try {
            const chunk = await request({ method: "terminal.read", params: { id } });
            if (!alive) return;
            if (chunk.data.length) await new Promise<void>(resolve => term.write(new Uint8Array(chunk.data), resolve));
            if (chunk.exited) { term.writeln("\r\n[Session ended]"); term.options.disableStdin = true; await request({ method: "terminal.close", params: { id } }); id = null; exit.current(); return; }
            timer = setTimeout(() => void poll(), chunk.data.length ? 0 : 25);
          } catch (e) { fail(e); }
        }
        void poll();
      } catch (e) { fail(e); }
    }
    void start();
    return () => { alive = false; clearTimeout(timer); cancelAnimationFrame(frame); window.removeEventListener("resize", repaint); document.removeEventListener("fullscreenchange", repaint); window.removeEventListener("focus", repaint); window.removeEventListener("meridian-theme", theme); refit.current = null; observer?.disconnect(); emulator.current?.dispose(); emulator.current = null; fit.current = null; if (id !== null) void request({ method: "terminal.close", params: { id } }).catch(() => {}); };
  }, []);
  useEffect(() => {
    if (emulator.current) {
      emulator.current.options.fontSize = fontSize;
      // Reapply the explicit readable palette and repaint the canvas after
      // appearance changes; terminal text remains black on white by design.
      emulator.current.options.theme = colors();
      if (emulator.current.rows) emulator.current.refresh(0, emulator.current.rows - 1);
    }
    if (active) refit.current?.();
  }, [active, fontSize, appearance]);
  return <section className={`terminal-session${active ? " active" : ""}`} aria-hidden={!active}><div className="terminal-screen" ref={container} aria-label="Interactive terminal" />{error && <p className="terminal-error" role="alert">{error}</p>}<footer className="terminal-status"><span><i />{shell}</span><span>UTF-8 · Interactive shell</span></footer></section>;
}
function Terminal() {
  const appearance = useAppearance();
  const [ready, setReady] = useState(false), [error, setError] = useState("");
  const [tabs, setTabs] = useState<{ id: number; ended: boolean; cwd?: string }[]>([]), [active, setActive] = useState(1), [fontSize, setFontSize] = useState(13);
  const next = useRef(1);
  const tabsRef = useRef(tabs); tabsRef.current = tabs;
  useEffect(() => {
    const opened = on("terminal.opened", () => { if (!tabsRef.current.length) newTab(); });
    const openedAt = on("terminal.opened_at", event => newTab(event.data.path));
    const closed = on("terminal.closed", () => { tabsRef.current = []; setTabs([]); setActive(0); });
    void hello().then(() => setReady(true)).catch(e => setError(errorMessage(e)));
    return () => { opened(); openedAt(); closed(); };
  }, []);
  function newTab(cwd?: string) { const id = next.current++; tabsRef.current = [...tabsRef.current, { id, ended: false, cwd }]; setTabs(tabsRef.current); setActive(id); }
  function close(id: number) { setTabs(old => old.filter(t => t.id !== id)); if (active === id) setActive(tabs.find(t => t.id !== id)?.id ?? 0); }
  return <main className="terminal-app"><header className="terminal-tabs"><div role="tablist" aria-label="Terminal sessions">{tabs.map(tab => <div className="terminal-tab" key={tab.id}><button role="tab" aria-selected={tab.id === active} onClick={() => setActive(tab.id)}><span className="session-dot" />Session {tab.id}{tab.ended ? " · ended" : ""}</button><button aria-label={`Close session ${tab.id}`} onClick={() => close(tab.id)}>×</button></div>)}</div><button className="new-session" aria-label="New terminal session" disabled={!ready || tabs.length >= 16} onClick={() => newTab()}>＋</button><div className="terminal-tools"><button aria-label="Decrease text size" disabled={fontSize <= 11} onClick={() => setFontSize(fontSize - 1)}>A−</button><button aria-label="Increase text size" disabled={fontSize >= 24} onClick={() => setFontSize(fontSize + 1)}>A+</button></div></header>
    {error && <p role="alert">{error}</p>}{ready && tabs.map(tab => <Session key={tab.id} active={tab.id === active} fontSize={fontSize} cwd={tab.cwd} appearance={appearance} onExit={() => setTabs(old => old.map(t => t.id === tab.id ? { ...t, ended: true } : t))} />)}{!tabs.length && <div className="terminal-empty"><p>All sessions closed.</p><button onClick={() => newTab()}>Open a terminal</button></div>}
  </main>;
}
mount(<Terminal />);
