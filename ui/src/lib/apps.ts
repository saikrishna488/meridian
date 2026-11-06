/** Meridian-provided applications are managed by the shell, not the system package manager. */
export function isBuiltinApp(id: string) {
  return [
    "meridian-pulse.desktop",
    "meridian-photos.desktop",
    "meridian-media.desktop",
    "meridian-calendar.desktop",
    "meridian-clock.desktop",
    "meridian-terminal.desktop",
    "meridian-settings.desktop",
    "meridian-engines.desktop",
    "meridian-finder.desktop",
  ].includes(id);
}
