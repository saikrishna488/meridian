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
  applyFontEffects();
}

export type FontEffects = { stroke: number; shadow: number; blur: number };
const fontEffectsKey = "meridian.font-effects";
const fontEffectsEvent = "meridian-font-effects";
const defaultFontEffects: FontEffects = { stroke: 0, shadow: 0, blur: 0 };

export function getFontEffects(): FontEffects {
  try {
    const value = JSON.parse(localStorage.getItem(fontEffectsKey) ?? "null") as Partial<FontEffects> | null;
    return {
      stroke: clampEffect(value?.stroke, 0, 1),
      shadow: clampEffect(value?.shadow, 0, 8),
      blur: clampEffect(value?.blur, 0, 2),
    };
  } catch { return { ...defaultFontEffects }; }
}

function clampEffect(value: unknown, min: number, max: number): number {
  return typeof value === "number" && Number.isFinite(value) ? Math.max(min, Math.min(max, value)) : 0;
}

export function applyFontEffects(effects: FontEffects = getFontEffects()): void {
  const root = document.documentElement;
  root.style.setProperty("--font-effect-stroke", `${effects.stroke}px`);
  root.style.setProperty("--font-effect-shadow", effects.shadow ? `0 1px ${effects.shadow}px color-mix(in srgb, currentColor 55%, transparent)` : "none");
  root.style.setProperty("--font-effect-blur", `${effects.blur}px`);
}

export function saveFontEffects(effects: FontEffects): void {
  try { localStorage.setItem(fontEffectsKey, JSON.stringify(effects)); } catch {}
  applyFontEffects(effects);
  window.dispatchEvent(new CustomEvent<FontEffects>(fontEffectsEvent, { detail: effects }));
}

export function subscribeFontEffects(callback: (effects: FontEffects) => void): () => void {
  const onLocal = (event: Event) => callback((event as CustomEvent<FontEffects>).detail ?? getFontEffects());
  const onStorage = (event: StorageEvent) => { if (event.key === fontEffectsKey) callback(getFontEffects()); };
  window.addEventListener(fontEffectsEvent, onLocal);
  window.addEventListener("storage", onStorage);
  return () => { window.removeEventListener(fontEffectsEvent, onLocal); window.removeEventListener("storage", onStorage); };
}
