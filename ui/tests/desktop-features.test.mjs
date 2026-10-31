import { installBrowserMocks } from "./browser-mocks.mjs";
import { surfaceScript } from "./bundle.mjs";
import assert from "node:assert/strict";
import { test } from "node:test";
import { readFile } from "node:fs/promises";
import { JSDOM } from "jsdom";
const app = { id: "firefox.desktop", title: "Firefox", icon: "firefox" };
const search = { serial: 0, sections: [{ provider: "apps", items: [app] }] };
async function waitFor(fn) { for (let i = 0; i < 100; i++) { if (fn()) return; await new Promise(r => setTimeout(r, 10)); } assert.fail("UI did not update"); }
async function surface(t, name, respond) {
  const html = await readFile(new URL(`../dist/surfaces/${name}/index.html`, import.meta.url), "utf8");
  const dom = new JSDOM(html, { url: `https://meridian.test/surfaces/${name}/`, runScripts: "outside-only", pretendToBeVisual: true });
  t.after(() => dom.window.close());
  const { window } = dom, requests = [];
  window.TextEncoder = TextEncoder;
  window.HTMLElement.prototype.scrollIntoView = () => {};
  window.HTMLElement.prototype.scrollTo = () => {};
  installBrowserMocks(window);
  window.HTMLCanvasElement.prototype.getContext = () => new Proxy({ measureText: text => ({ width: text.length * 8, actualBoundingBoxAscent: 10, actualBoundingBoxDescent: 3 }), createLinearGradient: () => ({ addColorStop() {} }), getImageData: (_x, _y, width, height) => ({ data: new Uint8ClampedArray(width * height * 4), width, height }) }, { get(target, key) { return key in target ? target[key] : () => {}; }, set(target, key, value) { target[key] = value; return true; } });
  window.fetch = async () => { throw new Error("Custom-scheme fetch is unavailable"); };
  window.webkit = { messageHandlers: { meridian: { async postMessage(json) { const req = JSON.parse(json); requests.push(req); const result = await respond(req); return JSON.stringify(result?.error && typeof result.error === "object" ? { ok: false, error: result.error } : { ok: true, result }); } } } };
  window.eval(await surfaceScript(name));
  await waitFor(() => window.document.querySelector("#root").children.length);
  // Host events must arrive after the surface has registered its listeners.
  if (name !== "wallpaper") await waitFor(() => requests.some(req => req.method === "shell.hello"));
  return { window, requests, get: s => window.document.querySelector(s), emit: (event, data) => window.__meridianDispatch(JSON.stringify({ event, data })) };
}
function defaults(req) { if (req.method === "settings.bluetooth_audio") return { mode: "auto", active_codec: null, active_device: null }; if (["files.transfer_start", "files.transfer_status"].includes(req.method)) return { id: 1, bytes: 4, total: 4, entries: 1, done: true, error: null }; return req.method === "desktop.get" ? desktop : req.method === "search.query" ? { ...search, serial: req.params.serial } : req.method === "windows.list" ? [] : req.method === "dock.list" ? [app.id] : req.method === "dock.layout_get" ? [] : req.method === "shell.hello" ? { surface: "panel", capabilities: [] } : null; }
function drag(window, element, type, data, clientX = 1) {
  const event = new window.Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "dataTransfer", { value: { types: Object.keys(data), getData: key => data[key] ?? "", setData: (key, value) => { data[key] = value; }, effectAllowed: "", dropEffect: "" } });
  Object.defineProperty(event, "clientX", { value: clientX }); element.dispatchEvent(event);
}
test("dock reorders apps and forwards context actions to the native menu", async t => {
  const ui = await surface(t, "panel", defaults);
  await waitFor(() => ui.get('[aria-label="Firefox"]'));
  const data = {};
  drag(ui.window, ui.get("#applications"), "dragstart", data);
  drag(ui.window, ui.get('[aria-label="Firefox"]'), "drop", data);
  await waitFor(() => ui.requests.some(r => r.method === "dock.layout_set"));
  assert.deepEqual(ui.requests.find(r => r.method === "dock.layout_set").params.items, ["firefox.desktop", "@applications"]);
  ui.get('[aria-label="Firefox"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.requests.some(r => r.method === "menu.open"));
  const menu = ui.requests.find(r => r.method === "menu.open").params;
  assert.equal(menu.menu, "DockItem"); assert.equal(menu.item_id, app.id); assert.equal(menu.pinned, true);
  assert.ok(!ui.requests.some(r => r.method === "apps.uninstall"));
});
test("trash confirmation executes the reviewed target only after Uninstall", async t => {
  const ui = await surface(t, "applications", req => req.method === "apps.uninstall_plan" ? { id: app.id, name: "Firefox", target: "flatpak:--user:app/org.mozilla.firefox/x86_64/stable", detail: "Saved data is kept." } : defaults(req));
  await waitFor(() => ui.requests.some(r => r.method === "shell.hello"));
  ui.emit("apps.uninstall_requested", { id: app.id });
  await waitFor(() => ui.get(".uninstall-actions .danger")?.disabled === false);
  assert.ok(!ui.requests.some(r => r.method === "apps.uninstall"));
  ui.get(".uninstall-actions .danger").click();
  await waitFor(() => ui.requests.some(r => r.method === "apps.uninstall"));
  assert.equal(ui.requests.find(r => r.method === "apps.uninstall").params.target, "flatpak:--user:app/org.mozilla.firefox/x86_64/stable");
});

test("Meridian menu hosts session actions and power-off/restart require confirmation", async t => {
  for (const [id, method, confirm] of [["sleep", "session.sleep", false], ["lock", "session.lock", false], ["restart", "session.restart", true], ["shutdown", "session.shutdown", true], ["sign-out", "session.sign_out", true]]) {
    const ui = await surface(t, "desktop-menu", defaults);
    await waitFor(() => ui.requests.some(r => r.method === "shell.hello"));
    ui.emit("popover.shown"); ui.emit("menu.opened", { menu: "Meridian", x: 8 });
    await waitFor(() => !ui.get(".desktop-menu").classList.contains("hidden"));
    assert.equal(ui.get(`#${id}`).getAttribute("role"), "menuitem");
    ui.get(`#${id}`).click();
    if (confirm) {
      await waitFor(() => ui.get(`#${id}`).textContent.endsWith("?"));
      assert.ok(!ui.requests.some(r => r.method === method));
      ui.get(`#${id}`).click();
    }
    await waitFor(() => ui.requests.some(r => r.method === method));
  }
});

const desktop = { appearance: "light", wallpaper: "meridian-dusk", wallpaper_uri: "meridian://assets/wallpapers/meridian-dusk.svg", wallpapers: [{ id: "meridian-dusk", name: "Dusk", uri: "meridian://assets/wallpapers/meridian-dusk.svg" }, { id: "meridian-tide", name: "Tide", uri: "meridian://assets/wallpapers/meridian-tide.svg" }], items: [] };
function type(ui, selector, text) { const input = ui.get(selector); Object.getOwnPropertyDescriptor(ui.window.HTMLInputElement.prototype, "value").set.call(input, text); input.dispatchEvent(new ui.window.Event("input", { bubbles: true })); }
function menu(ui, selector, label) { [...ui.window.document.querySelectorAll(`${selector} button`)].find(b => b.textContent === label).click(); }
const folder = { name: "Projects", path: "/home/test/Projects", directory: true, symlink: false, size: 0, modified: 1 };
const file = { name: "notes.txt", path: "/home/test/notes.txt", directory: false, symlink: false, size: 12, modified: 1 };
function finderReply(req) {
  if (req.method === "files.bookmarks") return [];
  if (req.method === "files.bookmark_set") return [{ name: "Home", path: req.params.path }];
  if (req.method === "files.locations") return [{ name: "Home", path: "/home/test" }];
  if (req.method === "files.list") return { path: req.params.path, parent: "/home/test", entries: req.params.path.endsWith("Projects") ? [] : [folder, file], truncated: false };
  if (req.method === "apps.uninstall_plan") return { id: app.id, name: "Firefox", target: "flatpak:reviewed", detail: "Saved data stays." };
  return defaults(req);
}
test("Finder browses host folders, confirms trash, and manages installed apps", async t => {
  const ui = await surface(t, "finder", finderReply);
  await waitFor(() => ui.get('[aria-label="Projects"]'));
  ui.get('[aria-label="Projects"]').dispatchEvent(new ui.window.MouseEvent("dblclick", { bubbles: true }));
  await waitFor(() => ui.get("h1").textContent === "Projects");
  await waitFor(() => ui.get(".folder-empty")?.textContent.includes("empty"));
  ui.get('[aria-label="Go back"]').click(); await waitFor(() => ui.get('[aria-label="notes.txt"]'));
  ui.get('[aria-label="List view"]').click(); await waitFor(() => ui.get(".file-collection.list"));
  type(ui, '[aria-label="Search this folder"]', "notes"); await waitFor(() => ui.window.document.querySelectorAll(".file-item").length === 1);
  ui.get('[aria-label="notes.txt"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true })); await waitFor(() => ui.get('.finder-context'));
  menu(ui, '.finder-context', "Delete…"); await waitFor(() => ui.get('[role="dialog"]'));
  assert.ok(!ui.requests.some(r => r.method === "files.trash"));
  ui.get('[role="dialog"] button[type="submit"]').click(); await waitFor(() => ui.requests.some(r => r.method === "files.trash"));
  assert.equal(ui.requests.find(r => r.method === "files.trash").params.path, file.path);
  await waitFor(() => !ui.get('[role="dialog"]') && ui.get(".file-collection").getAttribute("aria-busy") === "false");
  [...ui.window.document.querySelectorAll('nav button')].find(b => b.textContent.endsWith("Applications")).click(); await waitFor(() => ui.get('[aria-label="Firefox"]'));
  ui.get('[aria-label="Firefox"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true })); await waitFor(() => ui.get('.finder-context'));
  menu(ui, '.finder-context', "Uninstall…"); await waitFor(() => ui.get('.uninstall-actions .danger')?.disabled === false);
  assert.ok(!ui.requests.some(r => r.method === "apps.uninstall"));
  ui.get('.uninstall-actions .danger').click(); await waitFor(() => ui.requests.some(r => r.method === "apps.uninstall"));
  assert.equal(ui.requests.find(r => r.method === "apps.uninstall").params.target, "flatpak:reviewed");
});
test("Finder creates folders with the reviewed destination and name", async t => {
  const ui = await surface(t, "finder", finderReply); await waitFor(() => ui.get('[aria-label="New folder"]').disabled === false);
  ui.get('[aria-label="New folder"]').click(); await waitFor(() => ui.get('[role="dialog"] input'));
  type(ui, '[role="dialog"] input', "My folder"); await waitFor(() => ui.get('[role="dialog"] input').value === "My folder");
  ui.get('[role="dialog"] button[type="submit"]').click(); await waitFor(() => ui.requests.some(r => r.method === "files.mkdir"));
  assert.deepEqual(ui.requests.find(r => r.method === "files.mkdir").params, { path: "/home/test", name: "My folder" });
});
test("Terminal renders real output, forwards paste to its PTY, and closes tabs", async t => {
  let next = 0, read = false;
  const ui = await surface(t, "terminal", req => {
    if (req.method === "terminal.start") return { id: ++next, shell: "/bin/bash" };
    if (req.method === "terminal.read") { const text = read ? "" : "REAL_SHELL_OK\r\n"; read = true; return { data: [...new TextEncoder().encode(text)], exited: false }; }
    return defaults(req);
  });
  await waitFor(() => ui.requests.some(r => r.method === "shell.hello"));
  ui.emit("terminal.opened");
  await waitFor(() => ui.get('.xterm-rows')?.textContent.includes("REAL_SHELL_OK"));
  const paste = new ui.window.Event("paste", { bubbles: true, cancelable: true }); Object.defineProperty(paste, "clipboardData", { value: { getData: () => "pwd\n" } });
  ui.get('.xterm-helper-textarea').dispatchEvent(paste); await waitFor(() => ui.requests.some(r => r.method === "terminal.write"));
  assert.deepEqual(ui.requests.filter(r => r.method === "terminal.write").map(r => r.params.data), ["pwd\r"]);
  assert.equal(ui.get(".terminal-status"), null);
  assert.ok(!ui.get(".terminal-session").textContent.includes("/bin/bash"));
  ui.get('[aria-label="New terminal session"]').click(); await waitFor(() => ui.window.document.querySelectorAll('[role="tab"]').length === 2); await waitFor(() => next === 2);
  ui.get('[aria-label="Close session 2"]').click(); await waitFor(() => ui.requests.some(r => r.method === "terminal.close" && r.params.id === 2));
});
test("Meridian built-in icons and installed app artwork are preserved", async t => {
  const built = { id: "meridian-finder.desktop", title: "Finder", icon: "system-file-manager" };
  const ui = await surface(t, "panel", req => req.method === "search.query" ? { serial: req.params.serial, sections: [{ provider: "apps", items: [app, built] }] } : req.method === "dock.list" ? [app.id, built.id] : defaults(req));
  await waitFor(() => ui.get('[aria-label="Finder"] img'));
  assert.equal(ui.get('[aria-label="Firefox"] img').src.split("?")[0], "meridian://app-icons/firefox.desktop"); assert.ok(ui.get('[aria-label="Firefox"] .system-artwork'));
  const fixed = ui.get('[aria-label="Finder"] img').src;
  assert.equal(ui.get('[aria-label="Finder"] img').src, fixed);
});
test("Settings selects wallpapers and opens the native custom image picker", async t => {
  const ui = await surface(t, "settings", req => req.method === "desktop.wallpaper_set" ? { ...desktop, wallpaper: req.params.id, wallpaper_uri: `meridian://assets/wallpapers/${req.params.id}.svg` } : req.method === "desktop.wallpaper_choose" ? { ...desktop, wallpaper: "custom", wallpaper_uri: "meridian://desktop/wallpaper?v=1" } : defaults(req));
  [...ui.window.document.querySelectorAll('nav button')].find(b => b.textContent.endsWith("Wallpaper")).click(); await waitFor(() => ui.get('.wallpaper-choice'));
  [...ui.window.document.querySelectorAll('.wallpaper-choice')].find(b => b.textContent === "Tide").click(); await waitFor(() => ui.requests.some(r => r.method === "desktop.wallpaper_set"));
  assert.equal(ui.requests.find(r => r.method === "desktop.wallpaper_set").params.id, "meridian-tide");
  await waitFor(() => ui.get('.wallpaper-custom button').disabled === false); ui.get('.wallpaper-custom button').click(); await waitFor(() => ui.get('.wallpaper-current').src === "meridian://desktop/wallpaper?v=1");
});
test("desktop right-click removes shortcuts without deleting their target files", async t => {
  const shortcut = { id: "file:/home/test/notes.txt", name: "notes.txt", kind: "file", target: "/home/test/notes.txt", icon: null };
  const ui = await surface(t, "wallpaper", req => req.method === "desktop.choose_files" ? { ...desktop, items: [shortcut] } : req.method === "desktop.remove" ? desktop : defaults(req));
  ui.get('.desktop').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 80, clientY: 80 })); await waitFor(() => ui.get('.desktop-context'));
  menu(ui, '.desktop-context', "Move files here…"); await waitFor(() => ui.get('[aria-label="notes.txt"]'));
  ui.get('[aria-label="notes.txt"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true })); await waitFor(() => ui.get('.desktop-context'));
  menu(ui, '.desktop-context', "Move to Trash"); await waitFor(() => ui.requests.some(r => r.method === "desktop.remove"));
  assert.ok(!ui.requests.some(r => ["files.trash", "files.rename"].includes(r.method)));
  await waitFor(() => !ui.get('[aria-label="notes.txt"]'));
  ui.emit("desktop.changed", { ...desktop, wallpaper_uri: "meridian://assets/wallpapers/meridian-tide.svg" }); await waitFor(() => ui.get('.wallpaper').style.backgroundImage.includes("meridian-tide"));
});

test("Terminal refits after fullscreen resize and keeps input focused", async t => {
  const ui = await surface(t, "terminal", req => req.method === "terminal.start" ? { id: 91, shell: "/bin/bash" } : req.method === "terminal.read" ? { data: [], exited: false } : defaults(req));
  await waitFor(() => ui.requests.some(r => r.method === "shell.hello"));
  ui.emit("terminal.opened");
  await waitFor(() => ui.get(".terminal-screen .xterm"));
  const screen = ui.get(".terminal-screen");
  Object.defineProperty(screen, "clientWidth", { configurable: true, value: 1200 });
  Object.defineProperty(screen, "clientHeight", { configurable: true, value: 700 });
  screen.style.width = "1200px"; screen.style.height = "700px";
  ui.window.dispatchEvent(new ui.window.Event("resize"));
  ui.window.document.dispatchEvent(new ui.window.Event("fullscreenchange"));
  await waitFor(() => ui.window.document.activeElement === ui.get(".xterm-helper-textarea"));
  const paste = new ui.window.Event("paste", { bubbles: true, cancelable: true });
  Object.defineProperty(paste, "clipboardData", { value: { getData: () => "echo FULLSCREEN_OK\n" } });
  ui.get(".xterm-helper-textarea").dispatchEvent(paste);
  await waitFor(() => ui.requests.some(r => r.method === "terminal.write" && r.params.data.includes("FULLSCREEN_OK")));
  ui.emit("terminal.opened"); ui.emit("terminal.opened");
  assert.equal(ui.window.document.querySelectorAll('[role="tab"]').length, 1);
});

test("Settings remains dark after system appearance broadcasts", async t => {
  const ui = await surface(t, "settings", defaults);
  [...ui.window.document.querySelectorAll("nav button")].find(b => b.textContent.endsWith("Appearance")).click();
  await waitFor(() => ui.get('[aria-label="System font"]'));
  for (const appearance of ["light", "dark"]) {
    ui.emit("desktop.changed", { ...desktop, appearance });
    assert.equal(ui.window.document.documentElement.dataset.theme, "dark");
  }
});
test("Applications exposes the desktop while dragging and creates an app shortcut", async t => {
  const ui = await surface(t, "applications", defaults);
  await waitFor(() => ui.requests.some(r => r.method === "shell.hello"));
  ui.emit("popover.shown");
  await waitFor(() => ui.get(".row"));
  const data = {};
  drag(ui.window, ui.get(".row"), "dragstart", data);
  await waitFor(() => ui.get(".menu.dragging"));
  drag(ui.window, ui.get("#scrim"), "drop", data);
  await waitFor(() => ui.requests.some(r => r.method === "desktop.add_app"));
  assert.equal(ui.requests.find(r => r.method === "desktop.add_app").params.id, "firefox.desktop");
  assert.ok(!ui.requests.some(r => r.method === "apps.uninstall"));
});

test("Desktop accepts folders whose names contain percent signs and spaces", async t => {
  const ui = await surface(t, "wallpaper", req => req.method === "desktop.add_file" ? { ...desktop, items: [] } : defaults(req));
  const uri = "file:///home/test/Folder%20100%25";
  drag(ui.window, ui.get(".desktop"), "drop", { "text/uri-list": uri });
  await waitFor(() => ui.requests.some(r => r.method === "desktop.add_file"));
  assert.equal(ui.requests.find(r => r.method === "desktop.add_file").params.path, "/home/test/Folder 100%");
});

function contrast(foreground, background) {
  const luminance = color => {
    const rgb = color.startsWith("#") ? color.slice(1).match(/../g).map(x => parseInt(x, 16)) : color.match(/[\d.]+/g).slice(0, 3).map(Number);
    const linear = rgb.map(x => { x /= 255; return x <= .04045 ? x / 12.92 : ((x + .055) / 1.055) ** 2.4; });
    return linear[0] * .2126 + linear[1] * .7152 + linear[2] * .0722;
  };
  const a = luminance(foreground), b = luminance(background);
  return (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
}
test("Terminal palettes keep default and ANSI text readable in both appearances", async () => {
  const { transform } = await import("esbuild");
  const source = await readFile(new URL("../src/surfaces/terminal/theme.ts", import.meta.url), "utf8");
  const { code } = await transform(source, { loader: "ts", format: "esm" });
  const { terminalTheme } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);
  for (const mode of ["light", "dark"]) {
    const palette = terminalTheme(mode);
    assert.ok(contrast(palette.foreground, palette.background) >= 7, `${mode} default text`);
    for (const [name, value] of Object.entries(palette)) {
      if (["background", "cursorAccent", "selectionBackground"].includes(name) || (mode === "dark" && name === "black")) continue;
      assert.ok(contrast(value, palette.background) >= 4.5, `${mode} ${name} is readable`);
    }
  }
});
test("Terminal updates rendered plain and ANSI text colors when appearance changes", async t => {
  let sent = false;
  const ui = await surface(t, "terminal", req => {
    if (req.method === "terminal.start") return { id: 81, shell: "/bin/bash" };
    if (req.method === "terminal.read") {
      const text = sent ? "" : "PROMPT \x1b[37mWHITE_TEXT\x1b[0m \x1b[97mBRIGHT_TEXT\x1b[0m \x1b[30mBLACK_TEXT\x1b[0m \x1b[38;2;250;250;250mTRUECOLOR_TEXT\x1b[0m\r\n";
      sent = true; return { data: [...new TextEncoder().encode(text)], exited: false };
    }
    return defaults(req);
  });
  await waitFor(() => ui.requests.some(r => r.method === "shell.hello"));
  ui.emit("terminal.opened");
  await waitFor(() => ui.get(".xterm-rows")?.textContent.includes("BRIGHT_TEXT"));
  for (const mode of ["light", "dark", "light"]) {
    ui.emit("desktop.changed", { ...desktop, appearance: mode });
    await waitFor(() => ui.window.document.documentElement.dataset.theme === "dark");
    const background = "#171a20";
    assert.ok(contrast(ui.window.getComputedStyle(ui.get(".xterm-rows")).color, background) >= 7, `${mode} rendered default color`);
    for (const word of ["WHITE_TEXT", "BRIGHT_TEXT", "BLACK_TEXT", "TRUECOLOR_TEXT"]) {
      const span = [...ui.window.document.querySelectorAll(".xterm-rows span")].find(span => span.textContent.includes(word));
      assert.ok(span, word);
      await waitFor(() => {
        const current = [...ui.window.document.querySelectorAll(".xterm-rows span")].find(node => node.textContent.includes(word));
        return current && contrast(ui.window.getComputedStyle(current).color, background) >= 4.5;
      });
    }
  }
});

test("Displays edits a multi-monitor layout and keeps a reviewed native trial", async t => {
  const modes = [{ width:1920,height:1080,refresh:60000,preferred:true },{ width:1280,height:720,refresh:60000,preferred:false }];
  const outputs = [{ name:"eDP-1",description:"Built-in Display",enabled:true,x:0,y:0,scale:1,transform:0,current:modes[0],modes },{ name:"HDMI-A-1",description:"External Display",enabled:true,x:1920,y:0,scale:1,transform:0,current:modes[0],modes }];
  const ui = await surface(t,"settings",r=>r.method==="settings.displays"?outputs:r.method==="settings.display_apply"?{token:"review-1",seconds:15}:r.method==="settings.get"?{wifi:true,bluetooth:true,brightness:70,volume:50}:defaults(r));
  [...ui.window.document.querySelectorAll("nav button")].find(b=>b.textContent.endsWith("Displays")).click();
  await waitFor(()=>ui.window.document.querySelectorAll(".display-list-row").length===2);
  [...ui.window.document.querySelectorAll(".display-list-row")].find(b=>b.textContent.includes("External Display")).click();
  await waitFor(()=>ui.get("#resolution"));
  ui.get("#resolution").value="1280:720:60000";ui.get("#resolution").dispatchEvent(new ui.window.Event("change",{bubbles:true}));
  [...ui.window.document.querySelectorAll(".settings-actions button")].find(b=>b.textContent==="Apply changes").click();
  await waitFor(()=>ui.get('[aria-label="Keep display settings"]'));
  const applied=ui.requests.find(r=>r.method==="settings.display_apply").params.outputs;
  assert.equal(applied.find(o=>o.name==="HDMI-A-1").width,1280);
  assert.equal(applied.find(o=>o.name==="eDP-1").width,1920);
  assert.ok(!ui.requests.some(r=>r.method==="settings.display_decide"));
  [...ui.window.document.querySelectorAll(".device-dialog button")].find(b=>b.textContent==="Keep changes").click();
  await waitFor(()=>ui.requests.some(r=>r.method==="settings.display_decide"));
  assert.deepEqual(ui.requests.find(r=>r.method==="settings.display_decide").params,{token:"review-1",keep:true});
});
test("Display trial reverts when the UI confirmation expires", async t => {
  const mode={width:1920,height:1080,refresh:60000,preferred:true};
  const ui=await surface(t,"settings",r=>r.method==="settings.displays"?[{name:"eDP-1",description:"Built-in Display",enabled:true,x:0,y:0,scale:1,transform:0,current:mode,modes:[mode]}]:r.method==="settings.display_apply"?{token:"expire-1",seconds:0}:r.method==="settings.get"?{wifi:null,bluetooth:null,brightness:70,volume:50}:defaults(r));
  [...ui.window.document.querySelectorAll("nav button")].find(b=>b.textContent.endsWith("Displays")).click();await waitFor(()=>ui.get("#resolution"));
  [...ui.window.document.querySelectorAll(".settings-actions button")].find(b=>b.textContent==="Apply changes").click();
  await waitFor(()=>ui.requests.some(r=>r.method==="settings.display_decide"));
  assert.deepEqual(ui.requests.find(r=>r.method==="settings.display_decide").params,{token:"expire-1",keep:false});
});
test("Wi-Fi joins a discovered network and clears the password after success", async t => {
  const network={ssid:"Office Wi-Fi",bssid:"AA:BB:CC:DD:EE:FF",device:"wlan0",signal:90,security:"WPA2",connected:false};
  const ui=await surface(t,"settings",r=>r.method==="settings.wifi_networks"?[network]:r.method==="settings.get"?{wifi:true,bluetooth:true,brightness:70,volume:50}:defaults(r));
  [...ui.window.document.querySelectorAll("nav button")].find(b=>b.textContent.endsWith("Wi-Fi")).click(); await waitFor(() => [...ui.window.document.querySelectorAll("button")].some(b => b.textContent === "New connection" && !b.disabled));
  [...ui.window.document.querySelectorAll("button")].find(b => b.textContent === "New connection").click(); await waitFor(()=>ui.get(".device-name strong")?.textContent==="Office Wi-Fi");
  [...ui.window.document.querySelectorAll(".settings-row button")].find(b=>b.textContent==="Connect").click();await waitFor(()=>ui.get('[aria-label="Wi-Fi password"]'));
  type(ui,'[aria-label="Wi-Fi password"]',"test-passphrase");
  ui.get(".device-dialog").dispatchEvent(new ui.window.Event("submit",{bubbles:true,cancelable:true}));
  await waitFor(()=>ui.requests.some(r=>r.method==="settings.wifi_connect"));
  assert.deepEqual(ui.requests.find(r=>r.method==="settings.wifi_connect").params,{bssid:network.bssid,device:"wlan0",password:"test-passphrase"});
  await waitFor(()=>!ui.get('[aria-label="Wi-Fi password"]'));
});
test("Bluetooth connects a paired device through its discovered BlueZ path", async t => {
  const device={path:"/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF",name:"Headphones",address:"AA:BB:CC:DD:EE:FF",paired:true,connected:false};
  const ui=await surface(t,"settings",r=>r.method==="settings.bluetooth_devices"?[device]:r.method==="settings.get"?{wifi:true,bluetooth:true,brightness:70,volume:50}:defaults(r));
  [...ui.window.document.querySelectorAll("nav button")].find(b=>b.textContent.endsWith("Bluetooth")).click();await waitFor(()=>ui.get(".device-name strong")?.textContent==="Headphones");
  [...ui.window.document.querySelectorAll(".settings-row button")].find(b=>b.textContent==="Connect").click();await waitFor(()=>ui.requests.some(r=>r.method==="settings.bluetooth_action"));
  assert.deepEqual(ui.requests.find(r=>r.method==="settings.bluetooth_action").params,{path:device.path,action:"connect"});
  assert.ok(ui.get('#bluetooth').classList.contains("lg-switch"));
});
test("Settings does not offer nonfunctional sharing switches", async t => {
  const ui = await surface(t, "settings", defaults);
  assert.equal(ui.get('nav button[aria-label="Sharing"]'), null);
  assert.equal(ui.get('[aria-label="File Sharing"]'), null);
});


test("launcher context menu pins an app without launching it", async t => {
  const ui = await surface(t, "applications", defaults);
  ui.emit("popover.shown");
  await waitFor(() => ui.get("#row-0"));
  ui.get("#row-0").dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 100, clientY: 100 }));
  await waitFor(() => ui.get(".app-context"));
  menu(ui, ".app-context", "Add to Dock");
  await waitFor(() => ui.requests.some(r => r.method === "dock.pin"));
  assert.deepEqual(ui.requests.find(r => r.method === "dock.pin").params, { id: app.id, pinned: true });
  assert.ok(!ui.requests.some(r => r.method === "search.activate"));
});

