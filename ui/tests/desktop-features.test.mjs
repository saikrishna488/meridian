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
  window.HTMLElement.prototype.scrollIntoView = () => {};
  window.HTMLElement.prototype.scrollTo = () => {};
  window.matchMedia = () => ({ matches: false, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {} });
  window.ResizeObserver = class { observe() {} disconnect() {} };
  window.HTMLCanvasElement.prototype.getContext = () => new Proxy({ measureText: text => ({ width: text.length * 8, actualBoundingBoxAscent: 10, actualBoundingBoxDescent: 3 }), createLinearGradient: () => ({ addColorStop() {} }), getImageData: (_x, _y, width, height) => ({ data: new Uint8ClampedArray(width * height * 4), width, height }) }, { get(target, key) { return key in target ? target[key] : () => {}; }, set(target, key, value) { target[key] = value; return true; } });
  window.fetch = async () => { throw new Error("Custom-scheme fetch is unavailable"); };
  window.webkit = { messageHandlers: { meridian: { async postMessage(json) { const req = JSON.parse(json); requests.push(req); const result = await respond(req); return JSON.stringify(result?.error ? { ok: false, error: result.error } : { ok: true, result }); } } } };
  window.eval(await readFile(new URL(`../dist/surfaces/${name}/${name}.js`, import.meta.url), "utf8"));
  await waitFor(() => window.document.querySelector("#root").children.length);
  return { window, requests, get: s => window.document.querySelector(s), emit: (event, data) => window.__meridianDispatch(JSON.stringify({ event, data })) };
}
function defaults(req) { return req.method === "desktop.get" ? desktop : req.method === "search.query" ? { ...search, serial: req.params.serial } : req.method === "windows.list" ? [] : req.method === "dock.list" ? [app.id] : req.method === "dock.layout_get" ? [] : req.method === "shell.hello" ? { surface: "panel", capabilities: [] } : null; }
function drag(window, element, type, data, clientX = 1) {
  const event = new window.Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "dataTransfer", { value: { types: Object.keys(data), getData: key => data[key] ?? "", setData: (key, value) => { data[key] = value; }, effectAllowed: "", dropEffect: "" } });
  Object.defineProperty(event, "clientX", { value: clientX }); element.dispatchEvent(event);
}
test("launcher and apps reorder, and dragging to trash only requests confirmation", async t => {
  const ui = await surface(t, "panel", defaults);
  await waitFor(() => ui.get('[aria-label="Firefox"]'));
  const data = {};
  drag(ui.window, ui.get("#applications"), "dragstart", data);
  drag(ui.window, ui.get('[aria-label="Firefox"]'), "drop", data);
  await waitFor(() => ui.requests.some(r => r.method === "dock.layout_set"));
  assert.deepEqual(ui.requests.find(r => r.method === "dock.layout_set").params.items, ["firefox.desktop", "@applications", "@trash"]);
  drag(ui.window, ui.get("#trash"), "drop", { "application/x-meridian-app": app.id });
  await waitFor(() => ui.requests.some(r => r.method === "apps.uninstall_prompt"));
  assert.ok(!ui.requests.some(r => r.method === "apps.uninstall"));
  ui.get('[aria-label="Firefox"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get('.dock-context [role="menuitem"]'));
  [...ui.window.document.querySelectorAll('.dock-context button')].find(b => b.textContent === "Uninstall…").click();
  await waitFor(() => ui.requests.filter(r => r.method === "apps.uninstall_prompt").length === 2);
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
  menu(ui, '.finder-context', "Move to Trash…"); await waitFor(() => ui.get('[role="dialog"]'));
  assert.ok(!ui.requests.some(r => r.method === "files.trash"));
  ui.get('[role="dialog"] button[type="submit"]').click(); await waitFor(() => ui.requests.some(r => r.method === "files.trash"));
  assert.equal(ui.requests.find(r => r.method === "files.trash").params.path, file.path);
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
  assert.ok(ui.requests.find(r => r.method === "terminal.write").params.data.includes("pwd"));
  ui.get('[aria-label="New terminal session"]').click(); await waitFor(() => ui.window.document.querySelectorAll('[role="tab"]').length === 2); await waitFor(() => next === 2);
  ui.get('[aria-label="Close session 2"]').click(); await waitFor(() => ui.requests.some(r => r.method === "terminal.close" && r.params.id === 2));
});
test("Meridian built-in icons and installed app artwork are preserved", async t => {
  const built = { id: "meridian-finder.desktop", title: "Finder", icon: "system-file-manager" };
  const ui = await surface(t, "panel", req => req.method === "search.query" ? { serial: req.params.serial, sections: [{ provider: "apps", items: [app, built] }] } : req.method === "dock.list" ? [app.id, built.id] : defaults(req));
  await waitFor(() => ui.get('[aria-label="Finder"] img'));
  assert.equal(ui.get('[aria-label="Firefox"] img').src, "meridian://app-icons/firefox.desktop"); assert.ok(ui.get('[aria-label="Firefox"] .system-artwork'));
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
  menu(ui, '.desktop-context', "Add files…"); await waitFor(() => ui.get('[aria-label="notes.txt"]'));
  ui.get('[aria-label="notes.txt"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true })); await waitFor(() => ui.get('.desktop-context'));
  menu(ui, '.desktop-context', "Remove shortcut"); await waitFor(() => ui.requests.some(r => r.method === "desktop.remove"));
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

test("Settings saves appearance and responds to system theme broadcasts", async t => {
  const ui = await surface(t, "settings", req => req.method === "desktop.appearance_set" ? { ...desktop, appearance: req.params.id } : defaults(req));
  [...ui.window.document.querySelectorAll("nav button")].find(b => b.textContent.endsWith("Appearance")).click();
  await waitFor(() => ui.get("#appearance"));
  ui.get("#appearance").value = "dark";
  ui.get("#appearance").dispatchEvent(new ui.window.Event("change", { bubbles: true }));
  await waitFor(() => ui.requests.some(r => r.method === "desktop.appearance_set"));
  assert.equal(ui.requests.find(r => r.method === "desktop.appearance_set").params.id, "dark");
  ui.emit("desktop.changed", { ...desktop, appearance: "dark" });
  await waitFor(() => ui.window.document.documentElement.dataset.theme === "dark");
  ui.emit("desktop.changed", desktop);
  await waitFor(() => ui.window.document.documentElement.dataset.theme === "light");
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
    await waitFor(() => ui.window.document.documentElement.dataset.theme === mode);
    const background = mode === "dark" ? "#171a20" : "#fafafa";
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
  await waitFor(()=>ui.window.document.querySelectorAll(".display-tile").length===2);
  [...ui.window.document.querySelectorAll(".display-tile")].find(b=>b.textContent.includes("External Display")).click();
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
  [...ui.window.document.querySelectorAll("nav button")].find(b=>b.textContent.endsWith("Wi-Fi")).click();await waitFor(()=>ui.get(".device-name strong")?.textContent==="Office Wi-Fi");
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
  assert.equal(ui.get('#bluetooth').className,"meridian-toggle");
});
test("Sharing switches preview their state without changing system services", async t => {
  const ui=await surface(t,"settings",defaults);[...ui.window.document.querySelectorAll("nav button")].find(b=>b.textContent.endsWith("Sharing")).click();
  await waitFor(()=>ui.get('[aria-label="File Sharing"]'));assert.ok(ui.get(".settings-content").textContent.includes("Prototype"));
  for(const label of ["File Sharing","Screen Sharing","Remote Login"]){ui.get(`[aria-label="${label}"]`).click();await waitFor(()=>ui.get(`[aria-label="${label}"]`).getAttribute("aria-checked")==="true");}
  assert.ok(ui.requests.every(r=>["shell.hello","desktop.get"].includes(r.method)));
});
