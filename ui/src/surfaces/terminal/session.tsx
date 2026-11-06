import { Button, Dialog, SearchField, Switch, SurfaceButton } from "../../meridian-ui";
import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import { errorMessage, useMenuFocus } from "../../lib/react";
import type { Preferences, Profile } from "./preferences";
import type { TerminalInfo } from "../../lib/generated/TerminalInfo";
import { searchBuffer, revealMatch, type Match } from "./search";
import { Terminal as Emulator } from "../../vendor/xterm";
import { FitAddon } from "../../vendor/addon-fit";
import { Icon } from "../../components/Icon";
import { wheelScroller } from "./scroll";
import { terminalTheme } from "./theme";

export function Session({ active, focused, cwd, appearance, preferences, profile, searchRequest, onFocus, onReady, onState, onExit }: {
  active:boolean; focused:boolean; cwd?:string; appearance:string; preferences:Preferences; profile:Profile; searchRequest:number;
  onFocus:()=>void; onReady:(id:number)=>void; onState:(info:TerminalInfo)=>void; onExit:()=>void;
}) {
  const fontSize = preferences.size;
  const callbacks = useRef({onReady,onState,onExit}); callbacks.current={onReady,onState,onExit};
  const current = useRef({focused,preferences,appearance}); current.current={focused,preferences,appearance};
  const backendId = useRef<number | null>(null);
  const [finding,setFinding] = useState(false), [query,setQuery] = useState(""), [sensitive,setSensitive] = useState(false), [regex,setRegex] = useState(false);
  const [matches,setMatches] = useState<Match[]>([]), [matchIndex,setMatchIndex] = useState(-1), [searchError,setSearchError] = useState("");
  const [searchRevision,setSearchRevision] = useState(0), [link,setLink] = useState<string | null>(null), [commandStatus,setCommandStatus] = useState("");
  const searchSnapshot = useRef({length:0,base:0});
  const searchInput = useRef<HTMLInputElement>(null), searching = useRef<()=>void>(()=>{});
  useEffect(()=>{if(searchRequest){setFinding(true);}},[searchRequest]);
  useEffect(()=>{if(finding) searchInput.current?.focus();},[finding]);
  useEffect(()=>{
    searching.current(); setSearchError(""); setMatches([]);setMatchIndex(-1);
    if(!finding || !query || !emulator.current) return;
    let live=true;
    const timer=setTimeout(()=>{
      try {
        searchSnapshot.current={length:emulator.current!.buffer.active.length,base:emulator.current!.buffer.active.baseY};
        const job=searchBuffer(emulator.current!,query,sensitive,regex);searching.current=job.cancel;
        void job.promise.then(found=>{if(live){setMatches(found);setMatchIndex(found.length ? 0:-1);if(found[0] && emulator.current) revealMatch(emulator.current,found[0]);}}).catch(e=>{if(live)setSearchError(errorMessage(e));});
      } catch(e) {setSearchError(errorMessage(e));}
    },150);
    return ()=>{live=false;clearTimeout(timer);searching.current();};
  },[finding,query,sensitive,regex,searchRevision]);
  function findNext(direction:number) { if(!matches.length || !emulator.current) return; if(searchSnapshot.current.length!==emulator.current.buffer.active.length || searchSnapshot.current.base!==emulator.current.buffer.active.baseY){setSearchRevision(n=>n+1);return;}const index=(matchIndex+direction+matches.length)%matches.length;setMatchIndex(index);revealMatch(emulator.current,matches[index]!); }
  function closeSearch() {setFinding(false);emulator.current?.clearSelection();emulator.current?.focus();}
  async function openLink(folder:boolean) {
    if(!link || backendId.current===null) return;
    try {await request({method:"terminal.link",params:{id:backendId.current,target:link,folder}});setLink(null);}
    catch(e){setError(errorMessage(e));setLink(null);}
  }
  const container = useRef<HTMLDivElement>(null);
  const emulator = useRef<Emulator | null>(null);
  const fit = useRef<FitAddon | null>(null);
  const activeRef = useRef(active); activeRef.current = active;
  const refit = useRef<(() => void) | null>(null);
  const [error, setError] = useState("");
  const [scrolledUp, setScrolledUp] = useState(false);
  const [context, setContext] = useState<{ x: number; y: number } | null>(null);
  const contextMenu = useRef<HTMLDivElement>(null);
  useMenuFocus(contextMenu, !!context, () => setContext(null));
  useEffect(() => {
    if (!context) return;
    const dismiss = (event: PointerEvent) => { if (!contextMenu.current?.contains(event.target as Node)) setContext(null); };
    document.addEventListener("pointerdown", dismiss);
    return () => document.removeEventListener("pointerdown", dismiss);
  }, [context]);
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
        if (current.current.focused && !document.querySelector(".terminal-find input:focus, [role=dialog]")) term.focus();
      });
    };
    refit.current = repaint;
    window.addEventListener("resize", repaint);
    document.addEventListener("fullscreenchange", repaint);
    window.addEventListener("focus", repaint);
    const theme = () => { if (emulator.current) emulator.current.options.theme = terminalTheme(current.current.preferences.theme === "system" ? current.current.appearance : current.current.preferences.theme); repaint(); };
    window.addEventListener("meridian-theme", theme);
    const fail = (e: unknown) => { if (alive) setError(errorMessage(e)); };
    async function start() {
      try {
        const session = await request({method:"terminal.launch",params:{cwd:cwd || profile.cwd || null,shell:profile.shell || null,ssh_host:profile.host || null,ssh_port:profile.host ? profile.port : null,integration:profile.integration}});
        id = session.id; backendId.current = id; callbacks.current.onReady(id);
        if (!alive) { await request({ method: "terminal.close", params: { id } }); return; }
        const term = new Emulator({ fontSize, fontFamily: `${JSON.stringify(preferences.font)}, monospace`, cursorBlink: true, cursorStyle: preferences.cursor, letterSpacing: 0, lineHeight: 1, scrollback: preferences.scrollback, allowProposedApi: false, minimumContrastRatio: 4.5, theme: terminalTheme(preferences.theme === "system" ? appearance : preferences.theme), screenReaderMode:true, linkHandler:{activate:(event:MouseEvent,uri:string)=>{event.preventDefault();if(event.ctrlKey || event.metaKey)setLink(uri);}} });
        const addon = new FitAddon(); emulator.current = term; fit.current = addon;
        term.loadAddon(addon); term.open(container.current!);
        let commandStarted:number | null=null;
        term.parser.registerOscHandler(133,data=>{
          if(data === "C") {commandStarted=performance.now();setCommandStatus("Command running…");}
          if(data.startsWith("D;") && commandStarted!==null) {
            const seconds=(performance.now()-commandStarted)/1000, code=Number(data.slice(2).split(";")[0]);commandStarted=null;
            const summary=`Exit ${Number.isFinite(code) ? code : "unknown"} · ${seconds.toFixed(1)}s`;
            setCommandStatus(summary);
            if(seconds>=10 && current.current.preferences.notifications && (!current.current.focused || !document.hasFocus())) void request({method:"terminal.notify",params:{data:summary}}).catch(fail);
          }
          return true;
        });
        term.registerLinkProvider({provideLinks(y,callback){
          const line=term.buffer.active.getLine(y-1), text=line?.translateToString(true) ?? "";
          const pattern=/(?:https?:\/\/[^\s<>"']+|(?:\/|~\/|\.\.?\/)[^\s<>"']+|[\w.-]+\/[\w./-]+)/g;
          const links=[];let match;
          const column=(offset:number)=>{let consumed=0;for(let x=0;x<term.cols;x++){const cell=line?.getCell(x);if(!cell || cell.getWidth()===0)continue;consumed+=(cell.getChars()||" ").length;if(consumed>offset)return x+1;}return term.cols;};
          while((match=pattern.exec(text)) && links.length<100) {
            const target=match[0].replace(/[),;]+$/,"");
            links.push({text:target,range:{start:{x:column(match.index),y},end:{x:column(match.index+target.length-1),y}},activate:(event:MouseEvent,value:string)=>{event.preventDefault();if(event.ctrlKey||event.metaKey)setLink(value);}});
          }
          callback(links);
        }});
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
          if (event.ctrlKey && ((event.shiftKey && ["t", "w", "d", "e", "f", "q", "Enter", "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].map(key=>key.toLowerCase()).includes(event.key.toLowerCase())) || ["PageUp", "PageDown"].includes(event.key))) return false;
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
        let infoAt=0;
        async function poll() {
          if (!alive || id === null) return;
          try {
            const chunk = await request({ method: "terminal.read", params: { id } });
            if (!alive) return;
            if (chunk.data.length) { await new Promise<void>(resolve => term.write(new Uint8Array(chunk.data), resolve)); updateScroll(); }
            if(Date.now()-infoAt>1000 || chunk.exited) {infoAt=Date.now();const info=await request({method:"terminal.info",params:{id}});if(alive)callbacks.current.onState(info);}
            if (chunk.exited) { term.writeln("\r\n[Session ended — output preserved]"); term.options.disableStdin = true; exit.current(); return; }
            timer = setTimeout(() => void poll(), chunk.data.length ? 0 : activeRef.current ? 80 : 350);
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
      emulator.current.options.theme = terminalTheme(preferences.theme === "system" ? appearance : preferences.theme);
      emulator.current.options.fontFamily = `${JSON.stringify(preferences.font)}, monospace`;
      emulator.current.options.cursorStyle = preferences.cursor;
      emulator.current.options.scrollback = preferences.scrollback;
      if (emulator.current.rows) emulator.current.refresh(0, emulator.current.rows - 1);
    }
    if (active) refit.current?.();
  }, [active, focused, fontSize, appearance, preferences]);
  return <section className={`terminal-session${active ? " active" : ""}`} role="tabpanel" aria-label="Terminal output" aria-hidden={!active} onFocusCapture={onFocus} onPointerDown={onFocus} onDragOver={event=>{if(event.dataTransfer.types.includes("text/uri-list")||event.dataTransfer.types.includes("Files"))event.preventDefault();}} onDrop={event=>{
    const paths:string[]=[];
    for(const line of event.dataTransfer.getData("text/uri-list").split(/\r?\n/)){try{const uri=new URL(line);if(uri.protocol==="file:" && (!uri.hostname||uri.hostname==="localhost"))paths.push(decodeURIComponent(uri.pathname));}catch{}}
    for(const file of Array.from(event.dataTransfer.files)){const path=(file as File & {path?:string}).path;if(path)paths.push(path);}
    if(paths.length){event.preventDefault();emulator.current?.paste([...new Set(paths)].map(path=>"'"+path.replaceAll("'", "'\\''")+"'").join(" "));}
  }} onContextMenu={event => { event.preventDefault(); event.stopPropagation(); setContext({ x: event.clientX, y: event.clientY }); }}>
    {finding && <div className="terminal-find" onKeyDown={event=>{event.stopPropagation();if(event.key==="Escape"){event.preventDefault();closeSearch();}if(event.key==="Enter"){event.preventDefault();findNext(event.shiftKey ? -1:1);}}}>
      <SearchField ref={searchInput} aria-label="Find in terminal output" value={query} onValueChange={setQuery}/><label><input type="checkbox" checked={sensitive} onChange={e=>setSensitive(e.target.checked)}/>Aa</label><label><input type="checkbox" checked={regex} onChange={e=>setRegex(e.target.checked)}/>Regex</label>
      <Button aria-label="Previous match" disabled={!matches.length} onClick={()=>findNext(-1)}>↑</Button><Button aria-label="Next match" disabled={!matches.length} onClick={()=>findNext(1)}>↓</Button><Button title="Search again to include new output" onClick={()=>setSearchRevision(n=>n+1)}>Refresh</Button><Button aria-label="Close search" onClick={closeSearch}>×</Button>
      <span role="status">{searchError || (query ? `${matchIndex+1} / ${matches.length}${matches.length===5000 ? "+":""}`:"")}</span>
    </div>}
    {scrolledUp && <SurfaceButton className="terminal-follow" onClick={() => { emulator.current?.scrollToBottom(); emulator.current?.focus(); }}>Jump to latest output</SurfaceButton>}<div className="terminal-screen" ref={container} aria-label="Interactive terminal" />{commandStatus && <small className="terminal-command-status" role="status">{commandStatus}</small>}{error && <p className="terminal-error" role="alert">{error}</p>}
    {context && <div ref={contextMenu} className="terminal-context" role="menu" aria-label="Terminal actions" style={{ left: Math.max(8, Math.min(context.x, window.innerWidth - 190)), top: Math.max(8, Math.min(context.y, window.innerHeight - 150)) }}>
      <SurfaceButton role="menuitem" disabled={!emulator.current?.hasSelection()} onClick={() => { const text = emulator.current?.getSelection(); if (text) void request({ method: "terminal.copy", params: { data: text } }).catch(e => setError(errorMessage(e))); setContext(null); }}>Copy</SurfaceButton>
      <SurfaceButton role="menuitem" onClick={() => { void request({ method: "terminal.paste" }).then(text => { if (text) emulator.current?.paste(text); }).catch(e => setError(errorMessage(e))); setContext(null); }}>Paste</SurfaceButton>
      <SurfaceButton role="menuitem" onClick={()=>{setContext(null);setFinding(true);}}>Find in output…</SurfaceButton>
      <SurfaceButton role="menuitem" disabled={!emulator.current?.hasSelection()} onClick={()=>{const value=emulator.current?.getSelection().trim();if(value)setLink(value);setContext(null);}}>Open selected URL or path…</SurfaceButton>
      <SurfaceButton role="menuitem" onClick={() => { emulator.current?.selectAll(); emulator.current?.focus(); setContext(null); }}>Select all</SurfaceButton>
    </div>}
    {link && <Dialog open aria-label="Open terminal link" onClose={()=>setLink(null)}><h2>Open link</h2><p className="terminal-link-target">{link}</p><p>Open only paths or addresses you recognize. URLs require http or https.</p><div className="muk-dialog-actions"><Button onClick={()=>setLink(null)}>Cancel</Button>{!/^https?:\/\//.test(link) && <Button onClick={()=>void openLink(true)}>Open in Files</Button>}<Button variant="primary" onClick={()=>void openLink(false)}>{/^https?:\/\//.test(link) ? "Open in browser":"Open in editor"}</Button></div></Dialog>}
  </section>;
}