test("desktop reflects folder changes and labels real files for Trash", async t => {
  const ui = await surface(t, "wallpaper", defaults);
  const item = { id: "file:/home/test/Desktop/new.txt", name: "new.txt", kind: "file", target: "/home/test/Desktop/new.txt", icon: null };
  ui.emit("desktop.changed", { ...desktop, items: [item] });
  await waitFor(() => ui.get('[aria-label="new.txt"]'));
  ui.get('[aria-label="new.txt"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get(".desktop-context"));
  assert.ok([...ui.window.document.querySelectorAll('.desktop-context button')].some(b => b.textContent === "Move to Trash"));
  ui.emit("desktop.changed", desktop);
  await waitFor(() => !ui.get('[aria-label="new.txt"]'));
});

test("Settings Home leads to working settings and search opens a matching page", async t => {
  const ui = await surface(t, "settings", req => req.method === "settings.get" ? { wifi: true, bluetooth: false, brightness: 75, volume: 42 } : defaults(req));
  assert.equal(ui.get("#settings-title").textContent, "Home");
  assert.ok(!ui.get(".welcome-banner"));
  assert.ok(!ui.get(".settings-content").textContent.includes("Coming soon"));
  type(ui, '[aria-label="Search settings"]', "wallpaper");
  await waitFor(() => ui.window.document.querySelectorAll("nav button").length === 1);
  ui.get('[aria-label="Search settings"]').dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  await waitFor(() => ui.get("#settings-title").textContent === "Wallpaper");
  ui.get('[aria-label="Clear settings search"]').click();
  await waitFor(() => ui.get('nav [aria-label="Sound"]'));
  ui.get('nav [aria-label="Sound"]').click();
  await waitFor(() => ui.get('[aria-label="Volume"]'));
  assert.equal(ui.get('[aria-label="Volume"]').value, "42");
});

test("Finder navigates files with the keyboard and traps focus in rename dialogs", async t => {
  const entries = ["Documents", "Downloads"].map(name => ({ name, path: `/home/test/${name}`, directory: true, symlink: false, size: 0, modified: 0 }));
  const ui = await surface(t, "finder", req => req.method === "files.locations" ? [{ name: "Home", path: "/home/test" }] : req.method === "files.list" ? { path: req.params.path, parent: "/home", entries, truncated: false } : defaults(req));
  await waitFor(() => ui.get('[aria-label="Documents"]'));
  ui.get('[aria-label="Documents"]').click();
  await waitFor(() => ui.get('[aria-label="Documents"]').getAttribute("aria-pressed") === "true");
  ui.get('[aria-label="Documents"]').dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
  await waitFor(() => ui.get('[aria-label="Downloads"]').getAttribute("aria-pressed") === "true");
  ui.get('[aria-label="Downloads"]').dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "F2", bubbles: true }));
  await waitFor(() => ui.get('[role="dialog"] [aria-label="Name"]'));
  const input = ui.get('[aria-label="Name"]'), submit = ui.get('[role="dialog"] button[type="submit"]');
  await waitFor(() => ui.window.document.activeElement === input);
  submit.focus(); submit.dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true }));
  assert.equal(ui.window.document.activeElement, input);
  input.dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await waitFor(() => !ui.get('[role="dialog"]'));
  assert.ok(!ui.requests.some(req => req.method === "files.rename"));
});

