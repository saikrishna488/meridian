import { Button } from "../../components/ui/button";
import { Select } from "../../components/Select";
import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import { errorMessage, useDialogFocus } from "../../lib/react";
import type { UserInfo } from "../../lib/generated/UserInfo";
type Choice = { action: string; user?: UserInfo };
export function Users() {
  const [users, setUsers] = useState<UserInfo[]>([]), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const [choice, setChoice] = useState<Choice | null>(null), [name, setName] = useState(""), [realName, setRealName] = useState("");
  const [admin, setAdmin] = useState(false), [password, setPassword] = useState(""), [confirm, setConfirm] = useState(""), [message, setMessage] = useState("");
  const dialog = useRef<HTMLFormElement>(null), pending = useRef(false);
  function close() { if (!pending.current) { setChoice(null); setPassword(""); setConfirm(""); } }
  useDialogFocus(dialog, !!choice, close);
  async function load() { setUsers(await request({ method: "settings.users" }) ?? []); }
  async function refresh() {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError("");
    try { await load(); } catch (e) { setError(errorMessage(e)); } finally { pending.current = false; setBusy(false); }
  }
  useEffect(() => { void refresh(); }, []);
  function open(action: string, user?: UserInfo) { setPassword(""); setConfirm(""); setName(""); setRealName(""); setAdmin(false); setError(""); setChoice({ action, user }); }
  const needsPassword = choice?.action === "create" || choice?.action === "password";
  async function apply() {
    if (!choice || pending.current) return;
    if (needsPassword && (password.length < 8 || password !== confirm)) { setError("Use at least 8 characters and matching passwords."); return; }
    pending.current = true; setBusy(true); setError(""); setMessage("");
    try {
      if (choice.action === "create") await request({ method: "settings.user_create", params: { name, real_name: realName, administrator: admin, password } });
      else if (choice.user) await request({ method: "settings.user_action", params: { uid: choice.user.uid, action: choice.action, password: choice.action === "password" ? password : null } });
      setMessage(choice.action === "create" ? "Account created." : "Account updated."); setChoice(null);
    } catch (e) { setError(errorMessage(e)); }
    finally { setPassword(""); setConfirm(""); await load().catch(e => setError(errorMessage(e))); pending.current = false; setBusy(false); }
  }
  const titles: Record<string, string> = { create: "Add user", delete: "Delete account", password: "Set password", lock: "Lock account", unlock: "Unlock account", administrator: "Make administrator", standard: "Make standard user" };
  const admins = users.filter(u => u.administrator && !u.locked).length;
  return <>
    <p className="section-caption">Manage local login accounts. Changes require authorization. Deleting an account keeps its home folder and files.</p>
    <div className="settings-actions"><Button disabled={busy} onClick={() => void refresh()}>Refresh</Button><Button variant="primary" className="primary" disabled={busy} onClick={() => open("create")}>Add user…</Button></div>
    <p className="section-caption" role="status">{busy ? "Working…" : message}</p>
    {error && !choice && <p role="alert" className="settings-error">{error}</p>}
    <div className="settings-card">{users.map(user => {
      const protectedAccount = user.current || (user.administrator && !user.locked && admins <= 1);
      return <div className="settings-row user-management-row" key={user.uid}><div className="management-details"><strong>{user.real_name || user.name}{user.current && " (you)"}</strong><small className="row-detail">{user.name} · {user.administrator ? "Administrator" : "Standard user"} · {user.locked ? "Login locked" : "Login enabled"}</small><small className="row-detail management-path">{user.home}</small></div><div className="settings-actions user-management-actions"><Button disabled={busy} onClick={() => open("password", user)}>Password…</Button><Button disabled={busy || protectedAccount} onClick={() => open(user.administrator ? "standard" : "administrator", user)}>{user.administrator ? "Make standard…" : "Make admin…"}</Button><Button disabled={busy || (protectedAccount && !user.locked)} onClick={() => open(user.locked ? "unlock" : "lock", user)}>{user.locked ? "Unlock…" : "Lock…"}</Button><Button disabled={busy || protectedAccount} onClick={() => open("delete", user)}>Delete…</Button></div></div>;
    })}{!busy && !users.length && <p className="device-empty">No local accounts available. AccountsService must be installed and running.</p>}</div>
    {choice && <div className="device-dialog-scrim"><form ref={dialog} className="device-dialog management-form" role="dialog" aria-modal="true" aria-label={titles[choice.action]} onSubmit={e => { e.preventDefault(); void apply(); }}><h3>{titles[choice.action]}{choice.user ? `: ${choice.user.name}` : ""}</h3>
      {choice.action === "create" && <><label>Full name<input aria-label="Full name" maxLength={256} disabled={busy} value={realName} onChange={e => setRealName(e.target.value)} autoComplete="off" /></label><label>Username<input aria-label="Username" required pattern="[a-z_][a-z0-9_-]{0,31}" maxLength={32} disabled={busy} value={name} onChange={e => setName(e.target.value)} autoComplete="off" /></label><label>Account type<Select aria-label="Account type" disabled={busy} value={admin ? "admin" : "standard"} onChange={e => setAdmin(e.target.value === "admin")}><option value="standard">Standard user</option><option value="admin">Administrator</option></Select></label></>}
      {needsPassword && <><label>New password<input aria-label="New password" type="password" required minLength={8} maxLength={1024} value={password} onChange={e => setPassword(e.target.value)} disabled={busy} autoComplete="new-password" /></label><label>Confirm password<input aria-label="Confirm password" type="password" required minLength={8} maxLength={1024} value={confirm} onChange={e => setConfirm(e.target.value)} disabled={busy} autoComplete="new-password" /></label><small>Use at least 8 characters. Setting a password also unlocks the account.</small></>}
      {choice.action === "delete" && <p>This removes the login account for {choice.user?.name}. Its home folder and files are kept. This cannot be undone from Settings.</p>}
      {choice.action === "lock" && <p>Prevent new logins for this account. Existing sessions may continue running.</p>}
      {choice.action === "unlock" && <p>Allow this account to log in again with its existing password.</p>}
      {(choice.action === "administrator" || choice.action === "standard") && <p>{choice.action === "administrator" ? "Allow this account to authorize system changes." : "Remove administrator privileges from this account."} Existing sessions may need to sign out for group changes to take effect.</p>}
      {error && <p role="alert" className="settings-error">{error}</p>}
      <div className="settings-actions"><Button type="button" disabled={busy} onClick={close}>Cancel</Button><Button type="submit" className="primary" disabled={busy}>{busy ? "Authorizing…" : titles[choice.action]}</Button></div>
    </form></div>}
  </>;
}
