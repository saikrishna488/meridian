import { useEffect, useState } from "react";

const dateFormat = new Intl.DateTimeFormat(undefined, { weekday: "long", month: "long", day: "numeric" });
const compactDateFormat = new Intl.DateTimeFormat(undefined, { weekday: "short", month: "short", day: "numeric" });

export function Clock({ compact = false, format = "system" }: { compact?: boolean; format?: string }) {
  const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit", ...(format === "12" ? {hour12:true} : format === "24" ? {hour12:false} : {}) });
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;
    const tick = () => {
      const date = new Date();
      setNow(date);
      timer = setTimeout(tick, 60_000 - (date.getSeconds() * 1000 + date.getMilliseconds()) + 50);
    };
    tick();
    return () => clearTimeout(timer);
  }, []);
  return <div className="when" title={now.toLocaleString(undefined, { dateStyle: "full", timeStyle: "short" })}>
    <time className="clock" id="clock" dateTime={now.toISOString()}>{timeFormat.format(now)}</time>
    <p className="date" id="date">{(compact ? compactDateFormat : dateFormat).format(now)}</p>
  </div>;
}
