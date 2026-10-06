// Login screen. The password goes straight from the field to the host,
// which relays it to greetd; nothing is stored.

import { BridgeError, hello, request } from "../../lib/bridge.js";

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} missing`);
  return el as T;
}

const form = byId<HTMLFormElement>("form");
const username = byId<HTMLInputElement>("username");
const password = byId<HTMLInputElement>("password");
const session = byId<HTMLSelectElement>("session");
const submit = byId<HTMLButtonElement>("submit");
const message = byId<HTMLParagraphElement>("message");

function say(text: string, kind: "error" | "info" = "error"): void {
  message.textContent = text;
  message.classList.toggle("info", kind === "info");
}

function busy(on: boolean, label = "Log in"): void {
  for (const el of [username, password, session, submit]) el.disabled = on;
  submit.textContent = label;
}

// ---- clock --------------------------------------------------------------------

const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });
const dateFormat = new Intl.DateTimeFormat(undefined, { weekday: "long", month: "long", day: "numeric" });

function tick(): void {
  const now = new Date();
  const clock = byId<HTMLTimeElement>("clock");
  clock.textContent = timeFormat.format(now);
  clock.dateTime = now.toISOString();
  byId("date").textContent = dateFormat.format(now);
  // Wake once per minute, on the minute.
  setTimeout(tick, 60_000 - (now.getSeconds() * 1000 + now.getMilliseconds()) + 50);
}

// ---- login --------------------------------------------------------------------

form.addEventListener("submit", async (e) => {
  e.preventDefault();
  if (!username.value.trim()) {
    say("Enter your username");
    return username.focus();
  }
  say("");
  busy(true, "Logging in…");
  try {
    await request({
      method: "greeter.login",
      params: { username: username.value.trim(), password: password.value, session: session.value },
    });
    password.value = "";
    busy(true, "Starting…");
    say(`Starting ${session.selectedOptions[0]?.textContent ?? "session"}…`, "info");
  } catch (err) {
    password.value = "";
    busy(false);
    say(err instanceof BridgeError ? err.message : "Login failed");
    form.classList.remove("shake");
    void form.offsetWidth; // restart the animation
    form.classList.add("shake");
    password.focus();
  }
});

// ---- power (click twice to confirm) ---------------------------------------------

for (const button of document.querySelectorAll<HTMLButtonElement>(".power-button")) {
  let timer: number | undefined;
  button.addEventListener("click", async () => {
    if (!button.classList.contains("armed")) {
      button.classList.add("armed");
      button.textContent = `${button.dataset.label}?`;
      timer = setTimeout(() => {
        button.classList.remove("armed");
        button.textContent = button.dataset.label ?? "";
      }, 3000);
      return;
    }
    clearTimeout(timer);
    const action = button.dataset.action === "reboot" ? "reboot" : "power-off";
    try {
      await request({ method: "greeter.power", params: { action } });
      say(action === "reboot" ? "Restarting…" : "Shutting down…", "info");
    } catch (err) {
      say(err instanceof BridgeError ? err.message : "Couldn’t do that");
      button.classList.remove("armed");
      button.textContent = button.dataset.label ?? "";
    }
  });
}

// ---- init -----------------------------------------------------------------------

async function init(): Promise<void> {
  tick();
  await hello();
  const info = await request({ method: "greeter.info" });
  byId("host").textContent = info.hostname;
  session.replaceChildren(
    ...info.sessions.map((s) => {
      const option = document.createElement("option");
      option.value = s.id;
      option.textContent = s.name;
      option.selected = s.id === info.last_session;
      return option;
    }),
  );
  if (info.sessions.length === 0) {
    say("No desktop sessions are installed");
    busy(true);
    return;
  }
  if (info.last_user) {
    username.value = info.last_user;
    password.focus();
  } else {
    username.focus();
  }
}

init().catch((e) => {
  console.error("meridian greeter:", e);
  say("The login screen couldn’t start. Switch to a text console (Ctrl+Alt+F3) to log in.");
});
