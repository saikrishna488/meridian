// Supply the entire palette on creation and every appearance change.
export function terminalTheme(mode: string | undefined): Record<string, string> {
  void mode;
  return {
    background: "#ffffff", foreground: "#000000", cursor: "#000000", cursorAccent: "#ffffff", selectionBackground: "#9cc7ff",
    black: "#111111", red: "#a40000", green: "#08752c", yellow: "#795500", blue: "#124caa", magenta: "#7d2988", cyan: "#006a70", white: "#555555",
    brightBlack: "#666666", brightRed: "#c40000", brightGreen: "#008a35", brightYellow: "#8a6200", brightBlue: "#005bd6", brightMagenta: "#9c27a5", brightCyan: "#007e86", brightWhite: "#000000",
  };
}
