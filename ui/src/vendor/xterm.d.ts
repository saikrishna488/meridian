export class Terminal {
  constructor(options?: Record<string, unknown>);
  cols: number; rows: number;
  readonly buffer: { active: { type: "normal" | "alternate"; viewportY: number; baseY: number } };
  readonly modes: { mouseTrackingMode: string };
  scrollLines(lines: number): void;
  scrollPages(pages: number): void;
  scrollToTop(): void;
  scrollToBottom(): void;
  onScroll(callback: (position: number) => void): { dispose(): void };
  attachCustomWheelEventHandler(callback: (event: WheelEvent) => boolean): void;
  options: { theme?: Record<string, string>; fontSize?: number; disableStdin?: boolean };
  open(element: HTMLElement): void;
  loadAddon(addon: unknown): void;
  write(data: Uint8Array | string, callback?: () => void): void;
  writeln(data: string): void;
  focus(): void;
  resize(cols: number, rows: number): void;
  refresh(start: number, end: number): void;
  dispose(): void;
  onData(callback: (data: string) => void): { dispose(): void };
  onResize(callback: (size: { cols: number; rows: number }) => void): { dispose(): void };
  attachCustomKeyEventHandler(callback: (event: KeyboardEvent) => boolean): void;
  getSelection(): string;
  paste(data: string): void;
}
