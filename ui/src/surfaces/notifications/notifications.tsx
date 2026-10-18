import { Toggle } from "../../components/Toggle";
import { useEffect, useRef, useState } from "react";
import { on, request } from "../../lib/bridge";
import { errorMessage, mount, useCallbackRef, useDialogFocus, usePopover, useWindowKey } from "../../lib/react";
import { Icon } from "../../components/Icon";
import { AppIcon } from "../../components/AppIcon";
import type { NotificationState } from "../../lib/generated/NotificationState";
function Notifications() {
  const [state, setState] = useState<NotificationState>({ available: true, items: [], do_not_disturb: false }), [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const revision = useRef(0), pending = useRef(false), reading = useRef(false);
  const report = (e: unknown) => setError(errorMessage(e));
  const { visible, menu, dismiss } = usePopover(async () => {
    setError(""); const mine = revision.current;
    const result = await request({ method: "notifications.read" });
    if (mine === revision.current) setState(result);
  }, () => menu.current?.focus(), report);
  useEffect(() => on("notifications.changed", e => { revision.current++; setState(e.data); }), []);
  const markRead = useCallbackRef(async () => {
    if (reading.current) return;
    reading.current = true; const mine = revision.current;
    try { const result = await request({ method: "notifications.read" }); if (mine === revision.current) setState(result); }
    catch (e) { report(e); } finally { reading.current = false; }
  });
  useEffect(() => { if (visible && state.items.some(item => item.unread)) void markRead(); }, [visible, state.items, markRead]);
  useDialogFocus(menu, visible, () => { void dismiss().catch(report); });
  useWindowKey(e => { if (e.key === "Escape") { e.preventDefault(); void dismiss().catch(report); } });
  async function remove(id?: number, key?: string) {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError(""); const mine = revision.current;
    try {
      const result = await (id == null ? request({ method: "notifications.clear" }) : key === undefined ? request({ method: "notifications.dismiss", params: { id } }) : request({ method: "notifications.action", params: { id, key } }));
      if (mine === revision.current) setState(result);
    } catch (e) { report(e); } finally { pending.current = false; setBusy(false); }
  }
  const today = new Date().toDateString();
  const grouped = new Map<string, { name: string; today: boolean; items: NotificationState["items"] }>();
  for (const item of state.items) {
    const isToday = new Date(item.time).toDateString() === today;
    const name = `${isToday ? "Today" : "Earlier"} · ${item.app || "System"}`;
    if (!grouped.has(name)) grouped.set(name, { name, today: isToday, items: [] });
    grouped.get(name)!.items.push(item);
  }
  const groups = [...grouped.values()];
  async function doNotDisturb(enabled: boolean) {
    if (pending.current) return; pending.current = true; setBusy(true); setError(""); const mine = revision.current;
    try { const result = await request({ method: "notifications.do_not_disturb", params: { enabled } }); if (mine === revision.current) setState(result); }
    catch (error) { report(error); } finally { pending.current = false; setBusy(false); }
  }
  return <><div className="scrim" onClick={() => void dismiss().catch(report)} /><section ref={menu} className={`menu notification-center${visible ? "" : " hidden"}`} role="dialog" aria-label="Notifications" tabIndex={-1}>
    <header className="notification-heading"><div><span className="notification-eyebrow">ACTIVITY</span><h1>Notifications</h1></div>{state.items.length > 0 && <button className="notification-clear" disabled={busy} onClick={() => void remove()}>Clear all</button>}</header>
    <div className="notification-dnd"><div><strong>Do Not Disturb</strong><p className="notification-status">Hide new-message badges. Messages stay in this session’s history.</p></div><Toggle label="Do Not Disturb" checked={state.do_not_disturb === true} disabled={busy} onChange={enabled => void doNotDisturb(enabled)} /></div>
    {!state.available && <p role="status" className="notification-status">Another notification service is active, or the session bus is unavailable.</p>}
    {state.items.length ? <div className="notification-list">{groups.filter(group => group.items.length).map(group => <section className="notification-group" key={group.name}><h2 className="notification-group-title">{group.name}<span>{group.items.length}</span></h2>{group.items.map(item => <article key={item.id} className="notification"><div className="notification-meta"><AppIcon name={item.app || "System"} /><span>{item.app || "System"}</span><time title={new Date(item.time).toLocaleString()} dateTime={new Date(item.time).toISOString()}>{group.today ? new Date(item.time).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : new Date(item.time).toLocaleDateString([], { month: "short", day: "numeric" })}</time><button disabled={busy} aria-label={`Dismiss ${item.summary}`} onClick={() => void remove(item.id)}><Icon name="close" width="13" height="13" /></button></div><h3>{item.summary}</h3>{item.body && <p>{item.body}</p>}{item.actions.length > 0 && <div className="notification-actions">{item.actions.map(([key, label]) => <button key={key} disabled={busy} onClick={() => void remove(item.id, key)}>{label || "Open"}</button>)}</div>}</article>)}</section>)}</div> : <div className="notification-empty"><span><Icon name="bell" width="28" height="28" /></span><h2>No notifications</h2><p>New messages and app alerts will appear here.</p></div>}
    {error && <p role="alert" className="notification-status notification-error">{error}</p>}
  </section></>;
}
mount(<Notifications />);