test("launcher context menu supports arrow keys and Escape without launching", async t => {
  const ui = await surface(t, "applications", defaults);
  ui.emit("popover.shown"); await waitFor(() => ui.get("#row-0"));
  ui.get("#row-0").dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get(".app-context"));
  const menu = ui.get(".app-context"), buttons = [...menu.querySelectorAll("button:not(:disabled)")];
  await waitFor(() => ui.window.document.activeElement === buttons[0]);
  buttons[0].dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }));
  assert.equal(ui.window.document.activeElement, buttons[1]);
  buttons[1].dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await waitFor(() => !ui.get(".app-context"));
  assert.ok(!ui.requests.some(req => ["search.activate", "dock.pin"].includes(req.method)));
});

test("desktop stays uncluttered and file drags show the move action", async t => {
  const ui = await surface(t, "wallpaper", defaults);
  await waitFor(() => ui.get(".desktop"));
  assert.ok(!ui.get(".desktop-welcome"));
  drag(ui.window, ui.get(".desktop"), "dragover", { "text/uri-list": "file:///home/test/notes.txt" });
  await waitFor(() => ui.get(".desktop-drop-indicator"));
  assert.equal(ui.get(".desktop-drop-indicator").textContent, "Move to Desktop");
});

test("Terminal shortcuts create, switch and close tabs without sending shell commands", async t => {
  let id = 0;
  const ui = await surface(t, "terminal", req => req.method === "terminal.start" ? { id: ++id, shell: "/bin/bash" } : req.method === "terminal.read" ? { data: [], exited: false } : defaults(req));
  await waitFor(() => ui.requests.some(req => req.method === "shell.hello"));
  const key = (value, shiftKey = false) => ui.window.dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: value, ctrlKey: true, shiftKey, bubbles: true, cancelable: true }));
  await waitFor(() => !ui.get('[aria-label="New terminal session"]').disabled);
  key("T", true); await waitFor(() => ui.window.document.querySelectorAll('[role="tab"]').length === 1);
  await waitFor(() => ui.get('.terminal-session.active .xterm-helper-textarea'));
  const shellKey = (key, code, keyCode, shiftKey = false) => ui.get('.terminal-session.active .xterm-helper-textarea').dispatchEvent(new ui.window.KeyboardEvent("keydown", { key, code, keyCode, which: keyCode, ctrlKey: true, shiftKey, bubbles: true, cancelable: true }));
  shellKey("T", "KeyT", 84, true); await waitFor(() => ui.window.document.querySelectorAll('[role="tab"]').length === 2);
  await waitFor(() => ui.get('.terminal-session.active .xterm-helper-textarea'));
  shellKey("PageUp", "PageUp", 33); await waitFor(() => ui.get('[role="tab"][aria-selected="true"]').textContent.includes("Session 1"));
  shellKey("W", "KeyW", 87, true); await waitFor(() => ui.window.document.querySelectorAll('[role="tab"]').length === 1);
  assert.ok(!ui.requests.some(req => req.method === "terminal.write"));
});

