// Complete, high-contrast palettes; xterm also corrects low-contrast truecolor text.
export function terminalTheme(mode: string | undefined): Record<string, string> {
  if (mode === "dark") return {
    background: "#171a20", foreground: "#e7edf7", cursor: "#e7edf7", cursorAccent: "#171a20", selectionBackground: "#334d73",
    black: "#101218", red: "#ff8a91", green: "#9edb92", yellow: "#e6cc83", blue: "#8db8ff", magenta: "#d9a1ed", cyan: "#87d4dd", white: "#d3dbe8",
    brightBlack: "#9aa6ba", brightRed: "#ffabb0", brightGreen: "#b5e8aa", brightYellow: "#f5df9f", brightBlue: "#b0ceff", brightMagenta: "#e9bdf6", brightCyan: "#a9e6ec", brightWhite: "#ffffff",
  };
  return {
    background: "#fafafa", foreground: "#202530", cursor: "#202530", cursorAccent: "#fafafa", selectionBackground: "#b6d3f5",
    black: "#202530", red: "#a52a37", green: "#24632f", yellow: "#70500c", blue: "#2257a0", magenta: "#803c8b", cyan: "#00656c", white: "#505867",
    brightBlack: "#5a6473", brightRed: "#b22032", brightGreen: "#276b34", brightYellow: "#75550a", brightBlue: "#155db7", brightMagenta: "#873b99", brightCyan: "#00666f", brightWhite: "#202530",
  };
}
