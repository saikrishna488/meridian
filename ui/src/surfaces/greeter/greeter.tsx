import { useEffect, useRef, useState, type FormEvent } from "react";
import { hello, request } from "../../lib/bridge";
import { errorMessage, mount } from "../../lib/react";
import { Clock } from "../../components/Clock";
import { ConfirmButton } from "../../components/ConfirmButton";
import type { GreeterInfo } from "../../lib/generated/GreeterInfo";

function Greeter() {
  const [info, setInfo] = useState<GreeterInfo | null>(null);
  const [username, setUsername] = useState("");
  const [session, setSession] = useState("");
  const [busy, setBusy] = useState(true);
  const [label, setLabel] = useState("Log in");
  const [message, setMessage] = useState("");
  const [messageKind, setMessageKind] = useState("error");
  const [shake, setShake] = useState(false);
  const form = useRef<HTMLFormElement>(null);
  const userInput = useRef<HTMLInputElement>(null);
  // Password stays in the field; it is never retained in React state.
  const password = useRef<HTMLInputElement>(null);
  const submitting = useRef(false);
  const initialFocus = useRef<"username" | "password" | null>(null);
  useEffect(() => {
    let alive = true;
    void (async () => {
      await hello();
      const data = await request({ method: "greeter.info" });
      if (!alive) return;
      setInfo(data);
      setSession(data.sessions.find(item => item.id === data.last_session)?.id ?? data.sessions[0]?.id ?? "");
      setUsername(data.last_user ?? "");
      if (!data.sessions.length) { setMessage("No desktop sessions are installed"); return; }
      initialFocus.current = data.last_user ? "password" : "username";
      setBusy(false);
    })().catch(error => {
      console.error("meridian greeter:", error);
      if (alive) setMessage("The login screen couldn’t start. Switch to a text console (Ctrl+Alt+F3) to log in.");
    });
    return () => { alive = false; };
  }, []);
  useEffect(() => {
    if (!busy && initialFocus.current) {
      (initialFocus.current === "password" ? password : userInput).current?.focus();
      initialFocus.current = null;
    }
  }, [busy]);
  useEffect(() => {
    if (!shake) return;
    const timer = setTimeout(() => setShake(false), 600);
    return () => clearTimeout(timer);
  }, [shake]);
  async function login(event: FormEvent) {
    event.preventDefault();
    if (busy || submitting.current) return;
    if (!username.trim()) { setMessage("Enter your username"); userInput.current?.focus(); return; }
    submitting.current = true;
    setMessage(""); setMessageKind("error"); setBusy(true); setLabel("Logging in…");
    const secret = password.current?.value ?? "";
    if (password.current) password.current.value = "";
    try {
      await request({ method: "greeter.login", params: { username: username.trim(), password: secret, session } });
      setLabel("Starting…"); setMessageKind("info");
      setMessage(`Starting ${info?.sessions.find(item => item.id === session)?.name ?? "session"}…`);
    } catch (error) {
      setBusy(false); setLabel("Log in"); setMessage(errorMessage(error, "Login failed"));
      setShake(true); initialFocus.current = "password";
    } finally { submitting.current = false; }
  }
  function report(error: unknown) { setMessageKind("error"); setMessage(errorMessage(error, "Couldn’t do that")); }
  return <>
    <div className="wallpaper" aria-hidden="true" />
    <p className="host" id="host">{info?.hostname}</p>
    <main className="login">
      <Clock />
      <form ref={form} className={`card${shake ? " shake" : ""}`} id="form" autoComplete="off" noValidate onSubmit={event => void login(event)}>
        <label className="field"><span className="field-label">Username</span>
          <input ref={userInput} id="username" name="username" type="text" autoCapitalize="off" spellCheck={false} required
            disabled={busy} value={username} onChange={event => setUsername(event.target.value)} />
        </label>
        <label className="field"><span className="field-label">Password</span>
          <input ref={password} id="password" name="password" type="password" required disabled={busy} />
        </label>
        <label className="field"><span className="field-label">Session</span>
          <select id="session" name="session" disabled={busy} value={session} onChange={event => setSession(event.target.value)}>
            {info?.sessions.map(item => <option key={item.id} value={item.id}>{item.name}</option>)}
          </select>
        </label>
        <button className="submit" id="submit" type="submit" disabled={busy}>{label}</button>
        <p className={`message${messageKind === "info" ? " info" : ""}`} id="message" role="alert" aria-live="assertive">{message}</p>
      </form>
    </main>
    <nav className="power" aria-label="Power">
      <ConfirmButton id="reboot" className="power-button" label="Restart" report={report}
        action={async () => { await request({ method: "greeter.power", params: { action: "reboot" } }); setMessageKind("info"); setMessage("Restarting…"); }} />
      <ConfirmButton id="poweroff" className="power-button" label="Shut down" report={report}
        action={async () => { await request({ method: "greeter.power", params: { action: "power-off" } }); setMessageKind("info"); setMessage("Shutting down…"); }} />
    </nav>
  </>;
}

mount(<Greeter />);
