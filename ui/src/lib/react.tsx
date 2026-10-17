import { useEffect, useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import { createRoot } from "react-dom/client";
import { afterTransition, BridgeError, cssDuration, hello, nextFrame, on, request } from "./bridge";
import { applyAppearance } from "./appearance";

export function mount(element: ReactNode): void {
  const root = document.getElementById("root");
  if (!root) throw new Error("React root missing");
  // Meridian provides its own context actions. Suppress WebKit's native HTML
  // context menu globally while allowing React contextmenu handlers to run.
  document.addEventListener("contextmenu", event => event.preventDefault(), true);
  let appearanceRevision = 0;
  on("desktop.changed", event => { appearanceRevision++; applyAppearance(event.data.appearance); });
  // Each app window is its own WebView. Read the persisted value so newly
  // opened windows match the rest of the desktop before waiting for updates.
  void request({ method: "desktop.get" }).then(state => {
    if (appearanceRevision === 0 && state) applyAppearance(state.appearance);
  }).catch(error => console.warn("Could not read Meridian appearance:", error));
  createRoot(root).render(element);
}

export function errorMessage(error: unknown, fallback = "Something went wrong"): string {
  console.error("meridian UI:", error);
  return error instanceof BridgeError ? error.message : fallback;
}

// Stable callbacks let host subscriptions read the latest committed React state.
export function useCallbackRef<T extends (...args: never[]) => unknown>(callback: T): T {
  const ref = useRef(callback);
  useLayoutEffect(() => { ref.current = callback; });
  return useRef(((...args: Parameters<T>) => ref.current(...args)) as T).current;
}

export function useWindowKey(callback: (event: KeyboardEvent) => void): void {
  const handler = useCallbackRef(callback);
  useEffect(() => {
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [handler]);
}

export function usePopover(
  load: () => Promise<void>,
  focus: () => void,
  report: (error: unknown) => void,
): { visible: boolean; menu: RefObject<HTMLElement | null>; dismiss: () => Promise<void> } {
  const [visible, setVisible] = useState(false);
  const menu = useRef<HTMLElement>(null);
  const open = useRef(false);
  const generation = useRef(0);
  const loadLatest = useCallbackRef(load);
  const focusLatest = useCallbackRef(focus);
  const reportLatest = useCallbackRef(report);
  const dismiss = useCallbackRef(async () => {
    if (!open.current) return;
    open.current = false;
    const mine = ++generation.current;
    setVisible(false);
    await nextFrame();
    if (menu.current) await afterTransition(menu.current, cssDuration("--dur-fast"));
    if (mine === generation.current) await request({ method: "popover.close" });
  });
  useEffect(() => {
    const offShow = on("popover.shown", () => {
      open.current = true;
      const mine = ++generation.current;
      void (async () => {
        try { await loadLatest(); } catch (error) { reportLatest(error); }
        await nextFrame();
        if (mine !== generation.current || !open.current) return;
        setVisible(true);
        await nextFrame();
        if (mine === generation.current) focusLatest();
      })();
    });
    const offDismiss = on("popover.dismiss", () => { void dismiss().catch(reportLatest); });
    void hello().catch(reportLatest);
    return () => { offShow(); offDismiss(); open.current = false; generation.current++; };
  }, [loadLatest, focusLatest, reportLatest, dismiss]);
  return { visible, menu, dismiss };
}

/** Keep a modal's keyboard focus inside it and restore focus when it closes. */
export function useDialogFocus(ref: RefObject<HTMLElement | null>, open: boolean, close: () => void) {
  const dismiss = useCallbackRef(close);
  useEffect(() => {
    const element = ref.current;
    if (!open || !element) return;
    const previous = document.activeElement as HTMLElement | null;
    const controls = () => Array.from(element.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), a[href], [tabindex="0"]'));
    (element.querySelector<HTMLElement>('[autofocus], [data-autofocus]') ?? controls()[0])?.focus();
    const keydown = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); dismiss(); }
      if (event.key === "Tab") {
        const items = controls(); if (!items.length) { event.preventDefault(); return; }
        const index = items.indexOf(document.activeElement as HTMLElement);
        if (index < 0 || (!event.shiftKey && index === items.length - 1) || (event.shiftKey && index === 0)) {
          event.preventDefault(); (event.shiftKey ? items.at(-1) : items[0])?.focus();
        }
      }
    };
    element.addEventListener("keydown", keydown);
    return () => { element.removeEventListener("keydown", keydown); if (previous?.isConnected) previous.focus(); };
  }, [ref, open, dismiss]);
}

/** Arrow navigation for an open context menu, including keyboard dismissal. */
export function useMenuFocus(ref: RefObject<HTMLElement | null>, open: boolean, close: () => void) {
  const dismiss = useCallbackRef(close);
  useEffect(() => {
    const element = ref.current;
    if (!open || !element) return;
    const previous = document.activeElement as HTMLElement | null;
    const controls = () => Array.from(element.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
    controls()[0]?.focus();
    const keydown = (event: KeyboardEvent) => {
      const items = controls(), index = items.indexOf(document.activeElement as HTMLButtonElement);
      if (event.key === "Escape" || event.key === "Tab") { event.preventDefault(); event.stopPropagation(); dismiss(); }
      if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key) && items.length) {
        event.preventDefault(); event.stopPropagation();
        items[event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : (index + (event.key === "ArrowDown" ? 1 : items.length - 1)) % items.length]?.focus();
      }
    };
    element.addEventListener("keydown", keydown);
    return () => { element.removeEventListener("keydown", keydown); if (previous?.isConnected) previous.focus(); };
  }, [ref, open, dismiss]);
}
