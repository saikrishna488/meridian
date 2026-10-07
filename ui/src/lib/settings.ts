import { useEffect, useRef, useState } from "react";
import { on, request } from "./bridge";
import type { SettingsState } from "./generated/SettingsState";
export function useSystemSettings() {
 const [settings, setSettings] = useState<SettingsState>({ wifi:null, bluetooth:null, brightness:null, volume:null });
 const revision = useRef(0);
 useEffect(() => {
  const off = on("settings.changed", event => { revision.current++; setSettings(event.data); });
  const mine = revision.current;
  void request({ method:"settings.get" }).then(value => { if (value && mine === revision.current) setSettings(value); }).catch(console.error);
  return off;
 }, []);
 return { settings, setSettings };
}
