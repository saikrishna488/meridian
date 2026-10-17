import type { SVGProps } from "react";
const paths = {
  search: <><circle cx="10.5" cy="10.5" r="6.5" /><path d="m16 16 4 4" /></>,
  eye: <><path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z" /><circle cx="12" cy="12" r="3" /></>,
  close: <path d="m6 6 12 12M18 6 6 18" />,
  back: <path d="m14 5-7 7 7 7" />,
  forward: <path d="m10 5 7 7-7 7" />,
  up: <path d="m5 12 7-7 7 7M12 5v15" />,
  grid: <><rect x="4" y="4" width="6" height="6" rx="1" /><rect x="14" y="4" width="6" height="6" rx="1" /><rect x="4" y="14" width="6" height="6" rx="1" /><rect x="14" y="14" width="6" height="6" rx="1" /></>,
  list: <path d="M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01" />,
  refresh: <><path d="M20 7v5h-5M4 17v-5h5" /><path d="M6 7a7 7 0 0 1 12-1l2 6M4 12l2 6a7 7 0 0 0 12-1" /></>,
  plus: <path d="M12 5v14M5 12h14" />,
  folder: <path d="M3 7a2 2 0 0 1 2-2h5l3 3h6a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />,
  home: <><path d="m3 11 9-8 9 8M5 10v11h14V10" /><path d="M9 21v-7h6v7" /></>,
  settings: <><path d="m9 3-.6 3-2.7 1.5L3 7l-1 4 2.3 2-1 3L6 19l3-1 3 3 3-3 3 1 2.7-3-1-3L22 11l-1-4-2.7.5L15.6 6 15 3Z" /><circle cx="12" cy="12" r="3" /></>,
  image: <><rect x="3" y="3" width="18" height="18" rx="3" /><circle cx="8" cy="8" r="1.5" /><path d="m4 18 6-6 4 4 3-3 4 5" /></>,
  appearance: <><circle cx="12" cy="12" r="9" /><path d="M12 3v18" /><path d="M12 3a9 9 0 0 1 0 18Z" fill="currentColor" stroke="none" /></>,
  monitor: <><rect x="3" y="4" width="18" height="13" rx="2" /><path d="M12 17v4M8 21h8" /></>,
  wifi: <><path d="M2 8a16 16 0 0 1 20 0M5 12a11 11 0 0 1 14 0M8 16a6 6 0 0 1 8 0" /><circle cx="12" cy="20" r="1" fill="currentColor" /></>,
  bluetooth: <path d="m6 7 12 10-6 5V2l6 5L6 17" />,
  sound: <><path d="M4 9h4l5-4v14l-5-4H4ZM17 8a6 6 0 0 1 0 8M20 5a10 10 0 0 1 0 14" /></>,
  terminal: <><rect x="3" y="4" width="18" height="16" rx="3" /><path d="m7 9 3 3-3 3M13 15h4" /></>,
  keyboard: <><rect x="2" y="5" width="20" height="14" rx="3" /><path d="M6 9h.01M10 9h.01M14 9h.01M18 9h.01M6 12h.01M10 12h.01M14 12h.01M18 12h.01M7 16h10" /></>,
  check: <path d="m5 12 4 4L19 6" />,
  pin: <><path d="m9 3 12 12-3 3-5-2-4 4-5-5 4-4-2-5ZM7 17l-4 4" /></>,
  info: <><circle cx="12" cy="12" r="9" /><path d="M12 11v6M12 7h.01" /></>,
  lock: <><rect x="5" y="10" width="14" height="11" rx="2" /><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3" /></>,
  share: <><circle cx="18" cy="5" r="3" /><circle cx="6" cy="12" r="3" /><circle cx="18" cy="19" r="3" /><path d="m9 10 6-4M9 14l6 4" /></>,
} as const;
export type IconName = keyof typeof paths;
export function Icon({ name, ...props }: SVGProps<SVGSVGElement> & { name: IconName }) {
  return <svg className="ui-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" {...props}>{paths[name]}</svg>;
}
