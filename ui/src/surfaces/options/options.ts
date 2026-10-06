// Options menu: Wi-Fi and Bluetooth switches, brightness and volume sliders.
// Controls show the real system state; unavailable ones are disabled.

import { afterTransition, BridgeError, cssDuration, hello, nextFrame, on, request } from "../../lib/bridge.js";
import type { SettingsState } from "../../lib/generated/SettingsState";

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} missing`);
  return el as T;
}

const menu = byId<HTMLElement>("menu");
const scrim = byId<HTMLElement>("scrim");
const status = byId<HTMLParagraphElement>("status");

let open = false;

function report(e: unknown): void {
  console.error("meridian options:", e);
  status.textContent = e instanceof BridgeError ? e.message : "Something went wrong";
}

// ---- switches -------------------------------------------------------------

type SwitchKey = "wifi" | "bluetooth";

function switchControl(key: SwitchKey, method: "settings.set_wifi" | "settings.set_bluetooth") {
  const el = byId<HTMLButtonElement>(key);
  const text = el.querySelector(".switch-text") as HTMLElement;
  const render = (value: boolean | null) => {
    el.disabled = value === null;
    el.setAttribute("aria-checked", String(value === true));
    text.textContent = value === null ? "—" : value ? "On" : "Off";
  };
  el.addEventListener("click", () => {
    const next = el.getAttribute("aria-checked") !== "true";
    render(next); // optimistic; the service's change event confirms or reverts
    status.textContent = "";
    request({ method, params: { enabled: next } }).catch((e) => {
      render(!next);
      report(e);
    });
  });
  return render;
}

// ---- sliders ------------------------------------------------------------------

function sliderControl(key: "brightness" | "volume", method: "settings.set_brightness" | "settings.set_volume") {
  const el = byId<HTMLInputElement>(key);
  const out = byId<HTMLOutputElement>(`${key}-value`);
  let dragging = false;
  let pending: number | null = null;
  let scheduled = false;

  const paint = (value: number) => {
    el.value = String(value);
    el.style.setProperty("--fill", `${value}%`);
    out.textContent = `${value}%`;
  };
  const render = (value: number | null) => {
    el.disabled = value === null;
    if (value === null) {
      out.textContent = "—";
      el.style.setProperty("--fill", "0%");
      return;
    }
    if (!dragging) paint(value); // don't fight the user's thumb
  };
  // At most one request per frame while dragging.
  const flush = () => {
    scheduled = false;
    if (pending === null) return;
    const percent = pending;
    pending = null;
    request({ method, params: { percent } }).catch(report);
  };
  el.addEventListener("input", () => {
    const value = Number(el.value);
    paint(value);
    pending = value;
    if (!scheduled) {
      scheduled = true;
      requestAnimationFrame(flush);
    }
  });
  el.addEventListener("pointerdown", () => (dragging = true));
  el.addEventListener("pointerup", () => (dragging = false));
  el.addEventListener("pointercancel", () => (dragging = false));
  return render;
}

const renderWifi = switchControl("wifi", "settings.set_wifi");
const renderBluetooth = switchControl("bluetooth", "settings.set_bluetooth");
const renderBrightness = sliderControl("brightness", "settings.set_brightness");
const renderVolume = sliderControl("volume", "settings.set_volume");

function render(s: SettingsState): void {
  renderWifi(s.wifi);
  renderBluetooth(s.bluetooth);
  renderBrightness(s.brightness);
  renderVolume(s.volume);
}

// ---- show / hide -------------------------------------------------------------

async function show(): Promise<void> {
  open = true;
  status.textContent = "";
  render(await request({ method: "settings.get" }));
  await nextFrame();
  menu.classList.remove("hidden");
  menu.focus();
}

async function dismiss(): Promise<void> {
  if (!open) return;
  open = false;
  menu.classList.add("hidden");
  await afterTransition(menu, cssDuration("--dur-fast"));
  await request({ method: "popover.close" });
}

on("popover.shown", () => show().catch(report));
on("popover.dismiss", () => dismiss().catch(report));
on("settings.changed", (e) => render(e.data));

scrim.addEventListener("click", () => dismiss().catch(report));
window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    e.preventDefault();
    dismiss().catch(report);
  }
});

hello().catch(report);
