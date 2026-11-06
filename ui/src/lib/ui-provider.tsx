import type { ReactNode } from "react";
/** Meridian owns every control; no third-party UI provider is required. */
export function UIProvider({ children }: { children: ReactNode }) { return <>{children}</>; }
