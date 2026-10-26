import { useEffect, useState } from "react";

export type Appearance = "dark";
const eventName = "meridian-theme";

export function currentAppearance(): Appearance {
  return "dark";
}

/** Apply Meridian's appearance to the whole WebView and notify mounted apps. */
export function applyAppearance(value: string | undefined): Appearance {
  const mode: Appearance = "dark";
  const root = document.documentElement;
  root.dataset.theme = "dark";
  root.style.colorScheme = "dark";
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

export function applyFont(font: string | undefined): void {
  const allowed = ["Inter", "Noto Sans", "DejaVu Sans", "Liberation Sans"];
  const selected = font && allowed.includes(font) ? font : "Inter";
  document.documentElement.style.setProperty("--font-ui", `"${selected}", sans-serif`);
  document.documentElement.style.setProperty("--font-display", "var(--font-ui)");
}
