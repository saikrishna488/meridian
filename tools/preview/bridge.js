/* Development fixtures only. Never calls the native bridge or touches files. */
(() => {
  const previewPage = new URLSearchParams(location.search).get("page");
  const mode = new URLSearchParams(location.search).get("theme") || "light";
  const apps = [
    ["meridian-pulse.desktop", "Pulse"], ["meridian-finder.desktop", "Orbit"], ["meridian-terminal.desktop", "Meridian Terminal"], ["meridian-settings.desktop", "Settings"], ["firefox.desktop", "Firefox"], ["code.desktop", "Code"], ["calendar.desktop", "Calendar"], ["mail.desktop", "Mail"],
  ].map(([id, title]) => ({ id, title, subtitle: "Application", icon: null }));
  const desktop = { appearance: mode, dock_style: "floating", hover_background: "auto", font: "Inter", wallpaper: "meridian-dusk", wallpaper_uri: "/assets/wallpapers/meridian-dusk.svg", wallpapers: ["dusk", "tide", "alpine", "aurora"].map(id => ({ id: `meridian-${id}`, name: id[0].toUpperCase() + id.slice(1), uri: `/assets/wallpapers/meridian-${id}.svg` })), items: [] };
  const settings = { wifi: true, bluetooth: true, brightness: 75, volume: 42 };
  const locations = ["Home", "Desktop", "Documents", "Downloads", "Pictures", "Music", "Videos"].map(name => ({ name, path: name === "Home" ? "/home/alex" : `/home/alex/${name}` }));
  let pins = ["meridian-terminal.desktop", "meridian-finder.desktop", "firefox.desktop"], layout = [], terminalSent = false, nextTerminal = 1;
  let windows = [{ id: 1, name: "Firefox", title: "Meridian — Welcome", active: true }, { id: 2, name: "Firefox", title: "New tab", active: false }];
  let pulseTick = 0, bookmarks = [], calendarEvents = [];
  let notificationState = { available: true, items: [{ id: 1, app: "Calendar", summary: "Design review", body: "Starts in 10 minutes", time: Date.now(), unread: true, actions: [["open", "Open"]] }] };
  let trashEntries = [
    { uri: "trash:///Meeting%20notes.txt", name: "Meeting notes.txt", original: "/home/alex/Documents/Meeting notes.txt", deleted: "2026-10-17T09:15:00", size: 1240, directory: false },
    { uri: "trash:///Old%20project", name: "Old project", original: "/home/alex/Documents/Old project", deleted: "2026-10-16T14:30:00", size: 0, directory: true },
  ];
  let audio = { devices: [
    { id: 55, name: "built-in-speakers", description: "Built-in speakers", kind: "output", default: true },
    { id: 65, name: "headphones", description: "Studio headphones", kind: "output", default: false },
    { id: 56, name: "built-in-microphone", description: "Built-in microphone", kind: "input", default: true },
  ], output_volume: 42, input_volume: 75, output_muted: false, input_muted: false };
  let startupEntries = [
    { id: "mail.desktop", name: "Mail", description: "Keep up with your inbox", command: "mail", enabled: true, available: true, reason: "", user: true },
    { id: "calendar.desktop", name: "Calendar", description: "Your schedule, ready for the day", command: "calendar", enabled: false, available: true, reason: "", user: false },
    { id: "sync.desktop", name: "File Sync", description: "Keep your files up to date", command: "file-sync --background", enabled: true, available: true, reason: "", user: false },
  ];
  window.previewRequests = [];
  const emit = (event, data) => window.__meridianDispatch?.(JSON.stringify({ event, data }));
  window.previewEmit = emit;
  window.webkit = { messageHandlers: { meridian: { postMessage: async raw => {
    const { method, params = {} } = JSON.parse(raw);
    window.previewRequests.push({ method, params });
    let result = null;
    switch (method) {
      case "shell.hello": result = { surface: location.pathname.includes("utilities") ? (new URLSearchParams(location.search).get("app") || "calendar") : location.pathname.split("/").at(-2), capabilities: [] }; setTimeout(() => { emit("popover.shown"); if (location.pathname.includes("settings") && ["wifi", "bluetooth"].includes(previewPage)) emit("preferences.page", { page: previewPage }); if (location.pathname.includes("terminal")) emit("terminal.opened"); if (location.pathname.includes("desktop-menu")) emit("menu.opened", { menu: "Meridian", x: 12 }); }, 100); break;
      case "calendar.list": result = calendarEvents; break;
      case "calendar.save": calendarEvents = params; break;
      case "desktop.get": result = desktop; break;
      case "desktop.hover_background_set": desktop.hover_background = params.value; result = desktop; emit("desktop.changed", desktop); break;
      case "desktop.dock_style_set": desktop.dock_style = params.id; result = desktop; emit("desktop.changed", desktop); break;
      case "desktop.font_set": desktop.font = params.id; emit("desktop.changed", desktop); result = desktop; break;
      case "desktop.appearance_set": desktop.appearance = params.id; emit("desktop.changed", desktop); result = desktop; break;
      case "desktop.wallpaper_set": desktop.wallpaper = params.id; desktop.wallpaper_uri = `/assets/wallpapers/${params.id}.svg`; emit("desktop.changed", desktop); result = desktop; break;
      case "search.query": result = { serial: params.serial, sections: [{ provider: "apps", items: apps.filter(app => app.title.toLowerCase().includes((params.query || "").toLowerCase())) }] }; break;
      case "windows.list": result = windows; break;
      case "windows.activate": windows = windows.map(window => ({ ...window, active: window.id === params.id })); emit("windows.changed", windows); break;
      case "dock.list": result = pins; break;
      case "dock.layout_get": result = layout; break;
      case "dock.layout_set": layout = params.items; emit("dock.layout_changed", layout); break;
      case "dock.pin": pins = [...pins.filter(id => id !== params.id), ...(params.pinned ? [params.id] : [])]; emit("dock.changed", pins); break;
      case "notifications.do_not_disturb": notificationState.do_not_disturb = params.enabled; result = notificationState; emit("notifications.changed", notificationState); break;
      case "notifications.get": result = notificationState; break;
      case "notifications.read": notificationState.items.forEach(i => { i.unread = false; }); result = notificationState; emit("notifications.changed", notificationState); break;
      case "notifications.dismiss": case "notifications.action": notificationState.items = notificationState.items.filter(i => i.id !== params.id); result = notificationState; emit("notifications.changed", notificationState); break;
      case "notifications.clear": notificationState.items = []; result = notificationState; emit("notifications.changed", notificationState); break;
      case "settings.capability": result = { state: "ready", message: "Preview fixture · service available", adapters: params.feature === "bluetooth" ? [{ path: "/org/bluez/hci0", name: "Bluetooth adapter", enabled: true }] : [] }; break;
      case "settings.storage": result = { total: 512e9, available: 300e9, filesystem: "btrfs", source: "/dev/nvme0n1p2", warning: null, volumes: [
        { path: "/dev/nvme0n1p2", name: "System", kind: "part", size: 512e9, filesystem: "btrfs", mount_points: ["/"], read_only: false, removable: false, parent: "/dev/nvme0n1", actions: ["open"] },
        { path: "/dev/sdb1", name: "USB Drive", kind: "part", size: 32e9, filesystem: "exfat", mount_points: ["/run/media/alex/USB"], read_only: false, removable: true, parent: "/dev/sdb", actions: ["open", "unmount"] }
      ] }; break;
      case "settings.storage_action": throw new Error("Storage operations are unavailable in this read-only preview.");
      case "settings.devices": result = [
        { id: "net:wlan0", category: "Wi-Fi adapters", name: "Wireless adapter", status: "Connected", properties: [["Driver", "iwlwifi"], ["Interface", "wlan0"]], actions: ["disconnect"] },
        { id: "bluetooth:hci0", category: "Bluetooth", name: "hci0", status: "On", properties: [["Driver", "btusb"]], actions: ["off"] },
        { id: "printer:Office", category: "Printers", name: "Office", status: "Ready", properties: [["Queue", "Office"]], actions: ["pause"] },
        { id: "storage:nvme0n1", category: "Storage", name: "NVMe SSD", status: "Connected", properties: [["Driver", "nvme"], ["Capacity", "1024 GB"]], actions: ["storage"] },
        { id: "storage:sdb", category: "Storage", name: "USB Flash Drive", status: "Connected", properties: [["Capacity", "32 GB"]], actions: ["storage", "eject"] },
        { id: "storage:sdb1", category: "Storage", name: "sdb1", status: "Connected", properties: [["Mount points", "/media/alex/USB"]], actions: ["unmount"] },
      ]; break;
      case "settings.device_action": throw new Error("Device changes are unavailable in this read-only preview.");
      case "files.bookmarks": result = bookmarks; break;
      case "files.bookmark_set": bookmarks = bookmarks.some(l => l.path === params.path) ? bookmarks.filter(l => l.path !== params.path) : [...bookmarks, { name: params.path.split("/").filter(Boolean).at(-1), path: params.path }]; result = bookmarks; break;
      case "files.choose_folder": result = "/home/alex/Documents"; break;
      case "files.thumbnail": result = null; break;
      case "files.transfer": case "files.wallpaper": throw new Error("File changes are unavailable in this read-only preview.");
      case "settings.drivers": result = [{ name: "sample_network", path: "kernel/drivers/net/sample_network.ko", loaded: true, builtin: false, users: 1, devices: ["PCI 0000:01:00.0"] }, { name: "sample_audio", path: "kernel/sound/sample_audio.ko", loaded: false, builtin: false, users: 0, devices: [] }]; break;
      case "settings.users": result = [{ uid: 1000, name: "meridian", real_name: "Meridian User", administrator: true, locked: false, current: true, home: "/home/meridian" }]; break;
      case "settings.driver_set": case "settings.user_create": case "settings.user_action": throw new Error("System changes are unavailable in this read-only preview.");
      case "settings.startup": result = startupEntries; break;
      case "settings.startup_apps": result = apps; break;
      case "settings.startup_set": startupEntries = startupEntries.map(e => e.id === params.id ? { ...e, enabled: params.enabled, user: true } : e); result = startupEntries; break;
      case "settings.startup_add": { const app = apps.find(a => a.id === params.id); if (app) startupEntries.push({ id: app.id, name: app.title, description: app.subtitle, command: app.id, enabled: true, available: true, reason: "", user: true }); result = startupEntries; break; }
      case "files.trash_list": result = { entries: trashEntries, truncated: false }; break;
      case "files.trash_restore": result = trashEntries.find(e => e.uri === params.uri)?.original; trashEntries = trashEntries.filter(e => e.uri !== params.uri); break;
      case "files.properties": result = { name: params.path.split("/").at(-1), path: params.path, kind: params.path.endsWith(".pdf") ? "PDF document" : "Folder", size: params.path.endsWith(".pdf") ? 2457600 : null, modified: Date.now() / 1000 - 7200, created: Date.now() / 1000 - 864000, owner: "alex", group: "alex", permissions: "rw-r--r-- (644)", link_target: null }; break;
      case "power.status": result = { on_ac: false, batteries: [{ name: "BAT0", percent: 72, status: "Discharging", health: 96 }] }; break;
      case "settings.idle": result = window.previewIdle || { lock_after: 300, blank_after: 330 }; break;
      case "settings.idle_set": window.previewIdle = params; result = params; break;
      case "settings.audio": result = audio; break;
      case "settings.audio_action": { const device = audio.devices.find(d => d.id === params.id); if (device) { if (params.action === "default") audio.devices.forEach(d => { if (d.kind === device.kind) d.default = d.id === device.id; }); if (params.action === "volume") audio[device.kind + "_volume"] = params.percent; if (["mute", "unmute"].includes(params.action)) audio[device.kind + "_muted"] = params.action === "mute"; } result = audio; break; }
      case "settings.get": result = settings; break;
      case "settings.set_wifi": settings.wifi = params.enabled; emit("settings.changed", settings); break;
      case "settings.set_bluetooth": settings.bluetooth = params.enabled; emit("settings.changed", settings); break;
      case "settings.set_volume": settings.volume = params.percent; emit("settings.changed", settings); break;
      case "settings.set_brightness": settings.brightness = params.percent; emit("settings.changed", settings); break;
      case "settings.wifi_networks": result = [{ frequency: 5180, channel: 36, rate: "540 Mbit/s", ssid: "Home network", bssid: "00:00:00:00:00:01", device: "wlan0", signal: 91, security: "WPA2", connected: true }, { frequency: 2437, channel: 6, rate: "130 Mbit/s", ssid: "Guest network", bssid: "00:00:00:00:00:02", device: "wlan0", signal: 68, security: "WPA2", connected: false }]; break;
      case "settings.wifi_profiles": result = [{ uuid: "12345678-1234-1234-1234-123456789abc", name: "Home network", device: "wlan0" }, { uuid: "22345678-1234-1234-1234-123456789abc", name: "Studio", device: "" }]; break;
      case "settings.wifi_config": result = { uuid: params.uuid, autoconnect: true, metered: "unknown", method: "auto", addresses: "", gateway: "", dns: "", ignore_auto_dns: false }; break;
      case "settings.wifi_config_set": case "settings.wifi_profile_action": case "settings.wifi_connect": case "settings.bluetooth_action": throw new Error("Connection changes are unavailable in this read-only preview.");
      case "settings.wifi_scan": case "settings.bluetooth_scan": break;
      case "settings.bluetooth_devices": result = [
        { path: "/org/bluez/hci0/keyboard", name: "Wireless keyboard", address: "AA:BB:CC:DD:EE:01", icon: "input-keyboard", paired: true, connected: true, trusted: true, battery: 86 },
        { path: "/org/bluez/hci0/headphones", name: "Studio headphones", address: "AA:BB:CC:DD:EE:02", icon: "audio-headphones", paired: true, connected: false, trusted: true, battery: 74 },
        { path: "/org/bluez/hci0/speaker", name: "Living room speaker", address: "AA:BB:CC:DD:EE:03", icon: "audio-card", paired: false, connected: false, trusted: false, battery: null },
      ]; break;
      case "settings.displays": result = [{ name: "eDP-1", description: "Built-in display", enabled: true, x: 0, y: 0, scale: 1, transform: 0, current: { width: 1920, height: 1080, refresh: 60000 }, modes: [{ width: 1920, height: 1080, refresh: 60000, preferred: true }] }]; break;
      case "pulse.snapshot": {
        const wave = ++pulseTick;
        const make = (id, kind, name, detail, percent, extras = {}) => ({ id, kind, name, detail, percent, used: null, total: null, read_rate: null, write_rate: null, ...extras });
        result = { resources: [
          make("cpu", "cpu", "CPU", "Preview processor · 12 logical processors", 22 + 12 * Math.sin(wave / 3)),
          make("memory", "memory", "Memory", "Physical RAM · used / total", 43.7, { used: 7507602636, total: 17179869184 }),
          make("gpu:card0", "gpu", "AMD GPU · card0", "Preview graphics", 14 + 8 * Math.sin(wave / 4), { used: 1610612736, total: 8589934592 }),
          make("disk:nvme0n1", "storage", "nvme0n1", "Preview SSD · read / write", null, { total: 1024000000000, read_rate: 16000000 + 8000000 * Math.sin(wave / 2), write_rate: 6000000 + 4000000 * Math.cos(wave / 3) }),
          make("net:wlan0", "network", "wlan0", "up · download / upload", null, { read_rate: 2500000 + 1500000 * Math.sin(wave / 3), write_rate: 320000 + 200000 * Math.sin(wave / 2) }),
        ], processes: [{ pid: 1024, name: "meridian-shell", state: "S", cpu: 3.2, memory: 214958080 }, { pid: 1729, name: "firefox", state: "R", cpu: 8.4, memory: 681574400 }, { pid: 985, name: "labwc", state: "S", cpu: 0.8, memory: 48234496 }, { pid: 2012, name: "bash", state: "S", cpu: 0, memory: 5242880 }] };
        break;
      }
      case "files.locations": result = locations; break;
      case "files.list": result = { path: params.path, parent: params.path === "/" ? null : params.path.slice(0, params.path.lastIndexOf("/")) || "/", truncated: false, entries: (params.path === "/home/alex" ? ["Desktop", "Documents", "Downloads", "Pictures", "Music", "Videos"] : params.path.endsWith("Documents") ? ["Projects", "Invoices", "Getting started.pdf", "Meeting notes.txt", "Desktop ideas.md"] : []).map((name, index) => ({ name, path: `${params.path}/${name}`, directory: !name.includes("."), symlink: false, size: name.endsWith(".pdf") ? 2457600 : name.includes(".") ? 1240 : 0, modified: 1789516800 + index * 86400 })) }; break;
      case "terminal.start": case "terminal.start_at": result = { id: nextTerminal++, shell: "/bin/bash" }; terminalSent = false; break;
      case "terminal.read": result = { data: terminalSent ? [] : [...new TextEncoder().encode("\x1b[1;32malex@meridian\x1b[0m:~$ \r\n")], exited: false }; terminalSent = true; break;
      case "terminal.paste": result = ""; break;
      case "greeter.info": result = { hostname: "meridian", last_user: "alex", last_session: "meridian", sessions: [{ id: "meridian", name: "Meridian", command: "meridian-session" }] }; break;
      case "settings.check_updates": result = { installed: "0.1.1", latest: "v0.1.1", available: false, message: "Preview · you have the latest published version.", published: null }; break;
      case "settings.open_releases": result = null; break;
      case "settings.system_info":
      case "system.info": result = { version: "0.1.1", kernel: "6.12.0", architecture: "x86_64", hostname: "meridian-preview", os: "Debian", manufacturer: "Framework", model: "Laptop 13", processor: "Intel Core", memory: "16 GiB" }; break;
      case "search.activate": case "pulse.open": case "finder.open": case "terminal.open": case "preferences.open": result = null; break;
    }
    return JSON.stringify({ ok: true, result });
  } } } };
})();