test("late desktop snapshots cannot replace a newer folder change", async t => {
  const pending = [];
  const ui = await surface(t, "wallpaper", req => req.method === "desktop.get" ? new Promise(resolve => pending.push(resolve)) : defaults(req));
  await waitFor(() => pending.length >= 2);
  const item = { id: "file:/home/test/Desktop/latest.txt", name: "latest.txt", kind: "file", target: "/home/test/Desktop/latest.txt", icon: null };
  ui.emit("desktop.changed", { ...desktop, items: [item] });
  await waitFor(() => ui.get('[aria-label="latest.txt"]'));
  pending.forEach(resolve => resolve(desktop));
  await new Promise(resolve => setTimeout(resolve, 30));
  assert.ok(ui.get('[aria-label="latest.txt"]'));
});

test("password visibility preserves the input and Caps Lock feedback is clear", async t => {
  const ui = await surface(t, "locker", defaults);
  await waitFor(() => !ui.get("#password").disabled);
  type(ui, "#password", "example-password");
  ui.get('[aria-label="Show password"]').click();
  await waitFor(() => ui.get("#password").type === "text");
  assert.equal(ui.get("#password").value, "example-password");
  ui.get('[aria-label="Hide password"]').click();
  await waitFor(() => ui.get("#password").type === "password");
  const event = new ui.window.KeyboardEvent("keydown", { key: "A", bubbles: true });
  Object.defineProperty(event, "getModifierState", { value: name => name === "CapsLock" });
  ui.get("#password").dispatchEvent(event);
  await waitFor(() => ui.get(".caps-lock-hint"));
  assert.equal(ui.get(".caps-lock-hint").textContent, "Caps Lock is on");
  assert.ok(!ui.requests.some(req => req.method === "locker.unlock"));
});

test("Terminal wheel scrolls history and jumps back to latest", async t => {
  let sent = false;
  const ui = await surface(t, "terminal", req => {
    if (req.method === "terminal.start") return { id: 92, shell: "/bin/bash" };
    if (req.method === "terminal.read") {
      const data = sent ? [] : [...new TextEncoder().encode(Array.from({ length: 120 }, (_, i) => `history line ${i}\r\n`).join(""))];
      sent = true;
      return { data, exited: false };
    }
    return defaults(req);
  });
  await waitFor(() => ui.requests.some(r => r.method === "shell.hello"));
  ui.emit("terminal.opened");
  await waitFor(() => ui.get(".xterm-rows")?.textContent.includes("history line 119"));
  ui.get(".xterm").dispatchEvent(new ui.window.WheelEvent("wheel", { deltaY: -12, deltaMode: 1, bubbles: true, cancelable: true }));
  await waitFor(() => ui.get(".terminal-follow"));
  await waitFor(() => !ui.get(".xterm-rows").textContent.includes("history line 119"));
  ui.get(".terminal-follow").click();
  await waitFor(() => !ui.get(".terminal-follow"));
  await waitFor(() => ui.get(".xterm-rows").textContent.includes("history line 119"));
});

test("terminal wheel normalization handles pixels, lines, pages and fractional trackpad deltas", async () => {
  const { transform } = await import("esbuild");
  const source = await readFile(new URL("../src/surfaces/terminal/scroll.ts", import.meta.url), "utf8");
  const { code } = await transform(source, { loader: "ts", format: "esm" });
  const { wheelScroller } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);
  const scroll = wheelScroller();
  assert.equal(scroll(4, 0, 16, 24), 0);
  assert.equal(scroll(4, 0, 16, 24), 0);
  assert.equal(scroll(8, 0, 16, 24), 1);
  assert.equal(scroll(-3, 1, 16, 24), -3);
  assert.equal(scroll(1, 2, 16, 24), 24);
  assert.equal(scroll(Number.NaN, 0, 16, 24), 0);
});

test("Finder opens Terminal at the background folder, selected folder, or file parent", async t => {
  const ui = await surface(t, "finder", finderReply);
  await waitFor(() => ui.get('[aria-label="Projects"]'));
  for (const [selector, label, path] of [
    [".file-collection", "Open Terminal Here", "/home/test"],
    ['[aria-label="Projects"]', "Open in Terminal", folder.path],
    ['[aria-label="notes.txt"]', "Open in Terminal", "/home/test"],
  ]) {
    const before = ui.requests.filter(r => r.method === "terminal.open_at").length;
    ui.get(selector).dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
    await waitFor(() => ui.get(".finder-context"));
    menu(ui, ".finder-context", label);
    await waitFor(() => ui.requests.filter(r => r.method === "terminal.open_at").length > before);
    assert.deepEqual(ui.requests.filter(r => r.method === "terminal.open_at").at(-1).params, { path });
    await waitFor(() => !ui.get(".finder-context"));
  }
  ui.get('[aria-label="Projects"]').dispatchEvent(new ui.window.MouseEvent("dblclick", { bubbles: true }));
  await waitFor(() => ui.get(".folder-empty"));
  ui.get(".folder-empty h2").dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get(".finder-context"));
  menu(ui, ".finder-context", "Open Terminal Here");
  await waitFor(() => ui.requests.filter(r => r.method === "terminal.open_at").length === 4);
  assert.equal(ui.requests.filter(r => r.method === "terminal.open_at").at(-1).params.path, folder.path);
});

const pulseResource = (id, kind, percent, extra = {}) => ({ id, kind, name: kind === "cpu" ? "CPU" : id, detail: `${kind} device`, percent, used: null, total: null, read_rate: null, write_rate: null, ...extra });
const pulseData = {
  resources: [
    pulseResource("cpu", "cpu", 24.5),
    pulseResource("Memory", "memory", 50, { used: 8589934592, total: 17179869184 }),
    pulseResource("GPU", "gpu", null, { detail: "Usage unavailable from this driver" }),
    pulseResource("nvme0n1", "storage", null, { total: 1024000000000, read_rate: 1048576, write_rate: 524288 }),
    pulseResource("wlan0", "network", null, { read_rate: 2097152, write_rate: 1024 }),
  ],
  processes: [
    { pid: 42, name: "firefox", state: "R", cpu: 12.5, memory: 209715200 },
    { pid: 7, name: "meridian-shell", state: "S", cpu: 3.5, memory: 524288000 },
    { pid: 101, name: "worker", state: "S", cpu: null, memory: 1048576 },
  ],
};

test("Pulse displays monochrome resource percentages and a visible process list", async t => {
  const ui = await surface(t, "pulse", req => req.method === "pulse.snapshot" ? pulseData : defaults(req));
  await waitFor(() => ui.get(".pulse-summary-primary"));
  await waitFor(() => ui.get("tbody tr"));
  assert.ok(ui.get(".pulse-summary-primary").textContent.includes("24.5%"));
  assert.ok(ui.get(".pulse-summary-primary").textContent.includes("50.0%"));
  assert.equal(ui.window.document.querySelectorAll(".pulse-resource-row").length, 3);
  assert.ok(ui.get(".pulse-resource-list").textContent.includes("1.0 MiB/s"));
  assert.ok(ui.get(".pulse-memory-detail").textContent.includes("16.0 GiB"));
  assert.ok(ui.get(".pulse-resource-list").textContent.includes("Usage unavailable from this driver"));
  assert.equal(ui.window.document.querySelectorAll("svg, canvas, .pulse-chart, .resource-card, .process-dot").length, 0);
  assert.ok(ui.get("tbody tr").textContent.includes("firefox"));
  assert.ok(ui.get("tbody tr").textContent.includes("1.2%"));
  assert.ok([...ui.window.document.querySelectorAll("th")].some(th => th.textContent.includes("RAM %")));
  ui.emit("desktop.changed", { ...desktop, appearance: "dark" });
  await waitFor(() => ui.window.document.documentElement.dataset.theme === "dark");
});

test("Pulse processes support sorting and searching without switching views", async t => {
  const ui = await surface(t, "pulse", req => req.method === "pulse.snapshot" ? pulseData : defaults(req));
  await waitFor(() => ui.get("tbody tr"));
  assert.ok(ui.get("tbody tr").textContent.includes("firefox"));
  [...ui.window.document.querySelectorAll("th button")].find(b => b.textContent.startsWith("Memory")).click();
  await waitFor(() => ui.get("tbody tr").textContent.includes("meridian-shell"));
  assert.equal(ui.get('th[aria-sort="descending"]').textContent, "Memory ↓");
  type(ui, '[aria-label="Search processes"]', "101");
  await waitFor(() => ui.window.document.querySelectorAll("tbody tr").length === 1);
  assert.ok(ui.get("tbody tr").textContent.includes("worker"));
  assert.ok(ui.get("tbody tr").textContent.includes("—"));
  type(ui, '[aria-label="Search processes"]', "missing");
  await waitFor(() => ui.get(".process-empty")?.textContent.includes("No matching"));
});

test("Pulse has no pause control and stops polling while closed", async t => {
  const ui = await surface(t, "pulse", req => req.method === "pulse.snapshot" ? pulseData : defaults(req));
  await waitFor(() => ui.get("tbody tr"));
  assert.ok(!ui.get(".pause-button"));
  assert.ok(ui.get(".pulse-header") && !ui.get(".pulse-footer") && !ui.get(".overview-intro"));
  const count = () => ui.requests.filter(r => r.method === "pulse.snapshot").length;
  const before = count();
  ui.emit("pulse.visibility", false);
  await new Promise(r => setTimeout(r, 2100));
  assert.equal(count(), before);
  ui.emit("pulse.visibility", true);
  await waitFor(() => count() === before + 1);
});

test("Pulse reports metric failures and can recover without losing its window", async t => {
  let failed = true;
  const ui = await surface(t, "pulse", req => req.method === "pulse.snapshot" ? failed ? { error: { code: "failed", message: "Metrics temporarily unavailable" } } : pulseData : defaults(req));
  await waitFor(() => ui.get('[role="alert"]'));
  assert.ok(ui.get('[role="alert"]').textContent.includes("Metrics temporarily unavailable"));
  failed = false;
  ui.emit("pulse.visibility", false);
  await new Promise(r => setTimeout(r, 20));
  ui.emit("pulse.visibility", true);
  await waitFor(() => ui.get("tbody tr") && !ui.get('[role="alert"]'));
});

test("Meridian menu opens Pulse", async t => {
  const ui = await surface(t, "desktop-menu", defaults);
  await waitFor(() => ui.requests.some(r => r.method === "shell.hello"));
  ui.emit("popover.shown"); ui.emit("menu.opened", { menu: "Meridian", x: 8 });
  await waitFor(() => [...ui.window.document.querySelectorAll('button')].some(b => b.textContent === "Pulse…"));
  menu(ui, ".desktop-menu", "Pulse…");
  await waitFor(() => ui.requests.some(r => r.method === "pulse.open"));
});

