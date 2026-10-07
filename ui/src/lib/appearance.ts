import { useEffect, useState } from "react";

export type Appearance = "light" | "dark";
const eventName = "meridian-theme";

export function currentAppearance(): Appearance {
  return document.documentElement.dataset.theme === "dark" ? "dark" : "light";
}

/** Apply Meridian's appearance to the whole WebView and notify mounted apps. */
export function applyAppearance(value: string | undefined): Appearance {
  const mode: Appearance = value === "dark" ? "dark" : "light";
  const root = document.documentElement;
  root.dataset.theme = mode;
  root.style.colorScheme = mode;
  window.dispatchEvent(new CustomEvent<Appearance>(eventName, { detail: mode }));
  return mode;
}

export function subscribeAppearance(callback: (mode: Appearance) => void): () => void {
  const listener = (event: Event) => callback((event as CustomEvent<Appearance>).detail ?? currentAppearance());
  window.addEventListener(eventName, listener);
  return () => window.removeEventListener(eventName, listener);
}

/** React hook for system apps that need to respond to appearance changes. */
export function useAppearance(): Appearance {
  const [mode, setMode] = useState<Appearance>(currentAppearance);
  useEffect(() => subscribeAppearance(setMode), []);
  return mode;
}
