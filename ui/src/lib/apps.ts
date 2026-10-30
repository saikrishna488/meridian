/** Meridian-provided applications are managed by the shell, not the system package manager. */
export function isBuiltinApp(id: string) {
  return [
    "meridian-pulse.desktop",
    "meridian-settings.desktop",
    "meridian-terminal.desktop",
    "meridian-finder.desktop",
  ].includes(id);
}
