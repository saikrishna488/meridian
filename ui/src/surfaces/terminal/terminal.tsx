import { useEffect, useRef, useState } from "react";
import { hello, on, request } from "../../lib/bridge";
import { errorMessage, mount, useWindowKey } from "../../lib/react";
import { currentAppearance, useAppearance } from "../../lib/appearance";
import { Terminal as Emulator } from "../../vendor/xterm";
import { FitAddon } from "../../vendor/addon-fit";
import { Icon } from "../../components/Icon";
import { wheelScroller } from "./scroll";
import { terminalTheme } from "./theme";

function colors() { return terminalTheme(currentAppearance()); }
function Session({ active, fontSize, cwd, appearance, onExit }: { active: boolean; fontSize: number; cwd?: string; appearance: string; onExit: () => void }) {
  const container = useRef<HTMLDivElement>(null);
  const emulator = useRef<Emulator | null>(null);
  const fit = useRef<FitAddon | null>(null);
  const activeRef = useRef(active); activeRef.current = active;
  const refit = useRef<(() => void) | null>(null);
  const [error, setError] = useState("");
  const [scrolledUp, setScrolledUp] = useState(false);
  const exit = useRef(onExit); exit.current = onExit;
  useEffect(() => {
    let alive = true, id: number | null = null, timer: ReturnType<typeof setTimeout> | undefined;
    let observer: ResizeObserver | undefined;
    let removePaste: (() => void) | undefined;
    let frame = 0;
    const repaint = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const element = container.current, term = emulator.current;
        if (!alive || !activeRef.current || !element?.clientWidth || !element.clientHeight || !term) return;
        const size = fit.current?.proposeDimensions();
        if (size && Number.isFinite(size.cols) && Number.isFinite(size.rows)) {
          const cols = Math.max(2, Math.min(500, size.cols)), rows = Math.max(1, Math.min(250, size.rows));
          if (cols !== term.cols || rows !== term.rows) term.resize(cols, rows);
        }
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
        const term = new Emulator({ fontSize, fontFamily: '"DejaVu Sans Mono", "Cascadia Code", monospace', cursorBlink: true, cursorStyle: "bar", letterSpacing: 0, lineHeight: 1, scrollback: 5000, allowProposedApi: false, minimumContrastRatio: 4.5, theme: colors() });
        const addon = new FitAddon(); emulator.current = term; fit.current = addon;
        term.loadAddon(addon); term.open(container.current!);
        // Own native/context-menu paste before xterm's textarea and container
        // listeners can handle the same bubbling event.
        const paste = (event: ClipboardEvent) => {
          event.preventDefault();
          event.stopPropagation();
          const text = event.clipboardData?.getData("text/plain");
          if (text) term.paste(text);
        };
        const screen = container.current!;
        screen.addEventListener("paste", paste, true);
        removePaste = () => screen.removeEventListener("paste", paste, true);
        const scroll = wheelScroller();
        const updateScroll = () => { if (alive) setScrolledUp(term.buffer.active.type === "normal" && term.buffer.active.viewportY < term.buffer.active.baseY); };
        term.onScroll(updateScroll);
        // Handle normal scrollback directly rather than relying on WebKit's
        // asynchronous native viewport scrolling. Leave application mouse modes alone.
        term.attachCustomWheelEventHandler(event => {
          if (event.ctrlKey || term.buffer.active.type !== "normal" || term.modes.mouseTrackingMode !== "none" || Math.abs(event.deltaX) > Math.abs(event.deltaY)) return true;
          const height = container.current?.querySelector<HTMLElement>(".xterm-screen")?.clientHeight ?? 0;
          const lines = scroll(event.deltaY, event.deltaMode, height ? height / term.rows : fontSize, term.rows);
          if (lines) term.scrollLines(lines);
          event.preventDefault();
          return false;
        });
        let inputQueue = Promise.resolve();
        term.onData(data => { inputQueue = inputQueue.then(async () => { if (alive && id !== null) await request({ method: "terminal.write", params: { id, data } }); }).catch(fail); });
        term.onResize(({ cols, rows }) => { inputQueue = inputQueue.then(async () => { if (alive && id !== null && cols >= 2 && rows >= 1) await request({ method: "terminal.resize", params: { id, cols: Math.min(500, cols), rows: Math.min(250, rows) } }); }).catch(fail); });
        // Ctrl+C copies a selection, or interrupts the shell without one.
        term.attachCustomKeyEventHandler(event => {
          // Let the application own tab shortcuts before xterm can send a
          // control character to the foreground shell.
          if (event.ctrlKey && ((event.shiftKey && ["t", "w"].includes(event.key.toLowerCase())) || ["PageUp", "PageDown"].includes(event.key))) return false;
          if (event.ctrlKey && !event.altKey && !event.metaKey) {
            const key = event.key.toLowerCase();
            if (key === "c" && (event.shiftKey || term.getSelection())) {
              event.preventDefault();
              if (event.type === "keydown" && !event.repeat) {
                const text = term.getSelection();
                if (text) void request({ method: "terminal.copy", params: { data: text } }).catch(fail);
              }
              return false;
            }
            if (key === "v") {
              // Returning false only stops xterm; cancel WebKit's native paste
              // as well, so the clipboard is inserted exactly once.
              event.preventDefault();
              if (event.type === "keydown" && !event.repeat) {
                void request({ method: "terminal.paste" }).then(text => { if (alive && text) term.paste(text); }).catch(fail);
              }
              return false;
            }
          }
          if (event.type === "keydown" && event.shiftKey && !event.ctrlKey && !event.altKey && ["PageUp", "PageDown", "Home", "End"].includes(event.key)) {
            event.preventDefault();
            if (event.key === "Home") term.scrollToTop();
            else if (event.key === "End") term.scrollToBottom();
            else term.scrollPages(event.key === "PageUp" ? -1 : 1);
            return false;
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
            if (chunk.data.length) { await new Promise<void>(resolve => term.write(new Uint8Array(chunk.data), resolve)); updateScroll(); }
            if (chunk.exited) { term.writeln("\r\n[Session ended]"); term.options.disableStdin = true; await request({ method: "terminal.close", params: { id } }); id = null; exit.current(); return; }
            timer = setTimeout(() => void poll(), chunk.data.length ? 0 : 25);
          } catch (e) { fail(e); }
        }
        void poll();
      } catch (e) { fail(e); }
    }
    void start();
    return () => { alive = false; clearTimeout(timer); cancelAnimationFrame(frame); window.removeEventListener("resize", repaint); document.removeEventListener("fullscreenchange", repaint); window.removeEventListener("focus", repaint); window.removeEventListener("meridian-theme", theme); refit.current = null; observer?.disconnect(); removePaste?.(); emulator.current?.dispose(); emulator.current = null; fit.current = null; if (id !== null) void request({ method: "terminal.close", params: { id } }).catch(() => {}); };
  }, []);
  useEffect(() => {
    if (emulator.current) {
      emulator.current.options.fontSize = fontSize;
      // Repaint existing terminal cells when the desktop appearance changes.
      emulator.current.options.theme = colors();
      if (emulator.current.rows) emulator.current.refresh(0, emulator.current.rows - 1);
    }
    if (active) refit.current?.();
  }, [active, fontSize, appearance]);
  return <section className={`terminal-session${active ? " active" : ""}`} role="tabpanel" aria-label="Terminal output" aria-hidden={!active}>{scrolledUp && <button className="terminal-follow" onClick={() => { emulator.current?.scrollToBottom(); emulator.current?.focus(); }}>Jump to latest output</button>}<div className="terminal-screen" ref={container} aria-label="Interactive terminal" />{error && <p className="terminal-error" role="alert">{error}</p>}</section>;
}
function Terminal() {
  const appearance = useAppearance();
  const [ready, setReady] = useState(false), [error, setError] = useState("");
  const [tabs, setTabs] = useState<{ id: number; ended: boolean; cwd?: string }[]>([]), [active, setActive] = useState(1), [fontSize, setFontSize] = useState(() => { try { const value = Number(localStorage.getItem("meridian.terminal-font-size")); return Number.isInteger(value) && value >= 11 && value <= 24 ? value : 13; } catch { return 13; } });
  useEffect(() => { try { localStorage.setItem("meridian.terminal-font-size", String(fontSize)); } catch {} }, [fontSize]);
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
  useWindowKey(event => {
    if (!event.ctrlKey) return;
    if (event.shiftKey && event.key.toLowerCase() === "t" && ready && tabs.length < 16) { event.preventDefault(); newTab(); }
    if (event.shiftKey && event.key.toLowerCase() === "w" && active) { event.preventDefault(); close(active); }
    if (["PageUp", "PageDown"].includes(event.key) && tabs.length) {
      event.preventDefault(); const index = tabs.findIndex(t => t.id === active);
      setActive(tabs[(index + (event.key === "PageDown" ? 1 : tabs.length - 1)) % tabs.length]!.id);
    }
  });
  return <main className="terminal-app"><header className="terminal-tabs"><div role="tablist" aria-label="Terminal sessions">{tabs.map(tab => <div className="terminal-tab" key={tab.id}><button role="tab" aria-selected={tab.id === active} tabIndex={tab.id === active ? 0 : -1} onKeyDown={event => { if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) { event.preventDefault(); const index = tabs.findIndex(t => t.id === tab.id); const target = event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : (index + (event.key === "ArrowRight" ? 1 : tabs.length - 1)) % tabs.length; setActive(tabs[target]!.id); event.currentTarget.closest('[role="tablist"]')?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[target]?.focus(); } }} onClick={() => setActive(tab.id)}><span className={`session-dot${tab.ended ? " ended" : ""}`} />Session {tab.id}{tab.ended ? " · ended" : ""}</button><button aria-label={`Close session ${tab.id}`} onClick={() => close(tab.id)}><Icon name="close" width="12" height="12" /></button></div>)}</div><button className="new-session" aria-label="New terminal session" disabled={!ready || tabs.length >= 16} onClick={() => newTab()} title="New terminal session (Ctrl+Shift+T)"><Icon name="plus" width="17" height="17" /></button><div className="terminal-tools"><button aria-label="Decrease text size" disabled={fontSize <= 11} onClick={() => setFontSize(fontSize - 1)}>A−</button><button aria-label="Increase text size" disabled={fontSize >= 24} onClick={() => setFontSize(fontSize + 1)}>A+</button></div></header>
    {error && <p role="alert">{error}</p>}{ready && tabs.map(tab => <Session key={tab.id} active={tab.id === active} fontSize={fontSize} cwd={tab.cwd} appearance={appearance} onExit={() => setTabs(old => old.map(t => t.id === tab.id ? { ...t, ended: true } : t))} />)}{!tabs.length && <div className="terminal-empty"><Icon name="terminal" width="40" height="40" /><h1>Ready when you are</h1><p>Open a terminal to start a new shell session.</p><span className="terminal-shortcut">Ctrl + Shift + T</span><button onClick={() => newTab()}>Open a terminal</button></div>}
  </main>;
}
mount(<Terminal />);