test("Orbit bookmarks folders and sends reviewed file transfers", async t => {
  const ui = await surface(t, "finder", finderReply);
  await waitFor(() => ui.get('[aria-label="Projects"]'));
  ui.get('[aria-label="Bookmark folder"]').click();
  await waitFor(() => ui.get('nav[aria-label="Bookmarks"]'));
  assert.deepEqual(ui.requests.find(r => r.method === "files.bookmark_set").params, { path: "/home/test" });
  ui.get('[aria-label="notes.txt"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get(".finder-context")); menu(ui, ".finder-context", "Copy To…");
  await waitFor(() => ui.get('[aria-label="Destination folder"]'));
  type(ui, '[aria-label="Destination folder"]', "/home/test/Projects");
  assert.ok(!ui.requests.some(r => r.method === "files.transfer_start"));
  ui.get('form[role="dialog"] button[type="submit"]').click();
  await waitFor(() => ui.requests.some(r => r.method === "files.transfer_start"));
  assert.deepEqual(ui.requests.find(r => r.method === "files.transfer_start").params, { path: file.path, destination: folder.path, move_file: false });
});

test("Orbit previews images and sets a selected image as wallpaper", async t => {
  const image = { ...file, name: "photo.png", path: "/home/test/photo.png" };
  const ui = await surface(t, "finder", r => r.method === "files.list" ? { path: "/home/test", parent: "/home", entries: [image], truncated: false } : r.method === "files.thumbnail" ? "data:image/png;base64,test" : r.method === "files.wallpaper" ? desktop : finderReply(r));
  await waitFor(() => ui.get(".file-thumbnail img"));
  assert.equal(ui.requests.find(r => r.method === "files.thumbnail").params.path, image.path);
  ui.get('[aria-label="photo.png"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get(".finder-context")); menu(ui, ".finder-context", "Set as Wallpaper");
  await waitFor(() => ui.requests.some(r => r.method === "files.wallpaper"));
  assert.deepEqual(ui.requests.find(r => r.method === "files.wallpaper").params, { path: image.path });
});

test("shared dropdown supports keyboard selection and four system fonts", async t => {
  const ui = await surface(t, "settings", r => r.method === "desktop.font_set" ? { ...desktop, font: r.params.id } : r.method === "desktop.get" ? { ...desktop, font: "Inter" } : defaults(r));
  ui.get('nav [aria-label="Appearance"]').click();
  await waitFor(() => ui.get('.select-trigger[aria-label="System font"]'));
  assert.equal(ui.get("#system-font").options.length, 4);
  ui.get('.select-trigger[aria-label="System font"]').click();
  await waitFor(() => ui.get('[role="listbox"]'));
  ui.get('[role="listbox"]').dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
  await waitFor(() => ui.get('[role="option"][data-highlighted="true"]').textContent.startsWith("Noto Sans"));
  assert.ok(ui.get('[role="option"][aria-selected="true"]').textContent.startsWith("Inter"));
  ui.get('[role="listbox"]').dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  await waitFor(() => ui.requests.some(r => r.method === "desktop.font_set"));
  assert.deepEqual(ui.requests.find(r => r.method === "desktop.font_set").params, { id: "Noto Sans" });
  assert.ok(!ui.get('[role="listbox"]'));
  ui.emit("desktop.changed", { ...desktop, font: "Noto Sans" });
  await waitFor(() => ui.window.document.documentElement.style.getPropertyValue("--font-ui").includes("Noto Sans"));
});

test("top bar uses its logo and opens system notifications with unread state", async t => {
  const ui = await surface(t, "topbar", r => r.method === "notifications.get" ? { available: true, items: [] } : defaults(r));
  assert.ok(ui.get(".menu-brand svg")); assert.equal(ui.get(".menu-brand").textContent, "Meridian");
  ui.emit("notifications.changed", { available: true, items: [{ id: 1, unread: true }] });
  await waitFor(() => ui.get('[aria-label="Notifications, 1 unread"]'));
  ui.get("#notifications").click();
  await waitFor(() => ui.requests.some(r => r.method === "popover.toggle" && r.params.popover === "notifications"));
  ui.emit("notifications.changed", { available: true, items: [] });
  await waitFor(() => !ui.get(".notification-dot"));
});

test("notification center receives app messages, invokes actions and dismisses", async t => {
  const item = { id: 4, app: "Mail", summary: "New message", body: "Hello", time: Date.now(), unread: true, actions: [["open", "Open"]] };
  const ui = await surface(t, "notifications", r => r.method.startsWith("notifications.") ? { available: true, items: r.method === "notifications.get" ? [item] : [] } : defaults(r));
  await waitFor(() => ui.requests.some(r => r.method === "shell.hello"));
  ui.emit("popover.shown");
  await waitFor(() => ui.get(".notification h3"));
  assert.equal(ui.get(".notification h3").textContent, "New message");
  ui.get(".notification-actions button").click();
  await waitFor(() => ui.requests.some(r => r.method === "notifications.action"));
  assert.deepEqual(ui.requests.find(r => r.method === "notifications.action").params, { id: 4, key: "open" });
  await waitFor(() => ui.get(".notification-empty"));
  ui.emit("notifications.changed", { available: true, items: [item] });
  await waitFor(() => ui.get('[aria-label="Dismiss New message"]'));
  ui.get('[aria-label="Dismiss New message"]').click();
  await waitFor(() => ui.requests.some(r => r.method === "notifications.dismiss"));
  assert.deepEqual(ui.requests.find(r => r.method === "notifications.dismiss").params, { id: 4 });
});


test("missing third-party icons try the theme and retain a visible monogram", async t => {
  const ui = await surface(t, "panel", defaults);
  await waitFor(() => ui.get('[aria-label="Firefox"] img'));
  ui.get('[aria-label="Firefox"] img').dispatchEvent(new ui.window.Event("error"));
  await waitFor(() => ui.get('[aria-label="Firefox"] img')?.src.split("?")[0] === "meridian://icons/firefox");
  ui.get('[aria-label="Firefox"] img').dispatchEvent(new ui.window.Event("error"));
  await waitFor(() => ui.get('[aria-label="Firefox"] img')?.src.includes('/generic.svg'));
  ui.get('[aria-label="Firefox"] img').dispatchEvent(new ui.window.Event("error"));
  await waitFor(() => !ui.get('[aria-label="Firefox"] img'));
  assert.equal(ui.get('[aria-label="Firefox"] .app-icon').textContent, "F");
});

test("Startup Apps toggles a per-user entry and adds an installed app for next login", async t => {
  let entries = [{ id: "mail.desktop", name: "Mail", description: "Email", command: "mail", enabled: false, available: true, reason: "", user: false }];
  const ui = await surface(t, "settings", req => {
    if (req.method === "settings.startup") return entries;
    if (req.method === "settings.startup_set") { entries = entries.map(e => ({ ...e, enabled: req.params.enabled, user: true })); return entries; }
    if (req.method === "settings.startup_apps") return [app];
    if (req.method === "settings.startup_add") { entries = [...entries, { ...entries[0], id: app.id, name: app.title, enabled: true }]; return entries; }
    return defaults(req);
  });
  ui.get('nav [aria-label="Startup Apps"]').click();
  await waitFor(() => ui.get('[aria-label="Start Mail at sign-in"]') && !ui.get('[aria-label="Start Mail at sign-in"]').disabled);
  await waitFor(() => ui.window.glisseSwitchReady(ui.get('[aria-label="Start Mail at sign-in"]')));
  ui.get('[aria-label="Start Mail at sign-in"]').click();
  await waitFor(() => ui.get('[aria-label="Start Mail at sign-in"]').getAttribute("aria-checked") === "true");
  assert.deepEqual(ui.requests.find(r => r.method === "settings.startup_set").params, { id: "mail.desktop", enabled: true });
  [...ui.window.document.querySelectorAll("button")].find(b => b.textContent.includes("Add application")).click();
  await waitFor(() => ui.get('[aria-label="Search installed applications"]'));
  await waitFor(() => ui.get('.startup-app-picker button'));
  ui.get('.startup-app-picker button').click();
  await waitFor(() => ui.get('[aria-label="Start Firefox at sign-in"]'));
  assert.equal(ui.get('[role="dialog"]'), null);
  assert.ok(!ui.requests.some(r => r.method === "search.activate"));
  assert.ok(ui.get('.startup-message').textContent.includes("next time you sign in"));
});

test("Startup Apps reports errors without changing the saved switch", async t => {
  const entries = [{ id: "mail.desktop", name: "Mail", description: "", command: "mail", enabled: true, available: true, reason: "", user: true }];
  const ui = await surface(t, "settings", req => req.method === "settings.startup" ? entries : req.method === "settings.startup_set" ? { error: { code: "failed", message: "Startup folder is read-only" } } : defaults(req));
  ui.get('nav [aria-label="Startup Apps"]').click();
  await waitFor(() => ui.get('[aria-label="Start Mail at sign-in"]') && !ui.get('[aria-label="Start Mail at sign-in"]').disabled);
  await waitFor(() => ui.window.glisseSwitchReady(ui.get('[aria-label="Start Mail at sign-in"]')));
  ui.get('[aria-label="Start Mail at sign-in"]').click();
  await waitFor(() => ui.get('[role="alert"]'));
  assert.ok(ui.get('[role="alert"]').textContent.includes("read-only"));
  assert.equal(ui.get('[aria-label="Start Mail at sign-in"]').getAttribute("aria-checked"), "true");
  assert.equal(ui.get('[aria-label="Start Mail at sign-in"]').disabled, false);
});

test("Orbit opens read-only properties with Alt+Enter and navigates breadcrumb folders", async t => {
  const file = { name: "notes.txt", path: "/home/test/Documents/notes.txt", directory: false, symlink: false, size: 1240, modified: 12345 };
  const ui = await surface(t, "finder", req => req.method === "files.locations" ? [{ name: "Documents", path: "/home/test/Documents" }] : req.method === "files.list" ? { path: req.params.path, parent: "/home/test", entries: [file], truncated: false } : req.method === "files.properties" ? { name: file.name, path: file.path, kind: "Text document", size: file.size, modified: 12345, created: null, owner: "test", group: "test", permissions: "rw-r--r-- (644)", link_target: null } : defaults(req));
  await waitFor(() => ui.get('[aria-label="notes.txt"]'));
  const button = ui.get('[aria-label="notes.txt"]'); button.click(); button.focus();
  await waitFor(() => button.getAttribute("aria-pressed") === "true");
  button.dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "Enter", altKey: true, bubbles: true, cancelable: true }));
  await waitFor(() => ui.get('.properties-dialog dd'));
  assert.ok(ui.get('.properties-dialog').textContent.includes("Text document"));
  assert.ok(ui.get('.properties-dialog').textContent.includes("1.2 KB"));
  assert.ok(ui.get('.properties-dialog').textContent.includes("rw-r--r--"));
  assert.ok(!ui.requests.some(r => r.method === "files.open"));
  ui.get('[aria-label="Close properties"]').dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await waitFor(() => !ui.get('.properties-dialog'));
  assert.equal(ui.window.document.activeElement, button);
  [...ui.window.document.querySelectorAll('.finder-breadcrumbs button')].find(b => b.textContent === "test").click();
  await waitFor(() => ui.requests.some(r => r.method === "files.list" && r.params.path === "/home/test"));
});

