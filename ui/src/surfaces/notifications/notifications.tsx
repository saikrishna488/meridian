import { useEffect, useRef, useState } from "react";
import { on, request } from "../../lib/bridge";
import { errorMessage, mount, useDialogFocus, usePopover, useWindowKey } from "../../lib/react";
import { Icon } from "../../components/Icon";
import type { NotificationState } from "../../lib/generated/NotificationState";
function Notifications() {
  const [state, setState] = useState<NotificationState>({ available: true, items: [] }), [error, setError] = useState("");
  const revision = useRef(0);
  const report = (e: unknown) => setError(errorMessage(e));
  const { visible, menu, dismiss } = usePopover(async () => {
    setError(""); const mine = revision.current;
    const result = await request({ method: "notifications.read" });
    if (mine === revision.current) setState(result);
  }, () => menu.current?.focus(), report);
  useEffect(() => on("notifications.changed", e => { revision.current++; setState(e.data); }), []);
  useDialogFocus(menu, visible, () => { void dismiss().catch(report); });
  useWindowKey(e => { if (e.key === "Escape") { e.preventDefault(); void dismiss().catch(report); } });
  async function remove(id?: number) { try { setState(await (id == null ? request({ method: "notifications.clear" }) : request({ method: "notifications.dismiss", params: { id } }))); } catch (e) { report(e); } }
  async function activate(id: number, key: string) { try { setState(await request({ method: "notifications.action", params: { id, key } })); } catch (e) { report(e); } }
  return <><div className="scrim" onClick={() => void dismiss().catch(report)} /><section ref={menu} className={`menu notification-center${visible ? "" : " hidden"}`} role="dialog" aria-label="Notifications" tabIndex={-1}>
    <header><h1>Notifications</h1>{state.items.length > 0 && <button onClick={() => void remove()}>Clear All</button>}</header>
    {!state.available && <p role="status" className="notification-status">Another notification service is active, or the session bus is unavailable.</p>}
    {state.items.length ? <div className="notification-list">{state.items.map(item => <article key={item.id} className="notification"><div className="notification-meta"><span>{item.app || "System"}</span><time dateTime={new Date(item.time).toISOString()}>{new Date(item.time).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</time><button aria-label={`Dismiss ${item.summary}`} onClick={() => void remove(item.id)}><Icon name="close" width="13" height="13" /></button></div><h2>{item.summary}</h2>{item.body && <p>{item.body}</p>}{item.actions.length > 0 && <div className="notification-actions">{item.actions.map(([key, label]) => <button key={key} onClick={() => void activate(item.id, key)}>{label || "Open"}</button>)}</div>}</article>)}</div> : <p className="notification-empty">No notifications</p>}
    {error && <p role="alert" className="notification-status">{error}</p>}
  </section></>;
}
mount(<Notifications />);
