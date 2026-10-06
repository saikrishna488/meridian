// Applications menu: search + a text list of installed applications.
//
// The list shows the results of `search.query`; an empty query lists every
// application. Keyboard focus stays in the search field while Up/Down move
// the selection (the listbox/activedescendant pattern), so typing always
// searches and Left/Right keep editing the query.

import { afterTransition, BridgeError, cssDuration, hello, nextFrame, on, request } from "../../lib/bridge.js";
import type { ProviderId } from "../../lib/generated/ProviderId";
import type { SearchItem } from "../../lib/generated/SearchItem";
import type { SearchResults } from "../../lib/generated/SearchResults";

interface Result {
  provider: ProviderId;
  item: SearchItem;
}

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} missing`);
  return el as T;
}

const menu = byId<HTMLElement>("menu");
const scrim = byId<HTMLElement>("scrim");
const input = byId<HTMLInputElement>("search");
const scroller = byId<HTMLElement>("scroller");
const list = byId<HTMLUListElement>("list");
const empty = byId<HTMLParagraphElement>("empty");
const status = byId<HTMLParagraphElement>("status");

let results: Result[] = [];
let selected = -1;
let serial = 0;
let open = false;

// Rows are created once per result and reused across queries, so typing
// only reorders existing nodes.
const rows = new Map<string, HTMLLIElement>();
const resultOf = new WeakMap<Element, Result>();
let nextRowId = 0;

function rowFor(result: Result): HTMLLIElement {
  const key = `${result.provider}:${result.item.id}`;
  let row = rows.get(key);
  if (!row) {
    row = createRow(result.item);
    rows.set(key, row);
  }
  resultOf.set(row, result);
  return row;
}

function createRow(item: SearchItem): HTMLLIElement {
  const row = document.createElement("li");
  row.className = "row";
  row.id = `row-${nextRowId++}`;
  row.setAttribute("role", "option");
  row.setAttribute("aria-selected", "false");

  const name = document.createElement("span");
  name.className = "name";
  name.textContent = item.title;
  row.append(name);

  if (item.subtitle) {
    const detail = document.createElement("span");
    detail.className = "detail";
    detail.textContent = item.subtitle;
    row.append(detail);
  }
  return row;
}

// ---- search ---------------------------------------------------------------

async function runQuery(): Promise<void> {
  const mine = ++serial;
  const res: SearchResults = await request({
    method: "search.query",
    params: { query: input.value, serial: mine },
  });
  if (res.serial !== serial) return; // a newer query is in flight
  render(res.sections.flatMap((s) => s.items.map((item) => ({ provider: s.provider, item }))));
}

function render(next: Result[]): void {
  results = next;
  list.replaceChildren(...results.map(rowFor));
  const searching = input.value.trim() !== "";
  empty.hidden = results.length > 0;
  select(searching && results.length > 0 ? 0 : -1, { scroll: false });
  if (!searching) scroller.scrollTo({ top: 0, behavior: "instant" });
}

// ---- selection ----------------------------------------------------------

function select(index: number, { scroll = true } = {}): void {
  list.querySelector('[aria-selected="true"]')?.setAttribute("aria-selected", "false");
  selected = index;
  const row = index >= 0 ? (list.children[index] as HTMLElement | undefined) : undefined;
  if (!row) {
    input.removeAttribute("aria-activedescendant");
    return;
  }
  row.setAttribute("aria-selected", "true");
  input.setAttribute("aria-activedescendant", row.id);
  if (scroll) row.scrollIntoView({ block: "nearest", behavior: "smooth" });
}

function visibleRows(): number {
  const row = list.children[0] as HTMLElement | undefined;
  if (!row) return 1;
  return Math.max(1, Math.floor(scroller.clientHeight / row.offsetHeight) - 1);
}

function move(delta: number): void {
  if (results.length === 0) return;
  if (selected < 0) return select(0);
  // Moves past the ends stop at the end instead of doing nothing.
  select(Math.min(results.length - 1, Math.max(0, selected + delta)));
}

// ---- activation -------------------------------------------------------

async function activate(index: number): Promise<void> {
  const row = list.children[index];
  const result = row && resultOf.get(row);
  if (!row || !result) return;

  row.classList.remove("launching");
  void (row as HTMLElement).offsetWidth; // restart the animation
  row.classList.add("launching");

  try {
    await request({
      method: "search.activate",
      params: { provider: result.provider, item_id: result.item.id },
    });
  } catch (e) {
    status.textContent = `Couldn’t open ${result.item.title}`;
    throw e;
  }
  await dismiss();
}

// ---- show / hide -------------------------------------------------------------

async function show(): Promise<void> {
  open = true;
  status.textContent = "";
  if (input.value) input.value = "";
  await runQuery();
  await nextFrame();
  menu.classList.remove("hidden");
  input.focus();
}

async function dismiss(): Promise<void> {
  if (!open) return;
  open = false;
  menu.classList.add("hidden");
  await afterTransition(menu, cssDuration("--dur-fast"));
  await request({ method: "popover.close" });
}

function report(e: unknown): void {
  console.error("meridian applications:", e);
  if (e instanceof BridgeError && e.code === "unavailable") {
    status.textContent = "Not connected to meridian-shell";
  }
}

// ---- input wiring ---------------------------------------------------------

input.addEventListener("input", () => runQuery().catch(report));

window.addEventListener("keydown", (e) => {
  if (e.isComposing) return;
  const handled = (() => {
    switch (e.key) {
      case "ArrowDown":
        return move(1), true;
      case "ArrowUp":
        return move(-1), true;
      case "Tab":
        return move(e.shiftKey ? -1 : 1), true;
      case "PageDown":
        return move(visibleRows()), true;
      case "PageUp":
        return move(-visibleRows()), true;
      case "Enter":
        if (selected >= 0) activate(selected).catch(report);
        return true;
      case "Escape":
        if (input.value) {
          input.value = "";
          runQuery().catch(report);
        } else {
          dismiss().catch(report);
        }
        return true;
      default:
        // Start typing from anywhere: route printable keys to the search.
        if (e.key.length === 1 && !e.ctrlKey && !e.altKey && !e.metaKey && document.activeElement !== input) {
          input.focus();
        }
        return false;
    }
  })();
  if (handled) e.preventDefault();
});

list.addEventListener("click", (e) => {
  const row = (e.target as Element).closest(".row");
  if (!row) return;
  const index = Array.prototype.indexOf.call(list.children, row);
  select(index, { scroll: false });
  activate(index).catch(report);
});

// Clicks inside the menu keep keyboard focus in the search field.
menu.addEventListener("mousedown", (e) => {
  if (e.target !== input) {
    e.preventDefault();
    input.focus();
  }
});
scrim.addEventListener("click", () => dismiss().catch(report));

on("popover.shown", () => show().catch(report));
on("popover.dismiss", () => dismiss().catch(report));
on("search.invalidated", () => {
  rows.clear(); // names may have changed
  runQuery().catch(report);
});

hello().catch(report);
