import { on, request } from "./bridge";
import type { EngineState } from "./generated/EngineState";

export function applyEngineState(state: EngineState): void {
  const root = document.documentElement;
  root.dataset.engineFont = state.active.font_enabled ? state.active.font_profile : "off";
  root.dataset.engineAnimations = state.animations_enabled ? "on" : "off";
  root.dataset.engineTransparency = state.transparency_enabled ? "on" : "off";
  root.dataset.enginePerformance = state.active.cpu_enabled ? "efficient" : "balanced";
  window.dispatchEvent(new CustomEvent("meridian-engine-policy", { detail: state }));
}

/** One subscription per WebView; all Meridian surfaces share the saved policy. */
export function initializeEngineRendering(): void {
  let revision = 0;
  on("engines.changed", event => { revision++; applyEngineState(event.data); });
  const mine = revision;
  void request({ method: "engines.get" }).then(state => {
    if (mine === revision && state) applyEngineState(state);
  }).catch(error => console.warn("Could not read rendering engines:", error));
}
