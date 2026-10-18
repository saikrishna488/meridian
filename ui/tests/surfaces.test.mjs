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
  window.HTMLElement.prototype.scrollIntoView = function () {};
  window.HTMLElement.prototype.scrollTo = function () {};
  const requests = [];
  window.webkit = { messageHandlers: { meridian: { async postMessage(json) {
    const request = JSON.parse(json);
    requests.push(request);
    const response = await respond(request);
    return JSON.stringify(response?.error ? { ok: false, error: response.error } : { ok: true, result: response });
  } } } };
  const js = await readFile(new URL(`../dist/surfaces/${name}/${name}.js`, import.meta.url), "utf8");
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
  assert.ok(ui.get(".dock-trash #trash"));
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

test("Control Center displays availability and updates settings alongside power controls", async t => {
  const state = { wifi: true, bluetooth: null, brightness: 65, volume: 30 };
  const ui = await surface(t, "options", request => request.method === "settings.get" ? state : null);
  ui.emit("popover.shown");
  await waitFor(() => !ui.get("#menu").classList.contains("hidden"));
  assert.equal(ui.get("#wifi").getAttribute("aria-checked"), "true");
  assert.equal(ui.get("#bluetooth").disabled, true);
  assert.equal(ui.get("#brightness-value").textContent, "65%");
  ui.get("#wifi").click();
  await waitFor(() => ui.requests.some(request => request.method === "settings.set_wifi"));
  assert.deepEqual(ui.requests.find(request => request.method === "settings.set_wifi").params, { enabled: false });
  ui.emit("settings.changed", { ...state, volume: 80 });
  await waitFor(() => ui.get("#volume-value").textContent === "80%");
  ui.type("#volume", "42");
  await waitFor(() => ui.requests.some(request => request.method === "settings.set_volume"));
  assert.deepEqual(ui.requests.find(request => request.method === "settings.set_volume").params, { percent: 42 });
  assert.ok(ui.get("#lock"));
  assert.ok(ui.get("#sleep"));
  assert.ok(ui.get("#sign-out"));
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
  ui.get(".system-tray #options").click();
  await waitFor(() => ui.requests.some(r => r.method === "popover.toggle"));
  assert.deepEqual(ui.requests.at(-1), { method: "popover.toggle", params: { popover: "options" } });
  ui.emit("popover.state", { options: true, applications: false });
  await waitFor(() => ui.get("#options").getAttribute("aria-expanded") === "true");
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
  assert.deepEqual(ui.requests.find(r => r.method === "windows.activate").params, { id: 2 });
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

test("Device Manager groups hardware and confirms device-specific actions", async t => {
  const devices = [
    { id: "net:wlan0", category: "Wi-Fi adapters", name: "Wi-Fi adapter", status: "Connected", properties: [["Driver", "iwlwifi"]], actions: ["disconnect"] },
    { id: "storage:nvme0n1", category: "Storage", name: "SSD", status: "Connected", properties: [["Driver", "nvme"]], actions: ["storage"] },
  ];
  const ui = await surface(t, "settings", req => req.method === "settings.devices" ? devices : req.method === "settings.device_action" ? { error: { code: "failed", message: "Permission denied" } } : null);
  assert.ok(!ui.get('nav [aria-label="Driver Manager"]'));
  ui.get('nav [aria-label="Device Manager"]').click();
  await waitFor(() => ui.get(".device-properties h3")?.textContent === "Wi-Fi adapter");
  assert.equal(ui.window.document.querySelectorAll(".device-tree details").length, 2);
  assert.ok(ui.get(".device-properties").textContent.includes("iwlwifi"));
  [...ui.window.document.querySelectorAll(".device-properties button")].find(b => b.textContent === "Disconnect").click();
  await waitFor(() => ui.get('[role="dialog"]'));
  assert.ok(!ui.requests.some(r => r.method === "settings.device_action"));
  [...ui.get('[role="dialog"]').querySelectorAll("button")].find(b => b.textContent === "Disconnect").click();
  await waitFor(() => ui.get('[role="alert"]')?.textContent.includes("Permission denied"));
  assert.deepEqual(ui.requests.find(r => r.method === "settings.device_action").params, { id: "net:wlan0", action: "disconnect" });
  [...ui.window.document.querySelectorAll(".device-tree button")].find(b => b.textContent === "SSD").click();
  await waitFor(() => ui.get(".device-properties").textContent.includes("nvme"));
  assert.ok([...ui.window.document.querySelectorAll(".device-properties button")].some(b => b.textContent === "Open Disks"));
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

test("Nova confirms package changes and reports failure without claiming installation", async t => {
  const ui = await surface(t, "utilities", request => {
    if (request.method === "shell.hello") return { surface: "nova", capabilities: [] };
    if (request.method === "store.list") return [{id:"vlc",name:"VLC",category:"Media",description:"Video player",installed:false,available:true},{id:"missing",name:"Missing",category:"Media",description:"Unavailable",installed:false,available:false}];
    if (request.method === "store.action") return {error:{code:"failed",message:"Authentication cancelled"}};
    return null;
  });
  await waitFor(() => ui.get(".app-grid button"));
  assert.equal(ui.window.document.querySelectorAll(".app-grid button")[1].disabled,true);
  ui.get(".app-grid button").click();
  await waitFor(() => ui.get('[role="dialog"]'));
  assert.equal(ui.requests.filter(r => r.method === "store.action").length,0);
  ui.window.document.querySelectorAll(".dialog button")[1].click();
  await waitFor(() => ui.get('[role="alert"]')?.textContent === "Authentication cancelled");
  assert.deepEqual(ui.requests.find(r => r.method === "store.action").params,{id:"vlc",action:"install"});
  assert.equal(ui.get(".app-grid button").textContent,"Install");
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


test("launcher list view retains descriptions and launches the keyboard selection", async t => {
  const items = [{ ...app("terminal", "Terminal"), subtitle: "Command line" }, app("browser", "Browser")];
  const ui = await surface(t, "applications", req => req.method === "search.query" ? searchResult(req.params.serial, items) : null);
  ui.emit("popover.shown");
  await waitFor(() => ui.get("#row-1"));
  ui.get('[aria-label="List view"]').click();
  await waitFor(() => ui.get(".list-view .app-description"));
  assert.equal(ui.get(".app-description").textContent, "Command line");
  assert.equal(ui.window.localStorage.getItem("meridian.launcher.view"), "list");
  ui.get("#search").focus();
  ui.key("ArrowDown"); ui.key("ArrowDown");
  await waitFor(() => ui.get("#row-1").getAttribute("aria-selected") === "true");
  ui.key("Enter");
  await waitFor(() => ui.requests.some(req => req.method === "search.activate"));
  assert.equal(ui.requests.find(req => req.method === "search.activate").params.item_id, "browser");
});

test("Control Center requires confirmation for destructive power actions and recovers from failures", async t => {
  const ui = await surface(t, "options", req => req.method === "settings.get"
    ? { wifi: true, bluetooth: true, brightness: 50, volume: 40 }
    : req.method.startsWith("session.") ? { error: { code: "failed", message: "Power action unavailable" } } : null);
  ui.emit("popover.shown");
  await waitFor(() => !ui.get("#menu").classList.contains("hidden"));
  for (const [id, method] of [["restart", "session.restart"], ["shutdown", "session.shutdown"], ["sign-out", "session.sign_out"]]) {
    ui.get(`#${id}`).click();
    await waitFor(() => ui.get(`#${id}`).textContent.endsWith("?"));
    assert.equal(ui.requests.some(req => req.method === method), false);
    ui.get(`#${id}`).click();
    await waitFor(() => ui.requests.some(req => req.method === method));
    await waitFor(() => !ui.get(`#${id}`).disabled);
    assert.equal(ui.get("#status").textContent, "Power action unavailable");
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

test("dock context offers only quit and remove, including unpinned running apps", async t => {
  const ui = await surface(t, "panel", r => r.method === "windows.list" ? [{ id: 7, name: "Terminal", title: "Shell", active: true }] : null);
  await waitFor(() => ui.get('[aria-label="Terminal"]'));
  ui.get('[aria-label="Terminal"]').dispatchEvent(new ui.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  await waitFor(() => ui.get('.dock-context'));
  const buttons = [...ui.window.document.querySelectorAll('.dock-context button')];
  assert.deepEqual(buttons.map(b => b.textContent), ["Quit", "Remove from dock"]);
  assert.equal(buttons[1].disabled, true);
  buttons[0].click();
  await waitFor(() => ui.requests.some(r => r.method === "windows.close"));
  assert.deepEqual(ui.requests.find(r => r.method === "windows.close").params, { id: 7 });
});