test("launcher refreshes its app results when the catalog changes", async t => {
  const other = { ...app, id: "mail.desktop", title: "Mail" };
  let items = [app, other];
  const ui = await surface(t, "applications", req => req.method === "search.query" ? { serial: req.params.serial, sections: [{ provider: "apps", items }] } : defaults(req));
  ui.emit("popover.shown");
  await waitFor(() => ui.get('#row-1') && !ui.get('#menu').classList.contains('hidden'));
  assert.match(ui.get('#row-0').textContent, /Firefox/);
  items = [other]; ui.emit("search.invalidated");
  await waitFor(() => ui.get('#row-0')?.textContent.includes("Mail") && !ui.get('#row-1'));
  items = []; ui.emit("search.invalidated");
  await waitFor(() => ui.get('#empty'));
  assert.match(ui.get('#empty').textContent, /No apps available/);
});

test("Sound selects a playback device and controls microphone mute and level", async t => {
  let audio = { devices: [
    { id: 5, name: "speakers", description: "Built-in speakers", kind: "output", default: true },
    { id: 6, name: "headphones", description: "Headphones", kind: "output", default: false },
    { id: 7, name: "mic", description: "Microphone", kind: "input", default: true },
  ], output_volume: 42, input_volume: 75, output_muted: false, input_muted: false };
  const ui = await surface(t, "settings", req => {
    if (req.method === "settings.audio") return audio;
    if (req.method === "settings.audio_action") {
      if (req.params.action === "default") audio.devices.forEach(d => { if (d.kind === "output") d.default = d.id === req.params.id; });
      if (req.params.action === "mute") audio.input_muted = true;
      if (req.params.action === "volume") audio.input_volume = req.params.percent;
      return audio;
    }
    return defaults(req);
  });
  ui.get('nav [aria-label="Sound"]').click();
  await waitFor(() => [...ui.window.document.querySelectorAll(".sound-device-list .sound-option-row")].some(row => row.textContent.includes("Headphones")));
  const output = [...ui.window.document.querySelectorAll(".sound-device-list .sound-option-row")].find(row => row.textContent.includes("Headphones"));
  output.click();
  await waitFor(() => ui.get('.startup-message'));
  assert.deepEqual(ui.requests.find(r => r.method === "settings.audio_action").params, { id: 6, name: "headphones", action: "default", percent: null });
  ui.get('[aria-label="Mute microphone"]').click();
  await waitFor(() => ui.get('[aria-label="Unmute microphone"]'));
  const level = ui.get('[aria-label="Microphone level"]');
  const setter = Object.getOwnPropertyDescriptor(ui.window.HTMLInputElement.prototype, "value").set;
  setter.call(level, "60"); level.dispatchEvent(new ui.window.Event("input", { bubbles: true }));
  level.dispatchEvent(new ui.window.KeyboardEvent("keyup", { key: "ArrowLeft", bubbles: true }));
  await waitFor(() => ui.requests.some(r => r.method === "settings.audio_action" && r.params.percent === 60));
  assert.equal(ui.requests.find(r => r.method === "settings.audio_action" && r.params.action === "volume").params.name, "mic");
});

test("Trash restores only the selected item and can show its original folder", async t => {
  const first = { uri: "trash:///notes.txt", name: "notes.txt", original: "/home/test/Documents/notes.txt", deleted: "2026-10-17T10:00:00", size: 1200, directory: false };
  const second = { ...first, uri: "trash:///other.txt", name: "other.txt" };
  const ui = await surface(t, "finder", req => req.method === "files.locations" ? [{ name: "Home", path: "/home/test" }] : req.method === "files.list" ? { path: req.params.path, parent: "/home", entries: [], truncated: false } : req.method === "files.trash_list" ? { entries: [first, second], truncated: false } : req.method === "files.trash_restore" ? first.original : defaults(req));
  await waitFor(() => ui.get('.finder-sidebar button'));
  [...ui.window.document.querySelectorAll('.finder-sidebar button')].find(b => b.textContent === "Trash").click();
  await waitFor(() => ui.get('.trash-item'));
  assert.ok(ui.get('.restore-button').disabled);
  ui.get('[aria-label="notes.txt"]').click();
  await waitFor(() => !ui.get('.restore-button').disabled);
  ui.get('.restore-button').click();
  await waitFor(() => ui.get('.trash-restored'));
  assert.deepEqual(ui.requests.find(r => r.method === "files.trash_restore").params, { uri: first.uri });
  assert.equal(ui.get('[aria-label="notes.txt"]'), null);
  assert.ok(ui.get('[aria-label="other.txt"]'));
  ui.get('.trash-restored button').click();
  await waitFor(() => ui.requests.some(r => r.method === "files.list" && r.params.path === "/home/test/Documents"));
});

test("Trash preserves the item and reports an original-location conflict", async t => {
  const item = { uri: "trash:///notes.txt", name: "notes.txt", original: "/home/test/notes.txt", deleted: "", size: 1200, directory: false };
  const ui = await surface(t, "finder", req => req.method === "files.locations" ? [{ name: "Home", path: "/home/test" }] : req.method === "files.list" ? { path: req.params.path, parent: "/home", entries: [], truncated: false } : req.method === "files.trash_list" ? { entries: [item], truncated: false } : req.method === "files.trash_restore" ? { error: { code: "failed", message: "An item with the same name already exists" } } : defaults(req));
  ui.emit("finder.location", { path: "@trash" });
  await waitFor(() => ui.get('.trash-item'));
  ui.get('.trash-item').click(); await waitFor(() => !ui.get('.restore-button').disabled);
  ui.get('.restore-button').click(); await waitFor(() => ui.get('.trash-error'));
  assert.ok(ui.get('.trash-error').textContent.includes("already exists"));
  assert.ok(ui.get('.trash-item'));
  assert.equal(ui.get('.restore-button').disabled, false);
});

test("top bar clock opens the configured Calendar application", async t => {
  const ui = await surface(t, "topbar", defaults);
  ui.get('[aria-label="Open Calendar"]').click();
  await waitFor(() => ui.requests.some(r => r.method === "search.activate"));
  assert.deepEqual(ui.requests.find(r => r.method === "search.activate").params, { provider: "apps", item_id: "org.gnome.Calendar.desktop" });
});

test("Power preferences save supported lock timing and explain when it applies", async t => {
  const ui = await surface(t, "settings", req => req.method === "power.status" ? { on_ac: false, batteries: [{ name: "BAT0", percent: 72, status: "Discharging", health: 96 }] } : req.method === "settings.idle" ? { lock_after: 300, blank_after: 330 } : req.method === "settings.idle_set" ? req.params : defaults(req));
  ui.emit("preferences.page", { page: "power" });
  await waitFor(() => ui.get('[aria-label="Automatically lock after"]')?.disabled === false);
  const lock = ui.get('[aria-label="Automatically lock after"]'); lock.value = "600"; lock.dispatchEvent(new ui.window.Event("change", { bubbles: true }));
  await waitFor(() => [...ui.window.document.querySelectorAll('button')].some(b => b.textContent === "Save changes" && !b.disabled));
  [...ui.window.document.querySelectorAll('button')].find(b => b.textContent === "Save changes").click();
  await waitFor(() => ui.get('.startup-message'));
  assert.deepEqual(ui.requests.find(r => r.method === "settings.idle_set").params, { lock_after: 600, blank_after: 630 });
  assert.ok(ui.get('.startup-message').textContent.includes("next time you sign in"));
  assert.equal(ui.get('[role="meter"]').getAttribute("aria-valuenow"), "72");
  assert.ok(ui.get('.power-source').textContent.includes("Running on battery"));
});

test("top bar battery opens Power preferences and absent batteries stay hidden", async t => {
  const ui = await surface(t, "topbar", req => req.method === "power.status" ? { on_ac: false, batteries: [{ name: "BAT0", percent: 12, status: "Discharging", health: null }] } : defaults(req));
  await waitFor(() => ui.get('.taskbar-battery'));
  assert.ok(ui.get('.taskbar-battery').classList.contains("low"));
  ui.get('.taskbar-battery').click();
  await waitFor(() => ui.requests.some(r => r.method === "preferences.page_open"));
  assert.deepEqual(ui.requests.find(r => r.method === "preferences.page_open").params, { page: "power" });
  const desktop = await surface(t, "topbar", req => req.method === "power.status" ? { on_ac: true, batteries: [] } : defaults(req));
  await new Promise(resolve => setTimeout(resolve, 20));
  assert.equal(desktop.get('.taskbar-battery'), null);
});

test("notification mutation replies cannot discard a newer incoming message", async t => {
  const old = { id: 1, app: "Mail", summary: "Earlier message", body: "", time: Date.now() - 172800000, unread: false, actions: [] };
  const latest = { ...old, id: 2, summary: "New arrival", time: Date.now() };
  let finish;
  const ui = await surface(t, "notifications", req => req.method === "notifications.get" ? { available: true, items: [old] } : req.method === "notifications.dismiss" ? new Promise(resolve => { finish = resolve; }) : defaults(req));
  ui.emit("popover.shown");
  await waitFor(() => ui.get('[aria-label="Dismiss Earlier message"]'));
  ui.get('[aria-label="Dismiss Earlier message"]').click(); await waitFor(() => finish);
  ui.emit("notifications.changed", { available: true, items: [latest] });
  await waitFor(() => ui.get('[aria-label="Dismiss New arrival"]'));
  finish({ available: true, items: [] });
  await new Promise(resolve => setTimeout(resolve, 30));
  assert.ok(ui.get('[aria-label="Dismiss New arrival"]'));
  assert.ok(ui.get('.notification-group-title').textContent.includes("Today"));
});

test("Orbit refreshes external folder changes while preserving selection", async t => {
  const file = { name: "notes.txt", path: "/home/test/notes.txt", directory: false, symlink: false, size: 10, modified: 0 };
  let entries = [file];
  const ui = await surface(t, "finder", req => req.method === "files.locations" ? [{ name: "Home", path: "/home/test" }] : req.method === "files.list" ? { path: req.params.path, parent: "/home", entries, truncated: false } : defaults(req));
  await waitFor(() => ui.get('[aria-label="notes.txt"]'));
  ui.get('[aria-label="notes.txt"]').click();
  await waitFor(() => ui.get('[aria-label="notes.txt"]').getAttribute("aria-pressed") === "true");
  const before = ui.requests.filter(r => r.method === "files.list").length;
  ui.emit("files.changed", { path: "/some/other/folder" });
  await new Promise(resolve => setTimeout(resolve, 20));
  assert.equal(ui.requests.filter(r => r.method === "files.list").length, before);
  entries = [file, { ...file, name: "new.txt", path: "/home/test/new.txt" }];
  ui.emit("files.changed", { path: "/home/test" });
  await waitFor(() => ui.get('[aria-label="new.txt"]'));
  assert.equal(ui.get('[aria-label="notes.txt"]').getAttribute("aria-pressed"), "true");
});

test("an explicit Trash location is preserved while initial favorites are loading", async t => {
  let resolveLocations;
  const ui = await surface(t, "finder", req => req.method === "files.locations" ? new Promise(resolve => { resolveLocations = resolve; }) : req.method === "files.trash_list" ? { entries: [], truncated: false } : defaults(req));
  await waitFor(() => resolveLocations);
  ui.emit("finder.location", { path: "@trash" });
  await waitFor(() => ui.get('.trash-browser'));
  resolveLocations([{ name: "Home", path: "/home/test" }]);
  await new Promise(resolve => setTimeout(resolve, 30));
  assert.ok(ui.get('.trash-browser'));
  assert.ok(!ui.requests.some(r => r.method === "files.list"));
});

