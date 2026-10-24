import type { SVGProps } from "react";
/** Meridian: a horizon intersecting a rising, folded M. */
export function MeridianLogo(props: SVGProps<SVGSVGElement>) {
  return <svg className="meridian-logo" viewBox="0 0 32 32" width="26" height="26" aria-hidden="true" {...props}>
    <defs>
      <radialGradient id="meridian-orb" cx="28%" cy="22%" r="78%">
        <stop stopColor="#d6e8ff" />
        <stop offset=".28" stopColor="#78b5ff" />
        <stop offset=".68" stopColor="#4285f4" />
        <stop offset="1" stopColor="#6554c8" />
      </radialGradient>
      <linearGradient id="meridian-orb-shine" x1=".15" y1="0" x2=".8" y2="1">
        <stop stopColor="#fff" stopOpacity=".72" />
        <stop offset="1" stopColor="#fff" stopOpacity="0" />
      </linearGradient>
    </defs>
    <circle cx="16" cy="16" r="15" fill="url(#meridian-orb)" />
    <path d="M7.5 10.5c2-4.3 7-6.6 11.6-5.5 2.1.5 3.9 1.6 5.3 3.2-4.5-.8-9.1.1-12.4 2.5-2 1.4-3.6 3.3-4.7 5.5-.7-1.7-.6-3.8.2-5.7Z" fill="url(#meridian-orb-shine)" />
    <circle cx="16" cy="16" r="14.5" fill="none" stroke="#fff" strokeOpacity=".38" />
  </svg>;
}
