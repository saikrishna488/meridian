// Top bar: opens the menus and lists running windows.

import { hello, on, request } from "../../lib/bridge.js";
import type { WindowInfo } from "../../lib/generated/WindowInfo";

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} missing`);
  return el as T;
}

const applications = byId<HTMLButtonElement>("applications");
const options = byId<HTMLButtonElement>("options");
const list = byId<HTMLUListElement>("windows");

function report(e: unknown): void {
  console.error("meridian panel:", e);
}

applications.addEventListener("click", () =>
  request({ method: "popover.toggle", params: { popover: "applications" } }).catch(report),
);
options.addEventListener("click", () =>
  request({ method: "popover.toggle", params: { popover: "options" } }).catch(report),
);

function renderWindows(windows: WindowInfo[]): void {
  if (windows.length === 0) {
    const empty = document.createElement("li");
    empty.className = "windows-empty";
    empty.textContent = "No open windows";
    list.replaceChildren(empty);
    return;
  }
  list.replaceChildren(
    ...windows.map((w) => {
      const item = document.createElement("li");
      const button = document.createElement("button");
      button.className = "window";
      button.textContent = w.name;
      button.title = w.title ? `${w.name} — ${w.title}` : w.name;
      button.setAttribute("aria-pressed", String(w.active));
      button.addEventListener("click", () =>
        request({ method: "windows.activate", params: { id: w.id } }).catch(report),
      );
      item.append(button);
      return item;
    }),
  );
}

on("windows.changed", (e) => renderWindows(e.data));
on("popover.state", (e) => {
  applications.setAttribute("aria-expanded", String(e.data.applications));
  options.setAttribute("aria-expanded", String(e.data.options));
});

async function init(): Promise<void> {
  await hello();
  renderWindows(await request({ method: "windows.list" }));
}

init().catch(report);
