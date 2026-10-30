import { installBrowserMocks } from "./browser-mocks.mjs";
import { surfaceScript } from "./bundle.mjs";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { JSDOM } from "jsdom";

async function waitFor(predicate, message = "UI did not update") {
  for (let attempt = 0; attempt < 150; attempt++) {
    if (predicate()) return;
    await new Promise(resolve => setTimeout(resolve, 10));
  }
  assert.fail(message);
}

async function surface(t, name, respond = () => null) {
  const html = await readFile(new URL(`../dist/surfaces/${name}/index.html`, import.meta.url), "utf8");
  const dom = new JSDOM(html, { url: `https://meridian.test/surfaces/${name}/`, runScripts: "outside-only", pretendToBeVisual: true });
  t.after(() => dom.window.close());
  const { window } = dom;
  installBrowserMocks(window);
  window.TextEncoder = TextEncoder;
  window.HTMLElement.prototype.scrollIntoView = function () {};
  window.HTMLElement.prototype.scrollTo = function () {};
  const requests = [];
  window.webkit = { messageHandlers: { meridian: { async postMessage(json) {
    const request = JSON.parse(json);
    requests.push(request);
    const response = await respond(request);
    return JSON.stringify(response?.error && typeof response.error === "object" ? { ok: false, error: response.error } : { ok: true, result: response });
  } } } };
  const js = await surfaceScript(name);
  window.eval(js);
  await waitFor(() => window.document.querySelector("#root")?.children.length);
  if (name !== "wallpaper") await waitFor(() => requests.some(request => request.method === "shell.hello"));
  return {
    window, requests,
    get: selector => window.document.querySelector(selector),
    emit: (event, data) => window.__meridianDispatch(JSON.stringify({ event, ...(data === undefined ? {} : { data }) })),
    key: key => window.dispatchEvent(new window.KeyboardEvent("keydown", { key, bubbles: true })),
    type(selector, value) {
      const input = window.document.querySelector(selector);
      const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
      setter.call(input, value);
      input.dispatchEvent(new window.Event("input", { bubbles: true }));
    },
  };
}

const searchResult = (serial, items) => ({ serial, sections: [{ provider: "apps", items }] });
const app = (id, title) => ({ id, title, subtitle: "Application" });

test("dock follows host windows and activates the selected window", async t => {
  const ui = await surface(t, "panel", request => request.method === "windows.list" ? [{ id: 7, name: "Terminal", title: "Shell", active: true }] : null);
  await waitFor(() => ui.get(".window"));
  assert.equal(ui.get(".window").getAttribute("aria-pressed"), "true");
  ui.get(".window").click();
  await waitFor(() => ui.requests.some(request => request.method === "windows.activate"));
  assert.deepEqual(ui.requests.find(request => request.method === "windows.activate").params, { id: 7 });
  ui.emit("windows.changed", []);
  await waitFor(() => !ui.get(".window"));
  ui.emit("popover.state", { applications: true, options: false });
  await waitFor(() => ui.get("#applications").getAttribute("aria-expanded") === "true");
  assert.equal(ui.get(".system-tray"), null);
  assert.ok(ui.get("#applications"));
});

