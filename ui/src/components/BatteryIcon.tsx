export function BatteryIcon({ percent, charging = false }: { percent: number | null; charging?: boolean }) {
  const fill = percent === null ? 0 : Math.max(0, Math.min(100, percent)) / 100 * 13;
  return <svg className="ui-icon battery-icon" width="23" height="20" viewBox="0 0 26 22" fill="none" aria-hidden="true"><rect x="2" y="5" width="20" height="12" rx="3" stroke="currentColor" strokeWidth="1.5" /><path d="M24 9v4" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />{charging ? <path d="m13 5-5 7h4l-1 5 6-8h-4l1-4Z" fill="currentColor" /> : <rect x="5" y="8" width={fill} height="6" rx="1" fill="currentColor" />}</svg>;
}
