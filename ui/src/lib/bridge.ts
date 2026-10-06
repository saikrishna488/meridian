// Client side of the UI ⇄ host bridge.
//
// Requests go through WebKit's script message handler with reply; events
// arrive through `window.__meridianDispatch`, which the host calls with the
// serialized event as an argument. All message shapes come from the
// generated protocol types.

import type { ErrorBody } from "./generated/ErrorBody";
import type { GreeterInfo } from "./generated/GreeterInfo";
import type { Event as ShellEvent } from "./generated/Event";
import type { Request } from "./generated/Request";
import type { SearchResults } from "./generated/SearchResults";
import type { SettingsState } from "./generated/SettingsState";
import type { SurfaceInfo } from "./generated/SurfaceInfo";
import type { WindowInfo } from "./generated/WindowInfo";

/** Result type of each request method. Keep in sync with `shell/src/bridge.rs`. */
interface ResultMap {
  "shell.hello": SurfaceInfo;
  "search.query": SearchResults;
  "search.activate": null;
  "popover.toggle": null;
  "popover.close": null;
  "windows.list": WindowInfo[];
  "windows.activate": null;
  "settings.get": SettingsState;
  "settings.set_wifi": null;
  "settings.set_bluetooth": null;
  "settings.set_brightness": null;
  "settings.set_volume": null;
  "greeter.info": GreeterInfo;
  "greeter.login": null;
  "greeter.power": null;
}

type Reply = { ok: true; result: unknown } | { ok: false; error: ErrorBody };

interface MessageHandler {
  postMessage(message: string): Promise<unknown>;
}

declare global {
  interface Window {
    webkit?: { messageHandlers?: { meridian?: MessageHandler } };
    __meridianDispatch?: (event: string) => void;
  }
}

export class BridgeError extends Error {
  constructor(
    readonly code: ErrorBody["code"] | "unavailable",
    message: string,
  ) {
    super(message);
    this.name = "BridgeError";
  }
}

export async function request<R extends Request>(req: R): Promise<ResultMap[R["method"]]> {
  const handler = window.webkit?.messageHandlers?.meridian;
  if (!handler) {
    throw new BridgeError("unavailable", "not running inside meridian-shell");
  }
  const raw = await handler.postMessage(JSON.stringify(req));
  if (typeof raw !== "string") {
    throw new BridgeError("failed", `malformed reply to ${req.method}`);
  }
  const reply = JSON.parse(raw) as Reply;
  if (!reply.ok) {
    throw new BridgeError(reply.error.code, reply.error.message);
  }
  return reply.result as ResultMap[R["method"]];
}

type EventName = ShellEvent["event"];
type EventOf<N extends EventName> = Extract<ShellEvent, { event: N }>;
type Listener = (event: ShellEvent) => void;

const listeners = new Map<EventName, Set<Listener>>();

export function on<N extends EventName>(name: N, fn: (event: EventOf<N>) => void): () => void {
  let set = listeners.get(name);
  if (!set) {
    set = new Set();
    listeners.set(name, set);
  }
  const listener = fn as Listener;
  set.add(listener);
  return () => set.delete(listener);
}

window.__meridianDispatch = (json: string) => {
  let event: ShellEvent;
  try {
    event = JSON.parse(json) as ShellEvent;
  } catch (e) {
    console.error("meridian: malformed event", e);
    return;
  }
  for (const fn of listeners.get(event.event) ?? []) {
    try {
      fn(event);
    } catch (e) {
      console.error(`meridian: listener for ${event.event} failed`, e);
    }
  }
};

/** Handshake: marks the surface ready for events. Call after registering listeners. */
export function hello(): Promise<SurfaceInfo> {
  return request({ method: "shell.hello" });
}

/** Resolve after the next rendered frame (refresh-rate independent). */
export function nextFrame(): Promise<number> {
  return new Promise((resolve) => requestAnimationFrame(resolve));
}

/** Read a CSS `<time>` custom property (e.g. `--dur-med`) in milliseconds. */
export function cssDuration(name: string, el: Element = document.documentElement): number {
  const v = getComputedStyle(el).getPropertyValue(name).trim();
  const n = parseFloat(v);
  if (Number.isNaN(n)) return 0;
  return v.endsWith("ms") ? n : n * 1000;
}

/** Wait for `el`'s transition to end, bounded by the given duration. */
export function afterTransition(el: HTMLElement, maxMs: number): Promise<void> {
  return new Promise((resolve) => {
    if (maxMs <= 0) return resolve();
    const done = () => {
      el.removeEventListener("transitionend", onEnd);
      clearTimeout(timer);
      resolve();
    };
    const onEnd = (e: TransitionEvent) => {
      if (e.target === el) done();
    };
    el.addEventListener("transitionend", onEnd);
    // Fallback if no transition runs (e.g. property unchanged); +50 ms slack.
    const timer = setTimeout(done, maxMs + 50);
  });
}
