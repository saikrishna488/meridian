import { Dialog } from "./ui/dialog";
import { Button } from "./ui/button";
import { useEffect, useState } from "react";
import { request } from "../lib/bridge";
import { errorMessage } from "../lib/react";
import type { UninstallPlan } from "../lib/generated/UninstallPlan";

export function UninstallDialog({ id, close }: { id: string; close: () => void }) {
  const [plan, setPlan] = useState<UninstallPlan | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let alive = true;
    setPlan(null); setError("");
    void request({ method: "apps.uninstall_plan", params: { id } }).then(result => { if (alive) setPlan(result); }).catch(e => { if (alive) setError(errorMessage(e)); });
    return () => { alive = false; };
  }, [id]);
  async function uninstall() {
    if (!plan || busy) return;
    setBusy(true); setError("");
    try { await request({ method: "apps.uninstall", params: { id, target: plan.target } }); close(); }
    catch (e) { setError(errorMessage(e)); }
    finally { setBusy(false); }
  }
  return <Dialog open onClose={close} busy={busy} scrimClassName="uninstall-scrim"
    className="uninstall-dialog" role="alertdialog" aria-labelledby="uninstall-title" aria-describedby="uninstall-detail">
      <div className="uninstall-symbol" aria-hidden="true">⌫</div>
      <h2 id="uninstall-title">{plan ? `Uninstall ${plan.name}?` : "Uninstall application"}</h2>
      <p id="uninstall-detail">{plan?.detail ?? (error ? "This app could not be prepared for removal." : "Checking the installed application…")}</p>
      {error && <p role="alert" className="uninstall-error">{error}</p>}
      {busy && <p role="status">Uninstalling… Complete any authentication prompt that appears.</p>}
      <div className="uninstall-actions"><Button data-autofocus disabled={busy} onClick={close}>Cancel</Button><Button variant="danger" className="danger" disabled={!plan || busy} onClick={() => void uninstall()}>Uninstall</Button></div>
  </Dialog>;
}
