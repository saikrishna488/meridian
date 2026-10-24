import { useLayoutEffect, useRef, type ComponentProps } from "react";
import { useDialogFocus } from "../../lib/react";

// Reference counts preserve pre-existing inert state even for nested modals.
const isolated = new Map<HTMLElement, { count: number; inert: boolean }>();
let modalCount = 0;
let previousOverflow = "";

type Props = Omit<ComponentProps<"section">, "onClose" | "ref"> & {
  open: boolean; onClose: () => void; busy?: boolean; scrimClassName?: string;
};

/** Inline modal: cannot escape a native WebView or leave a closed portal behind. */
export function Dialog({ open, ...props }: Props) {
  return open ? <DialogContent {...props} /> : null;
}
function DialogContent({ onClose, busy = false, scrimClassName = "", className = "", children, role = "dialog", ...props }: Omit<Props, "open">) {
  const dialog = useRef<HTMLElement>(null), scrim = useRef<HTMLDivElement>(null);
  useDialogFocus(dialog, true, () => { if (!busy) onClose(); });
  useLayoutEffect(() => {
    // Isolate siblings up to body, preserving their pre-existing inert state.
    // This stays within this WebView; native focus is still owned by Rust.
    if (modalCount++ === 0) {
      previousOverflow = document.body.style.overflow;
      document.body.style.overflow = "hidden";
    }
    const siblings = new Set<HTMLElement>();
    let node: HTMLElement | null = scrim.current;
    while (node?.parentElement) {
      for (const sibling of node.parentElement.children) {
        if (sibling !== node && sibling instanceof HTMLElement) {
          siblings.add(sibling);
          const state = isolated.get(sibling) ?? { count: 0, inert: Boolean(sibling.inert) };
          state.count++; isolated.set(sibling, state); sibling.inert = true;
        }
      }
      node = node.parentElement;
      if (node === document.body) break;
    }
    return () => {
      if (--modalCount === 0) document.body.style.overflow = previousOverflow;
      for (const element of siblings) {
        const state = isolated.get(element);
        if (state && --state.count === 0) { element.inert = state.inert; isolated.delete(element); }
      }
    };
  }, []);
  return <div ref={scrim} className={`ui-dialog-scrim ${scrimClassName}`} onClick={event => event.stopPropagation()}>
    <section {...props} ref={dialog} role={role} aria-modal="true" aria-busy={busy || undefined}
      tabIndex={-1} className={`ui-dialog ${className}`}>{children}</section>
  </div>;
}