test("desktop keyboard opens item actions and Delete uses the desktop trash operation", async t => {
  const item = { id: "file:/home/test/Desktop/notes.txt", name: "notes.txt", kind: "file", target: "/home/test/Desktop/notes.txt", icon: null };
  const ui = await surface(t, "wallpaper", req => req.method === "desktop.get" ? { ...desktop, items: [item] } : req.method === "desktop.remove" ? desktop : defaults(req));
  await waitFor(() => ui.get('[aria-label="notes.txt"]'));
  const file = ui.get('[aria-label="notes.txt"]'); file.click(); file.focus();
  await waitFor(() => file.getAttribute("aria-pressed") === "true");
  file.dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "F10", shiftKey: true, bubbles: true, cancelable: true }));
  await waitFor(() => ui.get('.desktop-context'));
  assert.ok(ui.get('.desktop-context').textContent.includes("Move to Trash"));
  ui.get('.desktop-context button').dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await waitFor(() => !ui.get('.desktop-context'));
  file.dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "Delete", bubbles: true, cancelable: true }));
  await waitFor(() => ui.requests.some(r => r.method === "desktop.remove"));
  assert.deepEqual(ui.requests.find(r => r.method === "desktop.remove").params, { id: item.id });
  assert.ok(!ui.requests.some(r => ["apps.uninstall", "files.trash"].includes(r.method)));
});


test("Terminal clipboard shortcuts cancel native paste and write once while Ctrl+C still interrupts", async t => {
  const ui = await surface(t, "terminal", req => {
    if (req.method === "terminal.start") return { id: 93, shell: "/bin/bash" };
    if (req.method === "terminal.read") return { data: [], exited: false };
    if (req.method === "terminal.paste") return "echo once";
    return defaults(req);
  });
  await waitFor(() => ui.requests.some(r => r.method === "shell.hello"));
  ui.emit("terminal.opened");
  await waitFor(() => ui.get(".xterm-helper-textarea"));
  const input = ui.get(".xterm-helper-textarea");
  for (const shiftKey of [false, true]) {
    const before = ui.requests.filter(r => r.method === "terminal.write").length;
    const event = new ui.window.KeyboardEvent("keydown", { key: "v", code: "KeyV", keyCode: 86, ctrlKey: true, shiftKey, bubbles: true, cancelable: true });
    input.dispatchEvent(event);
    assert.equal(event.defaultPrevented, true);
    input.dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "v", code: "KeyV", keyCode: 86, ctrlKey: true, shiftKey, repeat: true, bubbles: true, cancelable: true }));
    input.dispatchEvent(new ui.window.KeyboardEvent("keyup", { key: "v", ctrlKey: true, shiftKey, bubbles: true, cancelable: true }));
    await waitFor(() => ui.requests.filter(r => r.method === "terminal.write").length > before);
    await new Promise(resolve => setTimeout(resolve, 25));
    assert.deepEqual(ui.requests.filter(r => r.method === "terminal.write").slice(before).map(r => r.params.data), ["echo once"]);
  }
  assert.equal(ui.requests.filter(r => r.method === "terminal.paste").length, 2);
  input.dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "c", code: "KeyC", keyCode: 67, ctrlKey: true, bubbles: true, cancelable: true }));
  await waitFor(() => ui.requests.some(r => r.method === "terminal.write" && r.params.data === "\x03"));
  assert.ok(!ui.requests.some(r => r.method === "terminal.copy"));
});