test("dock previews running windows with keyboard selection and follows dock style changes", async t => {
  const windows = [{ id: 1, name: "Terminal", title: "First shell", active: true }, { id: 2, name: "Terminal", title: "Second shell", active: false }];
  const desktop = { appearance: "dark", dock_style: "floating", font: "Inter", wallpaper: "", wallpaper_uri: "", wallpapers: [], items: [] };
  const ui = await surface(t, "panel", request => request.method === "windows.list" ? windows : request.method === "desktop.get" ? desktop : null);
  await waitFor(() => ui.get('[aria-label="Terminal"]'));
  const button = ui.get('[aria-label="Terminal"]');
  button.focus();
  button.dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
  await waitFor(() => ui.get(".dock-preview-window"));
  await waitFor(() => ui.window.document.activeElement === ui.get(".dock-preview-window"));
  assert.equal(ui.window.document.querySelectorAll(".dock-preview-window").length, 2);
  ui.window.document.activeElement.dispatchEvent(new ui.window.KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
  assert.equal(ui.window.document.activeElement.textContent, "Second shell");
  ui.window.document.activeElement.click();
  await waitFor(() => ui.requests.some(r => r.method === "windows.activate" && r.params.id === 2));
  await waitFor(() => !ui.get(".dock-preview"));
  ui.emit("desktop.changed", { ...desktop, dock_style: "minimal" });
  await waitFor(() => ui.get(".bar.dock-minimal"));
  ui.emit("desktop.changed", { ...desktop, dock_style: "full-width" });
  await waitFor(() => ui.get(".bar.dock-full-width"));
  assert.ok(ui.requests.some(r => r.method === "dock.input_region"));
});

test("native panel leave clears hover and dismisses its tooltip", async t => {
  const ui = await surface(t, "panel");
  const button = ui.get("#applications");
  button.dispatchEvent(new ui.window.MouseEvent("mouseover", { bubbles: true }));
  await waitFor(() => ui.get(".dock-preview"));
  assert.ok(button.parentElement.classList.contains("dock-hovered"));
  ui.window.dispatchEvent(new ui.window.Event("meridian-panel-leave"));
  await waitFor(() => !ui.get(".dock-preview") && !button.parentElement.classList.contains("dock-hovered"));
});

test("overflowing dock responds to a vertical mouse wheel", async t => {
  const ui = await surface(t, "panel");
  const list = ui.get(".dock-items");
  Object.defineProperty(list, "scrollWidth", { value: 900 });
  Object.defineProperty(list, "clientWidth", { value: 300 });
  const event = new ui.window.WheelEvent("wheel", { bubbles: true, cancelable: true, deltaY: 120 });
  list.dispatchEvent(event);
  assert.equal(list.scrollLeft, 120);
  assert.equal(event.defaultPrevented, true);
});

test("applications preserve keyboard selection, launching, and dismissal", async t => {
  const ui = await surface(t, "applications", request => request.method === "search.query"
    ? searchResult(request.params.serial, [app("terminal", "Terminal"), app("browser", "Browser")]) : null);
  ui.emit("popover.shown");
  await waitFor(() => !ui.get("#menu").classList.contains("hidden"));
  await waitFor(() => ui.window.document.activeElement === ui.get("#search"));
  ui.key("ArrowDown");
  await waitFor(() => ui.get("#search").getAttribute("aria-activedescendant") === "row-0");
  ui.key("ArrowDown");
  await waitFor(() => ui.get("#row-1").getAttribute("aria-selected") === "true");
  ui.key("Enter");
  await waitFor(() => ui.requests.some(request => request.method === "popover.close"));
  assert.deepEqual(ui.requests.find(request => request.method === "search.activate").params, { provider: "apps", item_id: "browser" });
});

test("applications ignore stale search replies and Escape clears the query before closing", async t => {
  let resolveSlow;
  const ui = await surface(t, "applications", request => {
    if (request.method !== "search.query") return null;
    if (request.params.query === "slow") return new Promise(resolve => { resolveSlow = () => resolve(searchResult(request.params.serial, [app("old", "Old result")])); });
    return searchResult(request.params.serial, [app("new", request.params.query || "All apps")]);
  });
  ui.emit("popover.shown");
  await waitFor(() => !ui.get("#menu").classList.contains("hidden"));
  ui.type("#search", "slow");
  await waitFor(() => resolveSlow);
  ui.type("#search", "fast");
  await waitFor(() => ui.get(".name")?.textContent === "fast");
  resolveSlow();
  await new Promise(resolve => setTimeout(resolve, 30));
  assert.equal(ui.get(".name").textContent, "fast");
  ui.key("Escape");
  await waitFor(() => ui.get("#search").value === "");
  assert.equal(ui.requests.some(request => request.method === "popover.close"), false);
  ui.key("Escape");
  await waitFor(() => ui.requests.some(request => request.method === "popover.close"));
});

test("Control Center displays availability and updates settings without duplicate session shortcuts", async t => {
  const state = { wifi: true, bluetooth: null, brightness: 65, volume: 30 };
  const ui = await surface(t, "options", request => request.method === "settings.get" ? state : null);
  ui.emit("popover.shown");
  await waitFor(() => !ui.get("#menu").classList.contains("hidden"));
  assert.equal(ui.get("#wifi").getAttribute("aria-checked"), "true");
  assert.equal(ui.get("#bluetooth").disabled, true);
  await waitFor(() => ui.get("#brightness-value").textContent === "65%");
  assert.equal(ui.get("#brightness-value").textContent, "65%");
  ui.get("#wifi").click();
  await waitFor(() => ui.requests.some(request => request.method === "settings.set_wifi"));
  assert.deepEqual(ui.requests.find(request => request.method === "settings.set_wifi").params, { enabled: false });
  ui.emit("settings.changed", { ...state, volume: 80 });
  await waitFor(() => ui.get("#volume-value").textContent === "80%");
  ui.type("#volume", "42");
  await waitFor(() => ui.requests.some(request => request.method === "settings.set_volume"));
  assert.deepEqual(ui.requests.find(request => request.method === "settings.set_volume").params, { percent: 42 });
  assert.equal(ui.get(".control-footer"), null);
  for (const id of ["lock", "sleep", "restart", "shutdown", "sign-out"]) assert.equal(ui.get(`#${id}`), null);
  assert.equal(ui.get("#menu").textContent.includes("Personalize"), false);
  assert.equal(ui.requests.some(request => request.method === "session.sign_out"), false);
});

test("greeter restores user/session, clears passwords, and recovers from failed login", async t => {
  const ui = await surface(t, "greeter", request => {
    if (request.method === "greeter.info") return { hostname: "Meridian", sessions: [{ id: "desktop", name: "Desktop" }], last_user: "alex", last_session: "desktop" };
    if (request.method === "greeter.login") return { error: { code: "failed", message: "Wrong password" } };
    return null;
  });
  await waitFor(() => !ui.get("#password").disabled);
  assert.equal(ui.get("#username").value, "alex");
  assert.equal(ui.get("#session").value, "desktop");
  ui.type("#password", "secret");
  ui.get("#form").dispatchEvent(new ui.window.Event("submit", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get("#message").textContent === "Wrong password");
  assert.equal(ui.get("#password").value, "");
  assert.equal(ui.get("#submit").disabled, false);
  await waitFor(() => ui.window.document.activeElement === ui.get("#password"));
  assert.deepEqual(ui.requests.find(request => request.method === "greeter.login").params, { username: "alex", password: "secret", session: "desktop" });
  ui.get("#reboot").click();
  await waitFor(() => ui.get("#reboot").textContent === "Restart?");
  assert.equal(ui.requests.some(request => request.method === "greeter.power"), false);
});

test("locker clears passwords and restores focus after failed authentication", async t => {
  const ui = await surface(t, "locker", request => request.method === "locker.unlock" ? { error: { code: "failed", message: "Try again" } } : null);
  ui.type("#password", "secret");
  ui.get("#form").dispatchEvent(new ui.window.Event("submit", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get("#message").textContent === "Try again");
  assert.equal(ui.get("#password").value, "");
  assert.equal(ui.get("#submit").disabled, false);
  await waitFor(() => ui.window.document.activeElement === ui.get("#password"));
});

test("wallpaper initializes desktop state without file-management requests", async t => {
  const ui = await surface(t, "wallpaper");
  assert.equal(ui.get(".wallpaper").getAttribute("aria-hidden"), "true");
  await waitFor(() => ui.requests.some(r => r.method === "shell.hello"));
  assert.ok(ui.requests.every(r => ["shell.hello", "desktop.get"].includes(r.method)));
});


test("top bar hosts Meridian and Control Center while the dock hosts applications", async t => {
  const dock = await surface(t, "panel");
  assert.ok(dock.get(".taskbar > .bar"));
  assert.equal(dock.get(".menu-brand"), null);
  assert.equal(dock.get("#options"), null);
  const ui = await surface(t, "topbar");
  assert.ok(ui.get(".topbar > .menu-brand"));
  assert.equal(ui.get(".bar"), null);
  ui.get(".system-tray .tray-indicator").click();
  await waitFor(() => ui.requests.some(r => r.method === "popover.toggle"));
  assert.deepEqual(ui.requests.at(-1), { method: "popover.toggle", params: { popover: "options" } });
  ui.emit("popover.state", { options: true, applications: false });
  await waitFor(() => ui.get(".system-tray .tray-indicator").getAttribute("aria-expanded") === "true");
});

test("dock groups app windows and keeps pinned apps after their windows close", async t => {
  const ui = await surface(t, "panel", request => {
    if (request.method === "dock.list") return ["terminal.desktop"];
    if (request.method === "search.query") return searchResult(0, [app("terminal.desktop", "Terminal")]);
    if (request.method === "windows.list") return [{ id: 1, name: "Terminal", active: false }, { id: 2, name: "Terminal", active: true }];
    return null;
  });
  await waitFor(() => ui.get(".running"));
  assert.equal(ui.window.document.querySelectorAll(".window").length, 1);
  ui.get(".window").click();
  await waitFor(() => ui.requests.some(r => r.method === "windows.activate"));
  assert.deepEqual(ui.requests.find(r => r.method === "windows.activate").params, { id: 1 });
  ui.emit("windows.changed", []);
  await waitFor(() => !ui.get(".running"));
  assert.ok(ui.get(".window"));
  ui.get(".window").click();
  await waitFor(() => ui.requests.some(r => r.method === "search.activate"));
  assert.deepEqual(ui.requests.find(r => r.method === "search.activate").params, { provider: "apps", item_id: "terminal.desktop" });
});

test("dragging an application to the taskbar pins its desktop id", async t => {
  const ui = await surface(t, "panel", request => request.method === "dock.list" ? ["finder.desktop"] : request.method === "search.query" ? searchResult(request.params.serial, [app("terminal.desktop", "Terminal"), app("finder.desktop", "Finder")]) : null);
  await waitFor(() => ui.get('[aria-label="Finder"]'));
  const data = new Map();
  const transfer = { types: ["application/x-meridian-app"], setData: (key, value) => data.set(key, value), getData: key => data.get(key) };
  function drag(target, type) {
    const event = new ui.window.Event(type, { bubbles: true, cancelable: true });
    Object.defineProperty(event, "dataTransfer", { value: transfer });
    target.dispatchEvent(event);
  }
  data.set("application/x-meridian-app", "terminal.desktop");
  drag(ui.get(".bar"), "dragover");
  drag(ui.get(".bar"), "drop");
  await waitFor(() => ui.requests.some(r => r.method === "dock.pin"));
  assert.deepEqual(ui.requests.find(r => r.method === "dock.pin").params, { id: "terminal.desktop", pinned: true });
});

test("top bar opens the Meridian menu from its left button", async t => {
  const ui = await surface(t, "topbar");
  ui.get(".menu-brand").click();
  await waitFor(() => ui.requests.some(r => r.method === "menu.open"));
  assert.deepEqual(ui.requests.at(-1).params, { menu: "Meridian", x: 0 });
});

test("Meridian About reads host version and laptop details and closes with Escape", async t => {
  const ui = await surface(t, "desktop-menu", r => r.method === "windows.list" ? [] : r.method === "system.info" ? { version: "0.2.3", manufacturer: "ASUS", model: "VivoBook", os: "Fedora", processor: "Intel", memory: "8.0 GiB" } : null);
  ui.emit("popover.shown");
  ui.emit("menu.opened", { menu: "Meridian", x: 8 });
  await waitFor(() => !ui.get(".desktop-menu").classList.contains("hidden"));
  assert.ok(![...ui.window.document.querySelectorAll('[role="menuitem"]')].some(button => button.textContent === "Open applications…"));
  ui.get('[role="menuitem"]').click();
  await waitFor(() => ui.get(".version")?.textContent === "Version 0.2.3");
  assert.ok(ui.get("dl").textContent.includes("ASUS VivoBook"));
  assert.equal(ui.get(".about").getAttribute("role"), "dialog");
  assert.equal(ui.get(".source-link").href, "https://github.com/saikrishna488/meridian");
  ui.key("Escape");
  await waitFor(() => ui.requests.some(r => r.method === "popover.close"));
});

test("settings search finds common terms and opens window shortcuts", async t => {
  const ui = await surface(t, "settings");
  ui.type('[aria-label="Search settings"]', " audio ");
  await waitFor(() => ui.get('nav button[aria-label="Sound"]'));
  assert.equal(ui.window.document.querySelectorAll("nav button").length, 1);
  ui.type('[aria-label="Search settings"]', "keyboard");
  await waitFor(() => ui.get('nav button[aria-label="Home"]'));
  ui.get('nav button[aria-label="Home"]').click();
  assert.match(ui.get(".shortcut-card").textContent, /Minimize window/);
  assert.match(ui.get(".shortcut-card").textContent, /Snap left \/ right/);
});

test("Storage replaces Device Manager and confirms safe volume operations", async t => {
  const state = { total: 100e9, available: 40e9, source: "/dev/vda1", filesystem: "ext4", warning: null, volumes: [
    { path: "/dev/sdb1", name: "USB", kind: "part", size: 10e9, filesystem: "exfat", mount_points: ["/run/media/test/USB"], read_only: false, removable: true, parent: "/dev/sdb", actions: ["open", "unmount"] }
  ] };
  const ui = await surface(t, "settings", req => req.method === "settings.storage" ? state : req.method === "settings.storage_action" ? { error: { code: "failed", message: "Volume is busy. Close files and retry." } } : null);
  assert.equal(ui.get('nav [aria-label="Device Manager"]'), null);
  ui.get('nav [aria-label="Storage"]').click();
  await waitFor(() => ui.get('.storage-volume'));
  assert.match(ui.get('.storage-overview').textContent, /60%/);
  [...ui.window.document.querySelectorAll('.storage-volume button')].find(b => b.textContent === 'Unmount').click();
  await waitFor(() => ui.get('[role="dialog"]'));
  assert.equal(ui.requests.some(r => r.method === 'settings.storage_action'), false);
  [...ui.get('[role="dialog"]').querySelectorAll('button')].find(b => b.textContent === 'Continue').click();
  await waitFor(() => ui.get('[role="alert"]')?.textContent.includes('Volume is busy'));
  assert.deepEqual(ui.requests.find(r => r.method === 'settings.storage_action').params, { id: '/dev/sdb1', action: 'unmount', identity: null });
});

test("users protect current account and confirm deletion while keeping home files", async t => {
  const users = [
    { uid: 1000, name: "owner", real_name: "Owner", administrator: true, locked: false, current: true, home: "/home/owner" },
    { uid: 1001, name: "guest", real_name: "Guest", administrator: false, locked: false, current: false, home: "/home/guest" },
  ];
  const ui = await surface(t, "settings", req => req.method === "settings.users" ? users : null);
  ui.get('nav [aria-label="Users"]').click();
  await waitFor(() => ui.window.document.querySelectorAll(".user-management-row").length === 2);
  const rows = [...ui.window.document.querySelectorAll(".user-management-row")];
  assert.ok([...rows[0].querySelectorAll("button")].find(b => b.textContent === "Delete…").disabled);
  [...rows[1].querySelectorAll("button")].find(b => b.textContent === "Delete…").click();
  await waitFor(() => ui.get('[role="dialog"]'));
  assert.match(ui.get('[role="dialog"]').textContent, /files are kept/);
  assert.ok(!ui.requests.some(r => r.method === "settings.user_action"));
  ui.get('[role="dialog"] button[type="submit"]').click();
  await waitFor(() => ui.requests.some(r => r.method === "settings.user_action"));
  assert.deepEqual(ui.requests.find(r => r.method === "settings.user_action").params, { uid: 1001, action: "delete", password: null });
});

test("new user form validates matching passwords and sends selected account type", async t => {
  const ui = await surface(t, "settings", req => req.method === "settings.users" ? [] : null);
  ui.get('nav [aria-label="Users"]').click();
  await waitFor(() => [...ui.window.document.querySelectorAll("button")].some(b => b.textContent === "Add user…" && !b.disabled));
  [...ui.window.document.querySelectorAll("button")].find(b => b.textContent === "Add user…").click();
  await waitFor(() => ui.get('[aria-label="Username"]'));
  ui.type('[aria-label="Username"]', "new_user");
  ui.type('[aria-label="Full name"]', "New User");
  ui.type('[aria-label="New password"]', "long-password");
  ui.type('[aria-label="Confirm password"]', "different-password");
  ui.get('[role="dialog"]').dispatchEvent(new ui.window.Event("submit", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get('[role="alert"]'));
  assert.ok(!ui.requests.some(r => r.method === "settings.user_create"));
  ui.type('[aria-label="Confirm password"]', "long-password");
  ui.get('[role="dialog"]').dispatchEvent(new ui.window.Event("submit", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.requests.some(r => r.method === "settings.user_create"));
  assert.equal(ui.requests.find(r => r.method === "settings.user_create").params.administrator, false);
});

test("General shows system details and checks GitHub only on request", async t => {
  const ui = await surface(t, "settings", req => req.method === "settings.system_info" ? { version: "0.1.0", kernel: "6.12.0", architecture: "x86_64", hostname: "desktop", os: "Debian", processor: "Test CPU", memory: "16 GiB", manufacturer: "Test", model: "Desktop" } : req.method === "settings.check_updates" ? { installed: "0.1.0", latest: "v0.2.0", available: true, message: "A newer Meridian release is available.", published: null } : null);
  ui.get('nav [aria-label="General"]').click();
  await waitFor(() => ui.get(".settings-content").textContent.includes("6.12.0"));
  assert.ok(!ui.requests.some(r => r.method === "settings.check_updates"));
  [...ui.window.document.querySelectorAll("button")].find(b => b.textContent === "Check for updates").click();
  await waitFor(() => ui.get(".update-status").textContent.includes("v0.2.0"));
  assert.match(ui.get(".update-status").textContent, /newer Meridian release/);
  [...ui.window.document.querySelectorAll("button")].find(b => b.textContent === "View releases").click();
  await waitFor(() => ui.requests.some(r => r.method === "settings.open_releases"));
});


test("Calendar loads saved events and persists deletion through the host", async t => {
  const today = new Date();
  const date = `${today.getFullYear()}-${String(today.getMonth()+1).padStart(2,"0")}-${String(today.getDate()).padStart(2,"0")}`;
  const ui = await surface(t,"utilities",request => {
    if(request.method === "shell.hello") return {surface:"calendar",capabilities:[]};
    if(request.method === "calendar.list") return [{id:"event-1",date,title:"Design review",time:"10:30"}];
    return null;
  });
  await waitFor(() => ui.get(".event"));
  assert.match(ui.get(".event").textContent,/Design review/);
  ui.get('.event button').click();
  await waitFor(() => !ui.get(".event"));
  assert.deepEqual(ui.requests.find(r => r.method === "calendar.save").params,[]);
});

test("Clock switches between local time, stopwatch and timer", async t => {
  const ui = await surface(t,"utilities",request => request.method === "shell.hello" ? {surface:"clock",capabilities:[]} : null);
  await waitFor(() => ui.get(".tabs"));
  ui.window.document.querySelectorAll(".tabs button")[1].click();
  await waitFor(() => ui.get(".clock-face .toolbar"));
  assert.equal(ui.get(".clock-face .toolbar button").textContent,"Start");
  ui.get(".clock-face .toolbar button").click();
  await waitFor(() => ui.get(".clock-face .toolbar button").textContent === "Pause");
  ui.window.document.querySelectorAll(".tabs button")[2].click();
  await waitFor(() => ui.get('input[type="number"]'));
  ui.type('input[type="number"]',"0");
  await waitFor(() => ui.get(".clock-face .toolbar button").disabled);
});


test("launcher grid exposes descriptions and launches the keyboard selection", async t => {
  const items = [{ ...app("terminal", "Terminal"), subtitle: "Command line" }, app("browser", "Browser")];
  const ui = await surface(t, "applications", req => req.method === "search.query" ? searchResult(req.params.serial, items) : null);
  ui.emit("popover.shown");
  await waitFor(() => ui.get("#row-1"));
  assert.ok(ui.get(".grid-view"));
  assert.match(ui.get("#row-0").title, /Command line/);
  ui.get("#search").focus();
  ui.key("ArrowRight"); ui.key("ArrowRight");
  await waitFor(() => ui.get("#row-1").getAttribute("aria-selected") === "true");
  ui.key("Enter");
  await waitFor(() => ui.requests.some(req => req.method === "search.activate"));
  assert.equal(ui.requests.find(req => req.method === "search.activate").params.item_id, "browser");
});

test("Meridian menu requires confirmation for destructive power actions and recovers from failures", async t => {
  const ui = await surface(t, "desktop-menu", req => req.method.startsWith("session.")
    ? { error: { code: "failed", message: "Power action unavailable" } } : null);
  ui.emit("popover.shown");
  await waitFor(() => !ui.get(".desktop-menu").classList.contains("hidden"));
  for (const [id, method] of [["restart", "session.restart"], ["shutdown", "session.shutdown"], ["sign-out", "session.sign_out"]]) {
    ui.get(`#${id}`).click();
    await waitFor(() => ui.get(`#${id}`).textContent.endsWith("?"));
    assert.equal(ui.requests.some(req => req.method === method), false);
    ui.get(`#${id}`).click();
    await waitFor(() => ui.requests.some(req => req.method === method));
    await waitFor(() => !ui.get(`#${id}`).disabled);
    assert.equal(ui.get('[role="alert"]').textContent, "Power action unavailable");
  }
  for (const id of ["lock", "sleep"]) {
    ui.get(`#${id}`).click();
    await waitFor(() => ui.requests.some(req => req.method === `session.${id}`));
    await waitFor(() => !ui.get(`#${id}`).disabled);
  }
});

test("focused app name updates and its menu quits all app windows", async t => {
  const windows = [{ id: 7, name: "Terminal", title: "Shell", active: true }, { id: 8, name: "Terminal", title: "Other shell", active: false }, { id: 9, name: "Browser", title: "Web", active: false }];
  const ui = await surface(t, "topbar", r => r.method === "windows.list" ? windows : null);
  await waitFor(() => ui.get(".focused-app")?.textContent === "Terminal");
  ui.get(".focused-app").click();
  await waitFor(() => ui.requests.some(r => r.method === "menu.open"));
  assert.deepEqual(ui.requests.find(r => r.method === "menu.open").params, { menu: "Application", window_id: 7, x: 0 });
  const menu = await surface(t, "desktop-menu", r => r.method === "windows.list" ? windows.map(w => ({ ...w, active: false })) : null);
  menu.emit("popover.shown");
  menu.emit("menu.opened", { menu: "Application", window_id: 7, x: 40 });
  await waitFor(() => menu.get('[role="menuitem"]')?.textContent === "Quit Terminal");
  menu.get('[role="menuitem"]').click();
  await waitFor(() => menu.requests.filter(r => r.method === "windows.close").length === 2);
  assert.deepEqual(menu.requests.filter(r => r.method === "windows.close").map(r => r.params.id), [7, 8]);
  ui.emit("windows.changed", [{ id: 9, name: "Browser", title: "Web", active: true }]);
  await waitFor(() => ui.get(".focused-app")?.textContent === "Browser");
  ui.emit("windows.changed", []);
  await waitFor(() => !ui.get(".focused-app"));
});

test("dock forwards running window context to the native menu", async t => {
  const ui = await surface(t, "panel", r => r.method === "windows.list" ? [{ id: 7, name: "Terminal", title: "Shell", active: true }] : null);
  await waitFor(() => ui.get('[aria-label="Terminal"]'));
  ui.get('[aria-label="Terminal"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.requests.some(r => r.method === "menu.open"));
  const params = ui.requests.find(r => r.method === "menu.open").params;
  assert.equal(params.menu, "DockItem"); assert.equal(params.window_id, 7); assert.equal(params.item_name, "Terminal"); assert.equal(params.pinned, false);
  const menu = await surface(t, "desktop-menu", r => r.method === "windows.list" ? [{ id: 7, name: "Terminal", title: "Shell", active: true }] : null);
  menu.emit("popover.shown"); menu.emit("menu.opened", params);
  await waitFor(() => menu.get('.dock-item-menu'));
  await waitFor(() => buttonWithText(menu, "Quit") && !buttonWithText(menu, "Quit").disabled);
  buttonWithText(menu, "Quit").click();
  await waitFor(() => menu.requests.some(r => r.method === "windows.close"));
  assert.deepEqual(menu.requests.find(r => r.method === "windows.close").params, { id: 7 });
});

test("Control Center becomes interactive while an optional service is slow", async t => {
  let complete;
  const ui = await surface(t, "options", req => req.method === "settings.get"
    ? new Promise(resolve => { complete = resolve; }) : null);
  ui.emit("popover.shown");
  await waitFor(() => !ui.get("#menu").classList.contains("hidden"));
  assert.ok(complete);
  assert.equal(ui.get("#wifi").disabled, true);
  complete({ wifi: true, bluetooth: null, brightness: null, volume: null });
  await waitFor(() => !ui.get("#wifi").disabled);
});

const wirelessState = { wifi: true, bluetooth: true, brightness: 50, volume: 40 };
const savedWifi = { uuid: "12345678-1234-1234-1234-123456789abc", name: "Studio", device: "wlan0" };
const wifiConfig = { uuid: savedWifi.uuid, autoconnect: true, metered: "unknown", method: "manual", addresses: "192.168.1.20/24", gateway: "192.168.1.1", dns: "1.1.1.1", ignore_auto_dns: true };
function buttonWithText(ui, text, root = ui.window.document) { return [...root.querySelectorAll("button")].find(b => b.textContent === text); }

test("Wi-Fi exposes access point details and edits saved configuration without reconnecting", async t => {
  const network = { ssid: "Studio", bssid: "AA:BB:CC:DD:EE:FF", device: "wlan0", signal: 85, security: "WPA3", connected: true, frequency: 5180, channel: 36, rate: "540 Mbit/s" };
  const ui = await surface(t, "settings", r => r.method === "settings.get" ? wirelessState : r.method === "settings.wifi_networks" ? [network] : r.method === "settings.wifi_profiles" ? [savedWifi] : r.method === "settings.wifi_config" ? wifiConfig : null);
  ui.get('nav [aria-label="Wi-Fi"]').click();
  await waitFor(() => ui.get('[aria-label="Configure Studio"]') && !ui.get('[aria-label="Configure Studio"]').disabled);
  ui.get('[aria-label="Details for Studio on AA:BB:CC:DD:EE:FF"]').click();
  await waitFor(() => ui.get(".wireless-properties"));
  assert.match(ui.get(".wireless-properties").textContent, /5180 MHz · Channel 36/);
  assert.match(ui.get(".wireless-properties").textContent, /AA:BB:CC:DD:EE:FF/);
  ui.get('[aria-label="Configure Studio"]').click();
  await waitFor(() => ui.get('[aria-label="DNS servers"]'));
  assert.equal(ui.get('[aria-label="IPv4 addresses"]').value, "192.168.1.20/24");
  await waitFor(() => ui.window.glisseSwitchReady(ui.get("#wifi-autoconnect")));
  ui.get('#wifi-autoconnect').click();
  await waitFor(() => ui.get('#wifi-autoconnect').getAttribute('aria-checked') === 'false');
  ui.type('[aria-label="DNS servers"]', "9.9.9.9, 1.1.1.1");
  ui.get('[role="dialog"]').dispatchEvent(new ui.window.Event("submit", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.requests.some(r => r.method === "settings.wifi_config_set"));
  assert.deepEqual(ui.requests.find(r => r.method === "settings.wifi_config_set").params, { ...wifiConfig, autoconnect: false, dns: "9.9.9.9, 1.1.1.1" });
  assert.equal(ui.requests.some(r => r.method === "settings.wifi_profile_action"), false);
  await waitFor(() => !ui.get('[role="dialog"]'));
  assert.match(ui.get('[role="status"]').textContent, /Reconnect/);
});

test("Wi-Fi preserves edits on save failure and requires confirmation before forgetting", async t => {
  const ui = await surface(t, "settings", r => r.method === "settings.get" ? wirelessState : r.method === "settings.wifi_profiles" ? [savedWifi] : r.method === "settings.wifi_config" ? wifiConfig : r.method === "settings.wifi_config_set" ? { error: { code: "failed", message: "Permission denied" } } : null);
  ui.get('nav [aria-label="Wi-Fi"]').click();
  await waitFor(() => ui.get('[aria-label="Configure Studio"]') && !ui.get('[aria-label="Configure Studio"]').disabled);
  ui.get('[aria-label="Configure Studio"]').click();
  await waitFor(() => ui.get('[aria-label="DNS servers"]'));
  ui.type('[aria-label="DNS servers"]', "8.8.8.8");
  ui.get('[role="dialog"]').dispatchEvent(new ui.window.Event("submit", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get('[role="dialog"] [role="alert"]'));
  assert.equal(ui.get('[aria-label="DNS servers"]').value, "8.8.8.8");
  assert.match(ui.get('[role="alert"]').textContent, /Permission denied/);
  await waitFor(() => !buttonWithText(ui, "Forget network…").disabled);
  buttonWithText(ui, "Forget network…").click();
  await waitFor(() => ui.get('[aria-label="Forget Studio"]'));
  assert.equal(ui.requests.some(r => r.method === "settings.wifi_profile_action"), false);
  buttonWithText(ui, "Cancel", ui.get('[role="dialog"]')).click();
  await waitFor(() => ui.get('[aria-label="DNS servers"]'));
  assert.equal(ui.get('[aria-label="DNS servers"]').value, "8.8.8.8");
  buttonWithText(ui, "Forget network…").click();
  await waitFor(() => ui.get('[aria-label="Forget Studio"]'));
  buttonWithText(ui, "Forget", ui.get('[role="dialog"]')).click();
  await waitFor(() => ui.requests.some(r => r.method === "settings.wifi_profile_action"));
  assert.deepEqual(ui.requests.find(r => r.method === "settings.wifi_profile_action").params, { uuid: savedWifi.uuid, action: "forget" });
});

test("Bluetooth groups paired devices, shows battery and address, and supports trust and confirmed removal", async t => {
  const paired = { path: "/org/bluez/hci0/dev_AA_BB", name: "Studio headphones", address: "AA:BB:CC:DD:EE:FF", paired: true, connected: true, trusted: false, icon: "audio-headphones", battery: 74 };
  const nearby = { ...paired, path: "/org/bluez/hci0/dev_CC_DD", name: "Keyboard", paired: false, connected: false, icon: "input-keyboard", battery: null };
  const ui = await surface(t, "settings", r => r.method === "settings.get" ? wirelessState : r.method === "settings.bluetooth_devices" ? [paired, nearby] : null);
  ui.get('nav [aria-label="Bluetooth"]').click();
  await waitFor(() => buttonWithText(ui, "Pair new device") && !buttonWithText(ui, "Pair new device").disabled);
  buttonWithText(ui, "Pair new device").click();
  await waitFor(() => ui.window.document.querySelectorAll(".wireless-item").length === 2 && !buttonWithText(ui, "Pair").disabled);
  assert.match(ui.window.document.querySelectorAll(".wireless-list")[0].textContent, /Studio headphones/);
  assert.match(ui.window.document.querySelectorAll(".wireless-list")[1].textContent, /Keyboard/);
  ui.get('[aria-label="Details for Studio headphones"]').click();
  await waitFor(() => ui.get('[aria-label="Trust Studio headphones"]'));
  assert.match(ui.get(".wireless-properties").textContent, /AA:BB:CC:DD:EE:FF/);
  assert.match(ui.get(".wireless-properties").textContent, /74%/);
  ui.get('[aria-label="Trust Studio headphones"]').click();
  await waitFor(() => ui.requests.some(r => r.method === "settings.bluetooth_action"));
  assert.deepEqual(ui.requests.find(r => r.method === "settings.bluetooth_action").params, { path: paired.path, action: "trust" });
  await waitFor(() => !buttonWithText(ui, "Forget device…").disabled);
  buttonWithText(ui, "Forget device…").click();
  await waitFor(() => ui.get('[role="dialog"]'));
  assert.equal(ui.requests.some(r => r.params?.action === "forget"), false);
  buttonWithText(ui, "Forget", ui.get('[role="dialog"]')).click();
  await waitFor(() => ui.requests.some(r => r.params?.action === "forget"));
  assert.deepEqual(ui.requests.find(r => r.params?.action === "forget").params, { path: paired.path, action: "forget" });
});

test("turning Wi-Fi off hides stale nearby results while saved configurations remain available", async t => {
  const ui = await surface(t, "settings", r => r.method === "settings.get" ? wirelessState : r.method === "settings.wifi_profiles" ? [savedWifi] : r.method === "settings.wifi_networks" ? [{ ssid: "Studio", bssid: "AA:BB:CC:DD:EE:FF", device: "wlan0", signal: 80, security: "WPA2", connected: true }] : null);
  ui.get('nav [aria-label="Wi-Fi"]').click();
  await waitFor(() => ui.get(".wireless-item"));
  ui.emit("settings.changed", { ...wirelessState, wifi: false });
  await waitFor(() => !ui.get(".wireless-item"));
  assert.ok(ui.get('[aria-label="Configure Studio"]'));
  assert.equal(buttonWithText(ui, "New connection").disabled, true);
  assert.match(ui.get(".wireless-empty").textContent, /Wi-Fi is off/);
});

test("hidden native menus release DOM input while preserving the host close handshake", async t => {
  for (const name of ["applications", "options", "notifications", "desktop-menu"]) {
    const ui = await surface(t, name, request => request.method === "notifications.get"
      ? { available: true, items: [], do_not_disturb: false }
      : request.method === "search.query" ? searchResult(request.params.serial, []) : null);
    const menu = ui.get('section[role="dialog"], section[role="menu"]');
    assert.ok(menu, name);
    assert.equal(menu.hasAttribute("inert"), true, `${name}: initially inert`);
    const scrim = ui.get('.scrim, .desktop-scrim');
    assert.equal(scrim.hidden, true, `${name}: initially no pointer scrim`);
    ui.emit("popover.shown");
    await waitFor(() => !menu.hasAttribute("inert"), `${name}: menu did not become interactive`);
    assert.equal(scrim.hidden, false);
    ui.emit("popover.dismiss");
    await waitFor(() => menu.hasAttribute("inert"));
    assert.equal(scrim.hidden, true, `${name}: scrim released during close`);
    await waitFor(() => ui.requests.some(request => request.method === "popover.close"));
  }
});

test("Wi-Fi distinguishes missing hardware from virtual Ethernet connectivity", async t => {
  const ui = await surface(t, "settings", req => req.method === 'settings.capability'
    ? { state: req.params.feature === 'wifi' ? 'hardware_unavailable' : 'ready', message: req.params.feature === 'wifi' ? 'No Wi-Fi adapter detected. Ethernet and virtual network connections remain available.' : 'An active network connection is available.', adapters: [] }
    : req.method === 'settings.wifi_networks' || req.method === 'settings.wifi_profiles' ? [] : null);
  ui.get('nav [aria-label="Wi-Fi"]').click();
  await waitFor(() => ui.get('.wireless-page')?.textContent.includes('An active network connection'));
  assert.equal(ui.get('.wireless-summary [aria-label="Wi-Fi"]').disabled, true);
  assert.match(ui.get('.wireless-summary').textContent, /No Wi-Fi adapter/);
});

test("wireless reports unavailable services and retains independently loaded saved profiles", async t => {
  const ui = await surface(t, "settings", req => req.method === 'settings.capability'
    ? { state: 'service_unavailable', message: req.params.feature === 'bluetooth' ? 'BlueZ unavailable. Start the service and retry.' : 'NetworkManager unavailable. Start the service and retry.', adapters: [] }
    : req.method === 'settings.wifi_networks' || req.method === 'settings.bluetooth_devices' ? { error: { code: 'unavailable', message: 'Service unavailable' } }
    : req.method === 'settings.wifi_profiles' ? [{ uuid: 'profile', name: 'Saved office', device: '' }] : null);
  ui.get('nav [aria-label="Wi-Fi"]').click();
  await waitFor(() => ui.get('.wireless-page')?.textContent.includes('Saved office'));
  assert.match(ui.get('.wireless-summary').textContent, /NetworkManager unavailable/);
  ui.get('nav [aria-label="Bluetooth"]').click();
  await waitFor(() => ui.get('.wireless-summary')?.textContent.includes('BlueZ unavailable'));
  assert.equal(ui.get('.wireless-summary [role="switch"]').disabled, true);
  assert.ok(ui.get('[role="alert"]'));
});

test("Displays keeps unsupported compositor output information read-only", async t => {
  const ui = await surface(t, 'settings', req => req.method === 'settings.capability'
    ? { state: 'unsupported', message: 'Compositor does not support configuration', adapters: [] }
    : req.method === 'settings.displays' ? [{ name: 'Virtual-1', description: '', enabled: true, x: 0, y: 0, scale: 1, transform: 0, modes: [], current: { width: 1280, height: 720, refresh: 60000, preferred: false } }] : null);
  ui.get('nav [aria-label="Displays"]').click();
  await waitFor(() => ui.get('.display-tile'));
  assert.match(ui.get('.settings-content').textContent, /read-only/);
  assert.equal(ui.get('[aria-label="Enable display"]').disabled, true);
  assert.equal([...ui.window.document.querySelectorAll('button')].find(b => b.textContent === 'Apply changes').disabled, true);
  assert.equal(ui.requests.some(r => r.method === 'settings.display_apply'), false);
});

test("Storage error has a retry and recovers without unrelated hardware probes", async t => {
  let count = 0;
  const ui = await surface(t, 'settings', req => req.method === 'settings.storage'
    ? ++count === 1 ? { error: { code: 'unavailable', message: 'lsblk is unavailable' } } : { total: 0, available: 0, filesystem: '', source: '', warning: 'Capacity unavailable', volumes: [] } : null);
  ui.get('nav [aria-label="Storage"]').click();
  await waitFor(() => ui.get('[role="alert"]'));
  [...ui.get('[role="alert"]').querySelectorAll('button')].find(b => b.textContent === 'Retry').click();
  await waitFor(() => ui.get('.storage-overview'));
  assert.equal(ui.requests.some(r => r.method === 'settings.devices'), false);
});
