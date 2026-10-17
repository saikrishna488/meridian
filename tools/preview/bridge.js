/* Development fixtures only. Never calls the native bridge or touches files. */
(() => {
  const mode = new URLSearchParams(location.search).get("theme") || "light";
  const apps = [
    ["meridian-finder.desktop", "Finder"], ["meridian-terminal.desktop", "Meridian Terminal"], ["meridian-settings.desktop", "Settings"], ["firefox.desktop", "Firefox"], ["code.desktop", "Code"], ["calendar.desktop", "Calendar"], ["mail.desktop", "Mail"],
  ].map(([id, title]) => ({ id, title, subtitle: "Application", icon: null }));
  const desktop = { appearance: mode, wallpaper: "meridian-dusk", wallpaper_uri: "/assets/wallpapers/meridian-dusk.svg", wallpapers: ["dusk", "tide", "alpine", "aurora"].map(id => ({ id: `meridian-${id}`, name: id[0].toUpperCase() + id.slice(1), uri: `/assets/wallpapers/meridian-${id}.svg` })), items: [] };
  const settings = { wifi: true, bluetooth: true, brightness: 75, volume: 42 };
  const locations = ["Home", "Desktop", "Documents", "Downloads", "Pictures", "Music", "Videos"].map(name => ({ name, path: name === "Home" ? "/home/alex" : `/home/alex/${name}` }));
  let pins = ["meridian-terminal.desktop", "meridian-finder.desktop"], layout = [], terminalSent = false, nextTerminal = 1;
  window.previewRequests = [];
  const emit = (event, data) => window.__meridianDispatch?.(JSON.stringify({ event, data }));
  window.previewEmit = emit;
  window.webkit = { messageHandlers: { meridian: { postMessage: async raw => {
    const { method, params = {} } = JSON.parse(raw);
    window.previewRequests.push({ method, params });
    let result = null;
    switch (method) {
      case "shell.hello": result = { surface: location.pathname.split("/").at(-2), capabilities: [] }; setTimeout(() => { emit("popover.shown"); if (location.pathname.includes("terminal")) emit("terminal.opened"); if (location.pathname.includes("desktop-menu")) emit("menu.opened", { menu: "Meridian", x: 12 }); }, 100); break;
      case "desktop.get": result = desktop; break;
      case "desktop.appearance_set": desktop.appearance = params.id; emit("desktop.changed", desktop); result = desktop; break;
      case "desktop.wallpaper_set": desktop.wallpaper = params.id; desktop.wallpaper_uri = `/assets/wallpapers/${params.id}.svg`; emit("desktop.changed", desktop); result = desktop; break;
      case "search.query": result = { serial: params.serial, sections: [{ provider: "apps", items: apps.filter(app => app.title.toLowerCase().includes((params.query || "").toLowerCase())) }] }; break;
      case "windows.list": result = []; break;
      case "dock.list": result = pins; break;
      case "dock.layout_get": result = layout; break;
      case "dock.layout_set": layout = params.items; emit("dock.layout_changed", layout); break;
      case "dock.pin": pins = [...pins.filter(id => id !== params.id), ...(params.pinned ? [params.id] : [])]; emit("dock.changed", pins); break;
      case "settings.get": result = settings; break;
      case "settings.set_wifi": settings.wifi = params.enabled; emit("settings.changed", settings); break;
      case "settings.set_bluetooth": settings.bluetooth = params.enabled; emit("settings.changed", settings); break;
      case "settings.set_volume": settings.volume = params.percent; emit("settings.changed", settings); break;
      case "settings.set_brightness": settings.brightness = params.percent; emit("settings.changed", settings); break;
      case "settings.wifi_networks": result = [{ ssid: "Home network", bssid: "00:00:00:00:00:01", device: "wlan0", signal: 91, security: "WPA2", connected: true }, { ssid: "Guest network", bssid: "00:00:00:00:00:02", device: "wlan0", signal: 68, security: "WPA2", connected: false }]; break;
      case "settings.bluetooth_devices": result = [{ path: "/keyboard", name: "Wireless keyboard", paired: true, connected: true }]; break;
      case "settings.displays": result = [{ name: "eDP-1", description: "Built-in display", enabled: true, x: 0, y: 0, scale: 1, transform: 0, current: { width: 1920, height: 1080, refresh: 60000 }, modes: [{ width: 1920, height: 1080, refresh: 60000, preferred: true }] }]; break;
      case "files.locations": result = locations; break;
      case "files.list": result = { path: params.path, parent: params.path === "/" ? null : params.path.slice(0, params.path.lastIndexOf("/")) || "/", truncated: false, entries: (params.path === "/home/alex" ? ["Desktop", "Documents", "Downloads", "Pictures", "Music", "Videos"] : []).map(name => ({ name, path: `${params.path}/${name}`, directory: true, symlink: false, size: 0, modified: 1789516800 })) }; break;
      case "terminal.start": case "terminal.start_at": result = { id: nextTerminal++, shell: "/bin/bash" }; terminalSent = false; break;
      case "terminal.read": result = { data: terminalSent ? [] : [...new TextEncoder().encode("\x1b[1;32malex@meridian\x1b[0m:~$ \r\n")], exited: false }; terminalSent = true; break;
      case "terminal.paste": result = ""; break;
      case "greeter.info": result = { hostname: "meridian", last_user: "alex", last_session: "meridian", sessions: [{ id: "meridian", name: "Meridian", command: "meridian-session" }] }; break;
      case "system.info": result = { version: "0.1.0", os: "Debian", manufacturer: "Framework", model: "Laptop 13", processor: "Intel Core", memory: "16 GiB" }; break;
      case "search.activate": case "finder.open": case "terminal.open": case "preferences.open": result = null; break;
    }
    return JSON.stringify({ ok: true, result });
  } } } };
})();
