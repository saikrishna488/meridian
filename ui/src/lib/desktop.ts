import { useLayoutEffect, useState } from "react";
import { on, request } from "./bridge";
import type { DesktopState } from "./generated/DesktopState";
export function useDesktop() {
  const [desktop, setDesktop] = useState<DesktopState | null>(null);
  useLayoutEffect(() => {
    let alive = true, revision = 0;
    const off = on("desktop.changed", event => { revision++; setDesktop(event.data); });
    const initial = revision;
    void request({ method: "desktop.get" }).then(value => { if (alive && initial === revision) setDesktop(value); }).catch(console.error);
    return () => { alive = false; off(); };
  }, []);
  return { desktop, setDesktop };
}
