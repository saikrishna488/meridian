import { Input } from "./ui/input";
import { SurfaceButton } from "./ui/button";
import { useState, type RefObject } from "react";
import { Icon } from "./Icon";

/** Uncontrolled input: the password stays in the DOM, never in React state. */
export function PasswordField({ inputRef, disabled, autoFocus = false }: { inputRef: RefObject<HTMLInputElement | null>; disabled: boolean; autoFocus?: boolean }) {
  const [visible, setVisible] = useState(false), [capsLock, setCapsLock] = useState(false);
  return <div className="field">
    <label className="field-label" htmlFor="password">Password</label>
    <div className="password-input"><Input ref={inputRef} id="password" name="password" aria-label="Password" type={visible ? "text" : "password"} autoFocus={autoFocus} required disabled={disabled} autoComplete="off" onKeyDown={event => setCapsLock(event.getModifierState("CapsLock"))} onKeyUp={event => setCapsLock(event.getModifierState("CapsLock"))} onBlur={() => setCapsLock(false)} />
      <SurfaceButton type="button" className="password-visibility" aria-label={visible ? "Hide password" : "Show password"} aria-pressed={visible} disabled={disabled} onClick={() => { setVisible(value => !value); inputRef.current?.focus(); }}><Icon name="eye" width="18" height="18" /></SurfaceButton>
    </div>{capsLock && <small className="caps-lock-hint" role="status">Caps Lock is on</small>}
  </div>;
}
