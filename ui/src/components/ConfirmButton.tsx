import { useEffect, useRef, useState } from "react";

export function ConfirmButton({ id, className, label, action, report, role, disabled = false }: {
  id: string; className: string; label: string; role?: "menuitem"; disabled?: boolean; action: () => Promise<void>; report: (error: unknown) => void;
}) {
  const [armed, setArmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);
  async function click() {
    if (busy || disabled) return;
    if (!armed) {
      setArmed(true);
      timer.current = setTimeout(() => setArmed(false), 3000);
      return;
    }
    clearTimeout(timer.current);
    setBusy(true);
    try { await action(); } catch (error) { report(error); }
    finally { setBusy(false); setArmed(false); }
  }
  return <button id={id} role={role} type="button" className={`${className}${armed ? " armed" : ""}`}
    disabled={busy || disabled} onClick={() => void click()}>{armed ? `${label}?` : label}</button>;
}
