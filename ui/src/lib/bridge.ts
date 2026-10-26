import type { SearchPreferences } from "./generated/SearchPreferences";
import type { PrimaryDisplayState } from "./generated/PrimaryDisplayState";
import type { FilePreview } from "./generated/FilePreview";
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
  "notifications.do_not_disturb": import("./generated/NotificationState").NotificationState;
  "settings.primary_display": PrimaryDisplayState;
  "settings.primary_display_set": PrimaryDisplayState;
  "settings.search": SearchPreferences;
  "settings.search_set": SearchPreferences;
  "files.preview": FilePreview;
  "shell.hello": SurfaceInfo;
  "desktop.font_set": import("./generated/DesktopState").DesktopState;
  "desktop.appearance_set": import("./generated/DesktopState").DesktopState;
  "desktop.get": import("./generated/DesktopState").DesktopState;
  "desktop.wallpaper_set": import("./generated/DesktopState").DesktopState;
  "desktop.wallpaper_choose": import("./generated/DesktopState").DesktopState;
  "desktop.add_app": import("./generated/DesktopState").DesktopState;
  "desktop.add_file": import("./generated/DesktopState").DesktopState;
  "desktop.choose_files": import("./generated/DesktopState").DesktopState;
  "desktop.remove": import("./generated/DesktopState").DesktopState;
  "desktop.open": null;
  "files.bookmarks": import("./generated/FileLocation").FileLocation[];
  "files.bookmark_set": import("./generated/FileLocation").FileLocation[];
  "files.choose_folder": string | null;
  "files.locations": import("./generated/FileLocation").FileLocation[];
  "files.list": import("./generated/FileListing").FileListing;
  "files.properties": import("./generated/FileProperties").FileProperties;
  "files.trash_list": import("./generated/TrashListing").TrashListing;
  "files.trash_restore": string;
  "files.open": null;
  "files.mkdir": null;
  "files.rename": null;
  "files.trash": null;
  "files.transfer": null;
  "files.transfers": import("./generated/TransferProgress").TransferProgress[];
  "files.transfer_start": import("./generated/TransferProgress").TransferProgress;
  "files.transfer_status": import("./generated/TransferProgress").TransferProgress;
  "files.transfer_cancel": null;
  "files.thumbnail": string | null;
  "files.wallpaper": import("./generated/DesktopState").DesktopState;
  "menu.open": null;
  "preferences.open": null;
  "preferences.wallpaper_open": null;
  "preferences.page_open": null;
  "power.status": import("./generated/PowerState").PowerState;
  "settings.idle": import("./generated/IdleSettings").IdleSettings;
  "settings.idle_set": import("./generated/IdleSettings").IdleSettings;
  "finder.open": null;
  "notifications.get": import("./generated/NotificationState").NotificationState;
  "notifications.read": import("./generated/NotificationState").NotificationState;
  "notifications.clear": import("./generated/NotificationState").NotificationState;
  "notifications.dismiss": import("./generated/NotificationState").NotificationState;
  "notifications.action": import("./generated/NotificationState").NotificationState;
  "calendar.list": import("./generated/CalendarEvent").CalendarEvent[];
  "calendar.save": null;
  "pulse.open": null;
  "pulse.snapshot": import("./generated/PulseSnapshot").PulseSnapshot;
  "terminal.open": null;
  "terminal.open_at": null;
  "terminal.start": import("./generated/TerminalSession").TerminalSession;
  "terminal.start_at": import("./generated/TerminalSession").TerminalSession;
  "terminal.copy": null;
  "terminal.paste": string | null;
  "terminal.read": import("./generated/TerminalChunk").TerminalChunk;
  "terminal.write": null;
  "terminal.resize": null;
  "terminal.close": null;
  "trash.open": null;
  "apps.uninstall_prompt": null;
  "apps.uninstall_plan": import("./generated/UninstallPlan").UninstallPlan;
  "apps.uninstall": null;
  "system.info": import("./generated/SystemInfo").SystemInfo;
  "dock.layout_get": string[];
  "dock.layout_set": null;
  "dock.list": string[];
  "dock.pin": null;
  "search.query": SearchResults;
  "search.activate": null;
  "popover.toggle": null;
  "popover.close": null;
  "windows.list": WindowInfo[];
  "windows.activate": null;
  "windows.close": null;
  "settings.displays": import("./generated/DisplayOutput").DisplayOutput[];
  "settings.display_apply": import("./generated/DisplayTrial").DisplayTrial;
  "settings.display_decide": null;
  "settings.monitor_brightness": import("./generated/MonitorBrightness").MonitorBrightness;
  "settings.wifi_networks": import("./generated/WifiNetwork").WifiNetwork[];
  "settings.wifi_scan": null;
  "settings.wifi_connect": null;
  "settings.wifi_profiles": import("./generated/WifiProfile").WifiProfile[];
  "settings.wifi_config": import("./generated/WifiConfig").WifiConfig;
  "settings.wifi_config_set": null;
  "settings.wifi_profile_action": null;
  "settings.bluetooth_devices": import("./generated/BluetoothDevice").BluetoothDevice[];
  "settings.bluetooth_scan": null;
  "settings.bluetooth_action": null;
  "settings.bluetooth_audio": import("./generated/BluetoothAudioState").BluetoothAudioState;
  "settings.bluetooth_audio_set": import("./generated/BluetoothAudioState").BluetoothAudioState;
  "settings.system_info": import("./generated/SystemInfo").SystemInfo;
  "settings.startup": import("./generated/StartupEntry").StartupEntry[];
  "settings.startup_set": import("./generated/StartupEntry").StartupEntry[];
  "settings.startup_add": import("./generated/StartupEntry").StartupEntry[];
  "settings.startup_apps": import("./generated/SearchItem").SearchItem[];
  "settings.check_updates": import("./generated/UpdateInfo").UpdateInfo;
  "settings.open_releases": null;
  "settings.users": import("./generated/UserInfo").UserInfo[];
  "settings.user_create": null;
  "settings.user_action": null;
  "settings.devices": import("./generated/ManagedDevice").ManagedDevice[];
  "settings.device_action": null;
  "settings.drivers": import("./generated/DriverInfo").DriverInfo[];
  "settings.driver_set": null;
  "settings.get": SettingsState;
  "settings.audio": import("./generated/AudioState").AudioState;
  "settings.audio_action": import("./generated/AudioState").AudioState;
  "settings.set_wifi": null;
  "settings.set_bluetooth": null;
  "settings.set_brightness": null;
  "settings.set_volume": null;
  "greeter.info": GreeterInfo;
  "greeter.login": null;
  "greeter.power": null;
  "session.lock": null;
  "session.sign_out": null;
  "session.sleep": null;
  "session.restart": null;
  "session.shutdown": null;
  "locker.unlock": null;
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
  const message = JSON.stringify(req);
  if (new TextEncoder().encode(message).length > 2 * 1024 * 1024) {
    throw new BridgeError("invalid_request", "Request exceeds the 2 MiB limit.");
  }
  const raw = await handler.postMessage(message);
  if (typeof raw !== "string") {
    throw new BridgeError("failed", `malformed reply to ${req.method}`);
  }
  let reply: Reply;
  try { reply = JSON.parse(raw) as Reply; }
  catch { throw new BridgeError("failed", `malformed reply to ${req.method}`); }
  if (!reply || typeof reply !== "object" || typeof reply.ok !== "boolean" || (!reply.ok && (!reply.error || typeof reply.error.message !== "string"))) {
    throw new BridgeError("failed", `malformed reply to ${req.method}`);
  }
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
let handshake: Promise<SurfaceInfo> | undefined;
export function hello(): Promise<SurfaceInfo> {
  return handshake ??= request({ method: "shell.hello" }).catch(error => { handshake = undefined; throw error; });
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