test("Orbit reports transfer progress, cancels once, and recovers after a failed transfer", async t => {
  let cancelled = false;
  const ui = await surface(t, "finder", req => {
    if (req.method === "files.transfer_start" || req.method === "files.transfer_status") return { id: 17, bytes: 65536, total: 1048576, entries: 2, done: cancelled, error: cancelled ? "Transfer cancelled. The original data was preserved." : null };
    if (req.method === "files.transfer_cancel") { cancelled = true; return null; }
    return finderReply(req);
  });
  await waitFor(() => ui.get('[aria-label="notes.txt"]'));
  ui.get('[aria-label="notes.txt"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get(".finder-context")); menu(ui, ".finder-context", "Copy To…");
  await waitFor(() => ui.get('[aria-label="Destination folder"]'));
  type(ui, '[aria-label="Destination folder"]', "/home/test/Projects");
  ui.get('form[role="dialog"] button[type="submit"]').click();
  await waitFor(() => ui.get('[aria-label="File transfer"]'));
  assert.equal(ui.get('progress').value, 65536);
  assert.equal(ui.get('progress').max, 1048576);
  const cancel = ui.get('[aria-label="File transfer"] button');
  cancel.click(); cancel.click();
  await waitFor(() => ui.requests.some(req => req.method === "files.transfer_cancel"));
  await waitFor(() => ui.get('.finder-notice')?.textContent.includes("original data was preserved"));
  assert.equal(ui.requests.filter(req => req.method === "files.transfer_cancel").length, 1);
  assert.equal(ui.get('[aria-label="File transfer"]'), null);
  assert.equal(ui.get('form[role="dialog"]'), null);
  assert.ok(ui.get('[aria-label="notes.txt"]'));
});

test("Orbit reconnects to an active transfer after its page reloads", async t => {
  let complete = false;
  const active = { id: 21, bytes: 10, total: 100, entries: 1, done: false, error: null };
  const ui = await surface(t, "finder", req => req.method === "files.transfers" ? [active] : req.method === "files.transfer_status" ? { ...active, bytes: complete ? 100 : 10, done: complete } : finderReply(req));
  await waitFor(() => ui.get('[aria-label="File transfer"]'));
  assert.ok(!ui.requests.some(req => req.method === "files.transfer_start"));
  complete = true;
  await waitFor(() => ui.get('.finder-notice')?.textContent.includes("Transfer complete"));
  assert.equal(ui.get('[aria-label="File transfer"]'), null);
});

test("Orbit tabs retain independent navigation histories and keyboard switching", async t => {
  const ui = await surface(t, "finder", finderReply);
  await waitFor(() => ui.get('[aria-label="Projects"]'));
  ui.get('[aria-label="Projects"]').dispatchEvent(new ui.window.MouseEvent('dblclick', { bubbles: true }));
  await waitFor(() => ui.get('h1')?.textContent === 'Projects');
  ui.get('[aria-label="New folder tab"]').click();
  await waitFor(() => ui.window.document.querySelectorAll('[role="tab"]').length === 2);
  ui.get('nav[aria-label="Favorites"] button[title="Home"]').click();
  await waitFor(() => ui.get('[aria-label="notes.txt"]'));
  ui.get('.finder-app').dispatchEvent(new ui.window.KeyboardEvent('keydown', { key: 'PageUp', ctrlKey: true, bubbles: true, cancelable: true }));
  await waitFor(() => ui.get('h1')?.textContent === 'Projects');
  assert.equal(ui.get('[aria-label="Go back"]').disabled, false);
  ui.get('[aria-label="Go back"]').click();
  await waitFor(() => ui.get('[aria-label="notes.txt"]'));
  ui.get('.finder-app').dispatchEvent(new ui.window.KeyboardEvent('keydown', { key: 'w', ctrlKey: true, bubbles: true, cancelable: true }));
  await waitFor(() => ui.window.document.querySelectorAll('[role="tab"]').length === 1);
});

test("Orbit preview renders markup as text and returns keyboard focus on close", async t => {
  const ui = await surface(t, 'finder', req => req.method === 'files.preview'
    ? { kind: 'text', content: '<script>unsafe()</script>', truncated: true } : finderReply(req));
  await waitFor(() => ui.get('[aria-label="notes.txt"]'));
  const item = ui.get('[aria-label="notes.txt"]'); item.focus(); item.click();
  await waitFor(() => item.getAttribute('aria-pressed') === 'true');
  item.dispatchEvent(new ui.window.KeyboardEvent('keydown', { key: ' ', bubbles: true, cancelable: true }));
  await waitFor(() => ui.get('[aria-label="File contents"]'));
  assert.equal(ui.get('[aria-label="File contents"]').textContent, '<script>unsafe()</script>');
  assert.equal(ui.get('.file-preview script'), null);
  assert.ok(ui.get('.file-preview').textContent.includes('64 KiB'));
  ui.get('[aria-label="Close preview"]').dispatchEvent(new ui.window.KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
  await waitFor(() => !ui.get('.file-preview'));
  assert.equal(ui.window.document.activeElement, item);
});

test("Settings persists launcher providers and opens typed settings search targets", async t => {
  const ui = await surface(t, 'settings', req => ['settings.search','settings.search_set'].includes(req.method)
    ? req.params ?? { settings: true, calculator: false } : defaults(req));
  ui.emit('preferences.page', { page: 'search' });
  await waitFor(() => ui.get('[aria-label="Search calculator"]')?.disabled === false);
  await waitFor(() => ui.window.glisseSwitchReady(ui.get('[aria-label="Search calculator"]')));
  // Glisse updates its controlled callback in a passive effect after loading.
  await new Promise(resolve => ui.window.requestAnimationFrame(() => ui.window.requestAnimationFrame(resolve)));
  ui.get('[aria-label="Search calculator"]').click();
  await waitFor(() => ui.requests.some(req => req.method === 'settings.search_set'));
  assert.deepEqual(ui.requests.find(req => req.method === 'settings.search_set').params, { settings: true, calculator: true });
  ui.emit('preferences.page', { page: 'bluetooth' });
  await waitFor(() => ui.get('#settings-title')?.textContent === 'Bluetooth');
});

test("non-application launcher results activate without app management actions", async t => {
  const ui = await surface(t, 'applications', req => req.method === 'search.query'
    ? { serial: req.params.serial, sections: [{ provider: 'settings', title: 'Settings', items: [{ id: 'wifi', title: 'Wi-Fi settings', subtitle: 'System settings', icon: 'preferences-system' }] }] } : defaults(req));
  ui.emit('popover.shown'); await waitFor(() => ui.get('.row'));
  ui.get('.row').dispatchEvent(new ui.window.MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
  assert.equal(ui.get('.app-context'), null);
  assert.equal(ui.get('.row').draggable, false);
  ui.get('.row').click();
  await waitFor(() => ui.requests.some(req => req.method === 'search.activate'));
  assert.deepEqual(ui.requests.find(req => req.method === 'search.activate').params, { provider: 'settings', item_id: 'wifi' });
});

test("Control Center DND preserves newer events without reading notification history", async t => {
  let finish;
  const ui = await surface(t, 'options', req => req.method === 'notifications.do_not_disturb'
    ? new Promise(resolve => { finish = resolve; }) : req.method === 'notifications.get'
    ? { available: true, do_not_disturb: false, items: [] } : defaults(req));
  ui.emit('popover.shown');
  await waitFor(() => !ui.get('[aria-label="Do Not Disturb"]').disabled);
  ui.get('[aria-label="Do Not Disturb"]').click();
  await waitFor(() => finish);
  ui.emit('notifications.changed', { available: true, do_not_disturb: true, items: [] });
  finish({ available: true, do_not_disturb: false, items: [] });
  await waitFor(() => !ui.get('[aria-label="Do Not Disturb"]').disabled);
  assert.equal(ui.get('[aria-label="Do Not Disturb"]').getAttribute('aria-checked'), 'true');
  assert.equal(ui.requests.some(req => req.method === 'notifications.read'), false);
});

test("Notifications groups messages by app and preserves unread until explicitly marked", async t => {
  const item = { id: 1, app: 'Mail', summary: 'Message', body: '', time: Date.now(), unread: false, actions: [] };
  const ui = await surface(t, 'notifications', req => req.method === 'notifications.get'
    ? { available: true, do_not_disturb: false, items: [item] } : defaults(req));
  ui.emit('popover.shown'); await waitFor(() => ui.get('.notification'));
  ui.emit('notifications.changed', { available: true, do_not_disturb: true, items: [item, { ...item, id: 2, app: 'Calendar', summary: 'Meeting' }] });
  await waitFor(() => ui.window.document.querySelectorAll('.notification-group').length === 2);
  assert.ok(ui.get('[aria-label="Do Not Disturb"]'));
  assert.equal(ui.requests.some(r => r.method === 'notifications.read'), false);
  assert.ok(ui.get('.notification-list').textContent.includes('Meeting'));
});

test("Orbit leaves sidebar keyboard activation available when a file is selected", async t => {
  const ui = await surface(t, 'finder', finderReply);
  await waitFor(() => ui.get('[aria-label="notes.txt"]'));
  ui.get('[aria-label="notes.txt"]').click();
  await waitFor(() => ui.get('[aria-label="notes.txt"]').getAttribute('aria-pressed') === 'true');
  const button = ui.get('nav[aria-label="Favorites"] button[title="Applications"]'); button.focus();
  const event = new ui.window.KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true });
  button.dispatchEvent(event);
  assert.equal(event.defaultPrevented, false);
  assert.equal(ui.requests.some(request => request.method === 'files.open'), false);
});

test("dock groups show a window count and cycle through an application's windows", async t => {
  const windows = [{ id: 1, name: 'Firefox', title: 'First window', active: true }, { id: 2, name: 'Firefox', title: 'Second window', active: false }];
  const ui = await surface(t, 'panel', req => req.method === 'windows.list' ? windows : defaults(req));
  await waitFor(() => ui.get('[aria-label="Firefox"] .dock-window-count'));
  assert.equal(ui.get('.dock-window-count').textContent, '2');
  ui.get('[aria-label="Firefox"]').click();
  await waitFor(() => ui.requests.some(req => req.method === 'windows.activate'));
  assert.deepEqual(ui.requests.find(req => req.method === 'windows.activate').params, { id: 2 });
  ui.emit('windows.changed', windows.map(window => ({ ...window, active: window.id === 2 })));
  await waitFor(() => ui.get('[aria-label="Firefox"]').getAttribute('aria-pressed') === 'true');
  await new Promise(resolve => setTimeout(resolve, 10));
  ui.get('[aria-label="Firefox"]').click();
  await waitFor(() => ui.requests.filter(req => req.method === 'windows.activate').length === 2);
  assert.equal(ui.requests.filter(req => req.method === 'windows.activate')[1].params.id, 1);
});

test("primary display selection persists and stale brightness replies cannot affect another monitor", async t => {
  const mode = { width: 1920, height: 1080, refresh: 60000, preferred: true };
  const outputs = ['HDMI-A-1', 'DP-1'].map(name => ({ name, description: name, enabled: true, x: 0, y: 0, scale: 1, transform: 0, current: mode, modes: [mode] }));
  let first;
  const ui = await surface(t, 'settings', req => req.method === 'settings.displays' ? outputs
    : req.method === 'settings.primary_display' ? { connector: null, effective: 'HDMI-A-1' }
    : req.method === 'settings.primary_display_set' ? { connector: req.params.connector, effective: req.params.connector }
    : req.method === 'settings.monitor_brightness' ? req.params.connector === 'HDMI-A-1'
      ? new Promise(resolve => { first = resolve; }) : { percent: 80 } : defaults(req));
  ui.emit('preferences.page', { page: 'displays' });
  await waitFor(() => first && ui.get('#primary-display')?.disabled === false);
  ui.get('#primary-display').value = 'DP-1';
  ui.get('#primary-display').dispatchEvent(new ui.window.Event('change', { bubbles: true }));
  await waitFor(() => ui.requests.some(req => req.method === 'settings.primary_display_set'));
  assert.deepEqual(ui.requests.find(req => req.method === 'settings.primary_display_set').params, { connector: 'DP-1' });
  [...ui.window.document.querySelectorAll('.display-list-row')].find(button => button.textContent.includes('DP-1')).click();
  await waitFor(() => ui.get('#monitor-brightness')?.value === '80');
  first({ percent: 10 });
  await new Promise(resolve => setTimeout(resolve, 25));
  assert.equal(ui.get('#monitor-brightness').value, '80');
  assert.equal(ui.get('#monitor-brightness').disabled, false);
});

test("Calendar uses one date tab stop and moves focus across month boundaries", async t => {
  const ui = await surface(t, "utilities", req => req.method === "shell.hello" ? { surface: "calendar", capabilities: [] } : req.method === "calendar.list" ? [] : null);
  await waitFor(() => ui.get('.month button[tabindex="0"]'));
  assert.equal(ui.window.document.querySelectorAll('.month button[tabindex="0"]').length, 1);
  const initial = ui.get('.month button[tabindex="0"]');
  const date = new Date(initial.dataset.date + 'T12:00:00');
  const next = new Date(date); next.setDate(date.getDate() + 7);
  initial.focus();
  initial.dispatchEvent(new ui.window.KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }));
  const dateKey = d => `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
  await waitFor(() => ui.window.document.activeElement?.dataset.date === dateKey(next));
  for (let i = 0; i < 5; i++) {
    next.setDate(next.getDate() + 7);
    ui.window.document.activeElement.dispatchEvent(new ui.window.KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }));
    await waitFor(() => ui.window.document.activeElement?.dataset.date === dateKey(next));
  }
  assert.equal(ui.window.document.querySelectorAll('.month button[tabindex="0"]').length, 1);
  assert.equal(ui.get('.month button[aria-pressed="true"]').dataset.date, dateKey(next));
  assert.ok(!ui.requests.some(r => r.method === 'calendar.save'));
});

test("Notifications expands long messages and prevents duplicate action invocation", async t => {
  const item = { id: 10, app: 'Mail', summary: 'Long message', body: 'A long body. '.repeat(60), time: Date.now(), unread: true, actions: [['open', 'Open']] };
  let finish;
  const ui = await surface(t, 'notifications', req => req.method === 'notifications.get'
    ? { available: true, do_not_disturb: false, items: [item] }
    : req.method === 'notifications.action' ? new Promise(resolve => { finish = resolve; }) : defaults(req));
  ui.emit('popover.shown');
  await waitFor(() => ui.get('.notification'));
  assert.equal(ui.requests.some(r => r.method === 'notifications.read'), false);
  const expand = ui.get('[aria-controls="notification-body-10"]'); expand.click();
  await waitFor(() => expand.getAttribute('aria-expanded') === 'true');
  const action = ui.get('.notification-actions button'); action.click(); action.click();
  await waitFor(() => finish);
  assert.equal(ui.requests.filter(r => r.method === 'notifications.action').length, 1);
  finish({ available: true, do_not_disturb: false, items: [] });
  await waitFor(() => ui.get('.notification-empty'));
});

test("shared hover preferences follow host events across Notification Center", async t => {
  const ui = await surface(t, 'notifications', req => req.method === 'desktop.get' ? { ...desktop, hover_background: 'blue' } : defaults(req));
  await waitFor(() => ui.window.document.documentElement.dataset.hoverBackground === 'blue');
  ui.emit('desktop.changed', { ...desktop, hover_background: 'green' });
  await waitFor(() => ui.window.document.documentElement.dataset.hoverBackground === 'green');
  ui.emit('desktop.changed', { ...desktop, hover_background: 'auto' });
  await waitFor(() => ui.window.document.documentElement.dataset.hoverBackground === 'auto');
});

test("failed display rollback retains the dialog and permits a safe retry", async t => {
  const mode = { width: 1280, height: 720, refresh: 60000, preferred: true };
  let attempts = 0;
  const ui = await surface(t, 'settings', r => r.method === 'settings.displays'
    ? [{ name: 'Virtual-1', description: 'Virtual display', enabled: true, x: 0, y: 0, scale: 1, transform: 0, current: mode, modes: [mode] }]
    : r.method === 'settings.display_apply' ? { token: 'retry-trial', seconds: 15 }
    : r.method === 'settings.display_decide' && ++attempts === 1 ? { error: { code: 'unavailable', message: 'Compositor disconnected. Retry Revert.' } } : defaults(r));
  ui.get('nav [aria-label="Displays"]').click();
  await waitFor(() => ui.get('#resolution'));
  [...ui.window.document.querySelectorAll('button')].find(b => b.textContent === 'Apply changes').click();
  await waitFor(() => ui.get('[aria-label="Keep display settings"]'));
  const revert = () => [...ui.get('[aria-label="Keep display settings"]').querySelectorAll('button')].find(b => b.textContent === 'Revert');
  revert().click();
  await waitFor(() => ui.get('[role="alert"]')?.textContent.includes('Compositor disconnected'));
  assert.ok(ui.get('[aria-label="Keep display settings"]'));
  await waitFor(() => !revert().disabled);
  revert().click();
  await waitFor(() => !ui.get('[aria-label="Keep display settings"]'));
  assert.equal(attempts, 2);
});

test("Meridian Files opens a Network page and browses mounted shares without launching Nautilus", async t => {
  const mounts = [
    { name: "Home", path: "/home/test" },
    { name: "Network · fileserver/share", path: "/mnt/team" },
  ];
  const ui = await surface(t, "finder", request => request.method === "files.locations" ? mounts
    : request.method === "files.list" ? { path: request.params.path, parent: "/mnt", entries: [], truncated: false }
    : defaults(request));
  await waitFor(() => ui.get('.finder-sidebar button[title="Network"]'));
  ui.get('.finder-sidebar button[title="Network"]').click();
  await waitFor(() => ui.get('.finder-network-row'));
  assert.match(ui.get('.finder-network-row').textContent, /fileserver\/share/);
  assert.equal(ui.requests.some(r => r.method === "files.list" && r.params.path === "@network"), false);
  ui.get('.finder-network-row').click();
  await waitFor(() => ui.requests.some(r => r.method === "files.list" && r.params.path === "/mnt/team"));
});

test("Settings lists default application purposes and saves selected installed handlers", async t => {
  const initial = [
    { purpose: "files", app_id: "meridian-finder.desktop" },
    { purpose: "browser", app_id: null },
    { purpose: "terminal", app_id: "kitty.desktop" },
    { purpose: "mail", app_id: null },
    { purpose: "pdf", app_id: null },
    { purpose: "images", app_id: null },
    { purpose: "text", app_id: null },
  ];
  const ui = await surface(t, "settings", request => {
    if (request.method === "settings.default_apps") return initial;
    if (request.method === "settings.default_app_set") return initial.map(value => value.purpose === request.params.purpose ? { ...value, app_id: request.params.app_id } : value);
    return defaults(request);
  });
  [...ui.window.document.querySelectorAll('.settings-sidebar nav button')].find(button => button.textContent === "Applications").click();
  await waitFor(() => ui.get('[aria-label="Default app for Web browser"]'));
  assert.equal(ui.window.document.querySelectorAll(".default-apps-row").length, 7);
  assert.equal(ui.get('[aria-label="Default app for Files"]').value, "meridian-finder.desktop");
  const select = ui.get('[aria-label="Default app for Web browser"]');
  const setter = Object.getOwnPropertyDescriptor(ui.window.HTMLSelectElement.prototype, "value").set;
  setter.call(select, "firefox.desktop");
  select.dispatchEvent(new ui.window.Event("change", { bubbles: true }));
  await waitFor(() => ui.requests.some(request => request.method === "settings.default_app_set"));
  assert.deepEqual(ui.requests.find(request => request.method === "settings.default_app_set").params,
    { purpose: "browser", app_id: "firefox.desktop" });
  await waitFor(() => ui.get('[aria-label="Default app for Web browser"]').value === "firefox.desktop");
});
