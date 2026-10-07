import { useEffect, useState } from "react";
import { on, request } from "./bridge";
import type { DesktopState } from "./generated/DesktopState";
export function useDesktop() {
  const [desktop, setDesktop] = useState<DesktopState | null>(null);
  useEffect(() => {
    const off = on("desktop.changed", event => setDesktop(event.data));
    void request({ method: "desktop.get" }).then(setDesktop).catch(console.error);
    return off;
  }, []);
  return { desktop, setDesktop };
}
