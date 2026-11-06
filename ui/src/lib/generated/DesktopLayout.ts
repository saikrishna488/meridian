import type { DesktopPosition } from "./DesktopPosition";
export type DesktopLayout = { icon_size: number; spacing: number; snap_to_grid: boolean; show_icons: boolean; window_buttons: string; positions: Record<string, DesktopPosition> };
