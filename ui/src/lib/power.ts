import { useEffect, useState } from "react";
import { request } from "./bridge";
import type { PowerState } from "./generated/PowerState";

export function usePowerStatus(): PowerState | null {
  const [state, setState] = useState<PowerState | null>(null);
  useEffect(() => {
    let alive = true, pending = false;
    async function refresh() {
      if (pending) return; pending = true;
      try { const result = await request({ method: "power.status" }); if (alive) setState(result); }
      catch (e) { console.debug("Power status unavailable", e); }
      finally { pending = false; }
    }
    void refresh(); const timer = setInterval(() => void refresh(), 30_000);
    window.addEventListener("focus", refresh);
    return () => { alive = false; clearInterval(timer); window.removeEventListener("focus", refresh); };
  }, []);
  return state;
}
