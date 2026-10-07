import { useEffect, useRef, useState } from "react";
import { request } from "../lib/bridge";
import { errorMessage } from "../lib/react";
import type { UninstallPlan } from "../lib/generated/UninstallPlan";

export function UninstallDialog({ id, close }: { id: string; close: () => void }) {
  const [plan, setPlan] = useState<UninstallPlan | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const cancel = useRef<HTMLButtonElement>(null);
  const dialog = useRef<HTMLElement>(null);
  useEffect(() => {
    let alive = true;
    setPlan(null); setError("");
    void request({ method: "apps.uninstall_plan", params: { id } }).then(result => { if (alive) setPlan(result); }).catch(e => { if (alive) setError(errorMessage(e)); });
    cancel.current?.focus();
    return () => { alive = false; };
  }, [id]);
  async function uninstall() {
    if (!plan || busy) return;
    setBusy(true); setError("");
    try { await request({ method: "apps.uninstall", params: { id, target: plan.target } }); close(); }
    catch (e) { setError(errorMessage(e)); }
    finally { setBusy(false); }
  }
  return <div className="uninstall-scrim" onClick={e => e.stopPropagation()}>
    <section ref={dialog} className="uninstall-dialog" role="alertdialog" aria-modal="true" aria-labelledby="uninstall-title" aria-describedby="uninstall-detail"
      onKeyDown={e => {
        e.stopPropagation();
        if (e.key === "Escape" && !busy) { e.preventDefault(); close(); }
        if (e.key === "Tab") {
          const buttons = [...dialog.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []];
          const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
          e.preventDefault(); buttons[(index + (e.shiftKey ? -1 : 1) + buttons.length) % buttons.length]?.focus();
        }
      }}>
      <div className="uninstall-symbol" aria-hidden="true">⌫</div>
      <h2 id="uninstall-title">{plan ? `Uninstall ${plan.name}?` : "Uninstall application"}</h2>
      <p id="uninstall-detail">{plan?.detail ?? (error ? "This app could not be prepared for removal." : "Checking the installed application…")}</p>
      {error && <p role="alert" className="uninstall-error">{error}</p>}
      {busy && <p role="status">Uninstalling… Complete any authentication prompt that appears.</p>}
      <div className="uninstall-actions"><button ref={cancel} disabled={busy} onClick={close}>Cancel</button><button className="danger" disabled={!plan || busy} onClick={() => void uninstall()}>Uninstall</button></div>
    </section>
  </div>;
}
