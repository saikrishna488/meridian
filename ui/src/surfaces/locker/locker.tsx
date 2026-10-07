import { useEffect, useRef, useState, type FormEvent } from "react";
import { hello, request } from "../../lib/bridge";
import { errorMessage, mount } from "../../lib/react";
import { useDesktop } from "../../lib/desktop";
import { Clock } from "../../components/Clock";

function Locker() {
  const { desktop } = useDesktop();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [shake, setShake] = useState(false);
  const password = useRef<HTMLInputElement>(null);
  const submitting = useRef(false);
  useEffect(() => { void hello().catch(error => setMessage(errorMessage(error))); }, []);
  useEffect(() => { if (!busy) password.current?.focus(); }, [busy]);
  useEffect(() => {
    if (!shake) return;
    const timer = setTimeout(() => setShake(false), 600);
    return () => clearTimeout(timer);
  }, [shake]);
  async function unlock(event: FormEvent) {
    event.preventDefault();
    if (submitting.current) return;
    submitting.current = true;
    setMessage(""); setBusy(true);
    const secret = password.current?.value ?? "";
    if (password.current) password.current.value = "";
    try { await request({ method: "locker.unlock", params: { password: secret } }); }
    catch (error) {
      setBusy(false); setMessage(errorMessage(error, "Couldn’t unlock")); setShake(true);
    } finally { submitting.current = false; }
  }
  return <>
    <div className="wallpaper" aria-hidden="true" style={desktop ? { backgroundImage: `url("${desktop.wallpaper_uri}")` } : undefined} />
    <main className="lock">
      <Clock />
      <form className={`card${shake ? " shake" : ""}`} id="form" autoComplete="off" noValidate onSubmit={event => void unlock(event)}>
        <label className="field"><span className="field-label">Password</span>
          <input ref={password} id="password" name="password" type="password" autoFocus required disabled={busy} />
        </label>
        <button className="submit" id="submit" type="submit" disabled={busy}>{busy ? "Unlocking…" : "Unlock"}</button>
        <p className="message" id="message" role="alert" aria-live="assertive">{message}</p>
      </form>
    </main>
  </>;
}

mount(<Locker />);
