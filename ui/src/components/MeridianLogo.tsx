import type { SVGProps } from "react";
/** Original geometric lettering, drawn as vectors without an icon or font dependency. */
export function MeridianLogo(props: SVGProps<SVGSVGElement>) {
  return <svg {...props} className={`meridian-logo ${props.className ?? ""}`} viewBox="0 0 268 28" width="144" height="18" aria-hidden="true" fill="none">
    <path d="M2 25V3L14 17 26 3V25 M60 3H38V25H60 M38 14H57 M72 25V3H87Q96 3 96 9Q96 14 87 14H72 M85 14L98 25 M110 3H126 M118 3V25 M110 25H126 M138 25V3H150Q164 3 164 14Q164 25 150 25Z M176 3H192 M184 3V25 M176 25H192 M204 25L216 3 228 25 M208 17H224 M240 25V3L264 25V3" stroke="currentColor" strokeWidth="2.6" strokeLinecap="square" strokeLinejoin="miter" />
  </svg>;
}
