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
  assert.equal(ui.get("#options"), null);
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

test("Control Center displays availability and updates settings without session actions", async t => {
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
  assert.equal(ui.get("#lock"), null);
  assert.equal(ui.get("#sleep"), null);
  assert.equal(ui.get("#sign-out"), null);
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


test("top bar hosts Options and the active application", async t => {
  const ui = await surface(t, "panel", request => request.method === "shell.hello" ? { surface: "topbar" } : request.method === "windows.list" ? [{ id: 1, name: "Browser", active: true }] : null);
  await waitFor(() => ui.get(".active-app")?.textContent === "Browser");
  assert.equal(ui.get(".bar"), null);
  ui.get("#options").click();
  await waitFor(() => ui.requests.some(r => r.method === "popover.toggle"));
  assert.deepEqual(ui.requests.at(-1), { method: "popover.toggle", params: { popover: "options" } });
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

test("dragging an application to the launcher dock pins its desktop id", async t => {
  const ui = await surface(t, "applications", request => request.method === "search.query" ? searchResult(request.params.serial, [app("terminal.desktop", "Terminal")]) : null);
  ui.emit("popover.shown");
  await waitFor(() => ui.get("#row-0"));
  const data = new Map();
  const transfer = { types: ["application/x-meridian-app"], setData: (key, value) => data.set(key, value), getData: key => data.get(key) };
  function drag(target, type) {
    const event = new ui.window.Event(type, { bubbles: true, cancelable: true });
    Object.defineProperty(event, "dataTransfer", { value: transfer });
    target.dispatchEvent(event);
  }
  drag(ui.get("#row-0"), "dragstart");
  drag(ui.get(".launcher-dock .bar"), "dragover");
  drag(ui.get(".launcher-dock .bar"), "drop");
  await waitFor(() => ui.requests.some(r => r.method === "dock.pin"));
  assert.deepEqual(ui.requests.find(r => r.method === "dock.pin").params, { id: "terminal.desktop", pinned: true });
});

test("top bar opens desktop menus from the Meridian symbol and File button", async t => {
  const ui = await surface(t, "panel", r => r.method === "shell.hello" ? { surface: "topbar" } : r.method === "windows.list" ? [] : null);
  assert.equal(ui.get(".topbar").textContent.includes("Applications"), false);
  ui.get(".menu-brand").click();
  await waitFor(() => ui.requests.some(r => r.method === "menu.open"));
  assert.equal(ui.requests.at(-1).params.menu, "Meridian");
  [...ui.window.document.querySelectorAll(".topbar button")].find(b => b.textContent === "File").click();
  await waitFor(() => ui.requests.at(-1).params.menu === "File");
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
