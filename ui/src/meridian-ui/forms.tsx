import { useId, useRef, useState, type ComponentProps, type ReactNode } from "react";
import { Input } from "../components/ui/input";
import { Button, type ButtonProps } from "../components/ui/button";
import { Dialog } from "../components/ui/dialog";
import { Icon } from "../components/Icon";

export type TextFieldProps = ComponentProps<"input"> & { label: string; help?: string; error?: string };
export function TextField({ label, help, error, id, className = "", ...props }: TextFieldProps) {
  const generated = useId();
  const fieldId = id ?? generated;
  const describedBy = [props["aria-describedby"], help && `${fieldId}-help`, error && `${fieldId}-error`].filter(Boolean).join(" ") || undefined;
  return <div className="muk-field">
    <label htmlFor={fieldId}>{label}</label>
    <Input {...props} id={fieldId} className={className} aria-invalid={error ? true : props["aria-invalid"]} aria-describedby={describedBy} />
    {help && <small id={`${fieldId}-help`}>{help}</small>}
    {error && <small id={`${fieldId}-error`} className="muk-field-error" role="alert">{error}</small>}
  </div>;
}
export function PasswordInput({ label, help, error, id, className = "", ...props }: Omit<TextFieldProps, "type">) {
  const generated = useId(), [visible, setVisible] = useState(false);
  const fieldId = id ?? generated;
  return <div className="muk-password">
    <TextField {...props} id={fieldId} label={label} help={help} error={error} className={className} type={visible ? "text" : "password"} />
    <Button size="small" disabled={props.disabled} aria-controls={fieldId} aria-pressed={visible} onClick={() => setVisible(value => !value)}>{visible ? "Hide password" : "Show password"}</Button>
  </div>;
}
export function LoadingButton({ loading = false, loadingLabel = "Working…", disabled, children, ...props }: ButtonProps & { loading?: boolean; loadingLabel?: string }) {
  return <Button {...props} disabled={disabled || loading} aria-busy={loading || undefined}>{loading ? loadingLabel : children}</Button>;
}
export function ConfirmDialog({ open, title, children, confirmLabel = "Confirm", danger = false, onConfirm, onClose }: {
  open: boolean; title: string; children: ReactNode; confirmLabel?: string; danger?: boolean;
  onConfirm: () => Promise<void>; onClose: () => void;
}) {
  // Unmount between confirmations so pending/error state cannot leak to the next action.
  return open ? <Confirmation title={title} confirmLabel={confirmLabel} danger={danger} onConfirm={onConfirm} onClose={onClose}>{children}</Confirmation> : null;
}
function Confirmation({ title, children, confirmLabel, danger, onConfirm, onClose }: Omit<ComponentProps<typeof ConfirmDialog>, "open">) {
  const id = useId(), pending = useRef(false);
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  async function confirm() {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError("");
    try { await onConfirm(); onClose(); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "The action failed. Try again."); }
    finally { pending.current = false; setBusy(false); }
  }
  return <Dialog open busy={busy} onClose={onClose} role="alertdialog" aria-labelledby={id} aria-describedby={`${id}-description`}>
    <h2 id={id}>{title}</h2><div id={`${id}-description`}>{children}</div>
    {error && <p role="alert" className="muk-field-error"><Icon name="info" /> {error}</p>}
    <div className="muk-dialog-actions"><Button data-autofocus disabled={busy} onClick={onClose}>Cancel</Button>
      <LoadingButton loading={busy} variant={danger ? "danger" : "primary"} onClick={() => void confirm()}>{confirmLabel}</LoadingButton></div>
  </Dialog>;
}
