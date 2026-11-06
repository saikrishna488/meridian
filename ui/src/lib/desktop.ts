import { useLayoutEffect, useState } from "react";
import { on, request } from "./bridge";
import type { DesktopState } from "./generated/DesktopState";
export function useDesktop() {
  const [attempt, setAttempt] = useState(0);
  const [error, setError] = useState("");
  const [desktop, setDesktop] = useState<DesktopState | null>(null);
  useLayoutEffect(() => {
    let alive = true, revision = 0;
    setError("");
    const off = on("desktop.changed", event => { revision++; setDesktop(event.data); });
    const initial = revision;
    void request({ method: "desktop.get" }).then(value => { if (alive && initial === revision) setDesktop(value); }).catch(cause => { if (alive) setError(cause instanceof Error ? cause.message : "Desktop preferences are unavailable."); });
    return () => { alive = false; off(); };
  }, [attempt]);
  return { desktop, setDesktop, error, refresh: () => setAttempt(value => value + 1) };
}
