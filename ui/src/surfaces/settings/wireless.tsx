import { Button } from "../../components/ui/button";
import { useEffect, useRef, useState } from "react";
import { Icon, type IconName } from "../../components/Icon";
import { Select } from "../../components/Select";
import { Toggle } from "../../components/Toggle";
import { request } from "../../lib/bridge";
import { errorMessage, useDialogFocus } from "../../lib/react";
import { useSystemSettings } from "../../lib/settings";
import type { BluetoothDevice } from "../../lib/generated/BluetoothDevice";
import type { WifiNetwork } from "../../lib/generated/WifiNetwork";
import type { WifiProfile } from "../../lib/generated/WifiProfile";
import type { WifiConfig } from "../../lib/generated/WifiConfig";

function deviceIcon(device: BluetoothDevice): IconName {
  if (device.icon?.includes("audio")) return "sound";
  if (device.icon?.includes("keyboard")) return "keyboard";
  return "bluetooth";
}
function security(network: WifiNetwork) { return network.security && network.security !== "--" ? network.security : "Open network"; }
function signal(value: number) { return value >= 75 ? "Excellent" : value >= 50 ? "Good" : value >= 25 ? "Fair" : "Weak"; }
function Properties({ items }: { items: [string, string][] }) {
  return <dl className="wireless-properties">{items.map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value || "Unavailable"}</dd></div>)}</dl>;
}

export function Wireless({ kind }: { kind: "wifi" | "bluetooth" }) {
  const { settings, setSettings } = useSystemSettings();
  const [networks, setNetworks] = useState<WifiNetwork[]>([]);
  const [profiles, setProfiles] = useState<WifiProfile[]>([]);
  const [devices, setDevices] = useState<BluetoothDevice[]>([]);
  const [discovered, setDiscovered] = useState(false);
  const [loading, setLoading] = useState(true), [busy, setBusy] = useState(false);
  const [operation, setOperation] = useState(""), [error, setError] = useState(""), [notice, setNotice] = useState("");
  const [query, setQuery] = useState(""), [expanded, setExpanded] = useState("");
  const [network, setNetwork] = useState<WifiNetwork | null>(null), [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [profile, setProfile] = useState<WifiProfile | null>(null), [config, setConfig] = useState<WifiConfig | null>(null);
  const [configLoading, setConfigLoading] = useState(false);
  const [confirmation, setConfirmation] = useState<{ name: string; uuid?: string; device?: BluetoothDevice } | null>(null);
  const joinDialog = useRef<HTMLFormElement>(null), configDialog = useRef<HTMLFormElement>(null), forgetDialog = useRef<HTMLElement>(null);
  const configRevision = useRef(0);
  const name = kind === "wifi" ? "Wi-Fi" : "Bluetooth", value = settings[kind];
  function closeJoin() { setNetwork(null); setPassword(""); setShowPassword(false); }
  function closeConfig() { configRevision.current++; setProfile(null); setConfig(null); setConfigLoading(false); }
  useDialogFocus(joinDialog, !!network, () => { if (!busy) closeJoin(); });
  useDialogFocus(configDialog, !!profile && !confirmation, () => { if (!busy) closeConfig(); });
  useDialogFocus(forgetDialog, !!confirmation, () => { if (!busy) setConfirmation(null); });

  async function load() {
    if (kind === "wifi") {
      const results = await Promise.allSettled([
        request({ method: "settings.wifi_networks" }).then(v => setNetworks(v ?? [])),
        request({ method: "settings.wifi_profiles" }).then(v => setProfiles(v ?? [])),
      ]);
      const failed = results.find(result => result.status === "rejected");
      if (failed?.status === "rejected") throw failed.reason;
    } else setDevices(await request({ method: "settings.bluetooth_devices" }) ?? []);
  }
  useEffect(() => {
    setDiscovered(false);
    void load().catch(e => setError(errorMessage(e))).finally(() => setLoading(false));
    return () => { configRevision.current++; };
  }, [kind]);

  async function perform(message: string, action: () => Promise<void>) {
    if (busy || loading) return;
    setBusy(true); setOperation(message); setError(""); setNotice("");
    try { await action(); }
    catch (e) { setError(errorMessage(e)); }
    finally { setBusy(false); setOperation(""); }
  }
  async function toggle(enabled: boolean) {
    await perform(`${enabled ? "Turning on" : "Turning off"} ${name}…`, async () => {
      await request({ method: kind === "wifi" ? "settings.set_wifi" : "settings.set_bluetooth", params: { enabled } });
      setSettings(await request({ method: "settings.get" }));
      if (enabled) await load();
    });
  }
  async function scan() {
    await perform(kind === "wifi" ? "Scanning for networks…" : "Discovering Bluetooth devices…", async () => {
      if (kind === "bluetooth") await request({ method: "settings.bluetooth_scan" });
      else await request({ method: "settings.wifi_scan" });
      await load();
      setDiscovered(true);
    });
  }
  async function connect() {
    if (!network) return;
    await perform(`Connecting to ${network.ssid}…`, async () => {
      try { await request({ method: "settings.wifi_connect", params: { bssid: network.bssid, device: network.device, password } }); }
      finally { setPassword(""); }
      closeJoin(); await load();
    });
  }
  async function deviceAction(device: BluetoothDevice, action: string) {
    await perform(`${action === "pair" ? "Pairing with" : "Updating"} ${device.name}…${action === "pair" ? " Confirm any pairing request." : ""}`, async () => {
      await request({ method: "settings.bluetooth_action", params: { path: device.path, action } });
      setConfirmation(null); await load();
    });
  }
  async function profileAction(item: WifiProfile, action: string) {
    await perform(`Updating ${item.name}…`, async () => {
      await request({ method: "settings.wifi_profile_action", params: { uuid: item.uuid, action } });
      setConfirmation(null);
      if (action === "forget") closeConfig();
      await load();
    });
  }
  async function openConfig(item: WifiProfile) {
    const revision = ++configRevision.current;
    setProfile(item); setConfig(null); setConfigLoading(true); setError(""); setNotice("");
    try {
      const result = await request({ method: "settings.wifi_config", params: { uuid: item.uuid } });
      if (revision === configRevision.current) setConfig(result);
    } catch (e) { if (revision === configRevision.current) setError(errorMessage(e)); }
    finally { if (revision === configRevision.current) setConfigLoading(false); }
  }
  function updateConfig(patch: Partial<WifiConfig>) { setConfig(current => current ? { ...current, ...patch } : current); }
  async function saveConfig() {
    if (!config) return;
    await perform(`Saving ${profile?.name}…`, async () => {
      await request({ method: "settings.wifi_config_set", params: config });
      closeConfig(); setNotice("Network settings saved. Reconnect to apply connection changes.");
      await load();
    });
  }
  const disabled = busy || loading;
  const matches = (text: string) => text.toLowerCase().includes(query.trim().toLowerCase());
  const visibleNetworks = networks.filter(n => matches(`${n.ssid} ${n.security} ${n.device}`));
  const visibleProfiles = profiles.filter(p => matches(p.name));
  const visibleDevices = devices.filter(d => matches(`${d.name} ${d.address}`));
  const paired = visibleDevices.filter(d => d.paired || d.connected), nearby = visibleDevices.filter(d => !d.paired && !d.connected);
  const connectedNetworks = visibleNetworks.filter(n => n.connected);
  const nearbyNetworks = visibleNetworks.filter(n => !n.connected);
  function wifiRows(items: WifiNetwork[]) {
    return items.map(n => {
      const id = `${n.device}:${n.bssid}`;
      return <article className={`wireless-item${n.connected ? " is-connected" : ""}`} key={id}><div className="settings-row wireless-row"><span className="wireless-symbol"><Icon name="wifi" /></span><div className="device-name"><strong>{n.ssid}{n.connected && <span className="wireless-badge"><Icon name="check" width="11" height="11" />Connected</span>}</strong><small className="row-detail">{signal(n.signal)} signal · {security(n)}{n.frequency ? ` · ${(n.frequency / 1000).toFixed(1)} GHz` : ""}</small></div><div className="wireless-row-actions">{!n.connected && <Button disabled={disabled} onClick={() => { setError(""); setNetwork(n); setPassword(""); setShowPassword(false); }}>Connect</Button>}<Button className="wireless-details-button" aria-label={`Details for ${n.ssid} on ${n.bssid}`} aria-expanded={expanded === id} aria-controls={`details-${id}`} onClick={() => setExpanded(expanded === id ? "" : id)}><Icon name="info" width="17" height="17" /></Button></div></div>
        {expanded === id && <div className="wireless-details" id={`details-${id}`}><Properties items={[["Security", security(n)], ["Signal strength", `${n.signal}% · ${signal(n.signal)}`], ["Access point", n.bssid], ["Adapter", n.device], ["Frequency / channel", n.frequency ? `${n.frequency} MHz · Channel ${n.channel}` : "Unavailable"], ["Advertised rate", n.rate]]} /></div>}
      </article>;
    });
  }
  function bluetoothRows(items: BluetoothDevice[]) {
    return items.map(d => <article className={`wireless-item${d.connected && value ? " is-connected" : ""}`} key={d.path}>
      <div className="settings-row wireless-row"><span className="wireless-symbol"><Icon name={deviceIcon(d)} /></span>
        <div className="device-name"><strong>{d.name}</strong><small className="row-detail">{d.connected && value ? "Connected" : d.paired ? "Paired" : "Ready to pair"}{d.battery != null && value && ` · ${d.battery}% battery`}</small></div>
        <div className="wireless-row-actions"><Button disabled={disabled || value !== true} onClick={() => void deviceAction(d, d.connected ? "disconnect" : d.paired ? "connect" : "pair")}>{d.connected && value ? "Disconnect" : d.paired ? "Connect" : "Pair"}</Button>
          <Button className="wireless-details-button" aria-label={`Details for ${d.name}`} aria-expanded={expanded === d.path} aria-controls={`details-${d.path}`} onClick={() => setExpanded(expanded === d.path ? "" : d.path)}><Icon name="info" width="17" height="17" /></Button></div>
      </div>
      {expanded === d.path && <div className="wireless-details" id={`details-${d.path}`}><Properties items={[["Device address", d.address], ["Pairing", d.paired ? "Paired" : "Not paired"], ["Device type", d.icon || "Bluetooth device"], ["Trusted", d.trusted ? "Yes" : "No"], ["Battery", d.battery == null ? "Not reported" : `${d.battery}%`]]} />
        {d.paired && <><div className="settings-row wireless-trust"><div><strong>Trust this device</strong><small className="row-detail">Allow this device to connect without asking again.</small></div><Toggle label={`Trust ${d.name}`} checked={!!d.trusted} disabled={disabled || value !== true} onChange={trusted => void deviceAction(d, trusted ? "trust" : "untrust")} /></div><Button className="wireless-forget" disabled={disabled} onClick={() => { setError(""); setConfirmation({ name: d.name, device: d }); }}>Forget device…</Button></>}
      </div>}
    </article>);
  }
  return <div className="wireless-page" aria-busy={disabled}>
    <div className="settings-card wireless-summary"><span className={`wireless-summary-icon${value ? " enabled" : ""}`}><Icon name={kind} width="25" height="25" /></span><div><h3>{name}</h3><p>{value === null ? "No adapter available" : !value ? `${name} is turned off` : kind === "wifi" ? networks.find(n => n.connected)?.ssid ? `Connected to ${networks.find(n => n.connected)!.ssid}` : "Ready to connect" : `${devices.filter(d => d.connected).length} connected · Discover and manage your devices`}</p></div><Toggle id={kind} label={name} checked={value === true} disabled={disabled || value === null} onChange={v => void toggle(v)} /></div>
    <div className="wireless-toolbar"><label className="wireless-search"><Icon name="search" width="16" height="16" /><input aria-label={kind === "wifi" ? "Filter networks" : "Filter Bluetooth devices"} placeholder={kind === "wifi" ? "Find a network" : "Find a device"} value={query} onChange={e => setQuery(e.target.value)} /></label><Button variant="primary" className="primary" disabled={disabled || value !== true} onClick={() => void scan()}><Icon name={discovered ? "refresh" : "plus"} width="15" height="15" />{busy ? (kind === "wifi" ? "Scanning…" : "Searching…") : discovered ? "Scan again" : (kind === "wifi" ? "New connection" : "Pair new device")}</Button></div>
    {(operation || loading || notice) && <p role="status" className="wireless-status">{operation || (loading ? `Loading ${kind === "wifi" ? "networks" : "devices"}…` : notice)}</p>}
    {error && !network && !profile && !confirmation && <p className="settings-error" role="alert">{error}</p>}
    {kind === "wifi" ? <>
      <div className="wireless-list-heading"><h3>Connected network</h3><span>{value === true ? connectedNetworks.length : 0} connected</span></div>
      <div className="settings-card wireless-list">{value === true && wifiRows(connectedNetworks)}{(!value || !connectedNetworks.length) && <div className="wireless-empty"><Icon name="wifi" width="28" height="28" /><strong>{loading ? "Loading connection…" : value === null ? "No Wi-Fi adapter" : !value ? "Wi-Fi is off" : "Not connected"}</strong><p>{value === true ? "Your active Wi-Fi connection will appear here." : "Turn on Wi-Fi to view your connection."}</p></div>}</div>
      {discovered && <><div className="wireless-list-heading"><h3>Nearby networks</h3><span>{value === true ? nearbyNetworks.length : 0} found</span></div><div className="settings-card wireless-list">{value === true && wifiRows(nearbyNetworks)}{(!value || !nearbyNetworks.length) && <div className="wireless-empty"><Icon name="wifi" width="28" height="28" /><strong>{loading ? "Looking for networks…" : value === null ? "No Wi-Fi adapter" : !value ? "Wi-Fi is off" : query ? "No matching networks" : "No networks found"}</strong><p>{value === true ? "Try scanning again or move closer to your router." : "Turn on Wi-Fi to see nearby networks."}</p></div>}</div></>}
      <div className="wireless-list-heading"><h3>Saved networks</h3><span>{visibleProfiles.length} saved</span></div><p className="section-caption">Manage automatic connections, metered usage, IPv4 and DNS for networks you’ve joined.</p>
      <div className="settings-card wireless-list">{visibleProfiles.map(p => <div className="settings-row wireless-row" key={p.uuid}><span className="wireless-symbol"><Icon name="lock" /></span><div className="device-name"><strong>{p.name}</strong><small className="row-detail">{p.device && value ? `Active on ${p.device}` : "Saved connection"}</small></div><div className="wireless-row-actions"><Button disabled={disabled || value !== true} onClick={() => void profileAction(p, p.device ? "disconnect" : "connect")}>{p.device && value ? "Disconnect" : "Connect"}</Button><Button aria-label={`Configure ${p.name}`} disabled={disabled} onClick={() => void openConfig(p)}><Icon name="settings" width="15" height="15" />Configure</Button></div></div>)}{!visibleProfiles.length && <p className="device-empty">{loading ? "Loading saved networks…" : query ? "No saved networks match your search." : "Networks you join will appear here."}</p>}</div>
    </> : <>
      <div className="wireless-list-heading"><h3>Your devices</h3><span>{paired.length} saved</span></div><div className="settings-card wireless-list">{bluetoothRows(paired)}{!paired.length && <p className="device-empty">{query ? "No saved devices match your search." : "Paired devices will appear here for easy reconnection."}</p>}</div>
      <div className="wireless-list-heading"><h3>Nearby devices</h3><span>{discovered && value === true ? nearby.length : "—"}</span></div><p className="section-caption">Put your headphones, keyboard or other accessory in pairing mode, then choose Pair new device.</p><div className="settings-card wireless-list">{discovered && value === true && bluetoothRows(nearby)}{(!discovered || value !== true || !nearby.length) && <div className="wireless-empty"><Icon name="bluetooth" width="28" height="28" /><strong>{loading ? "Loading paired devices…" : value === null ? "No Bluetooth adapter" : !value ? "Bluetooth is off" : !discovered ? "Ready to pair" : query ? "No matching devices" : "No nearby devices found"}</strong><p>{value === true ? !discovered ? "Nearby discovery starts only when you click Pair new device." : "Keep your device nearby and make sure it is discoverable." : "Turn on Bluetooth to discover and connect devices."}</p></div>}</div>
      <section className="settings-card bluetooth-codecs"><div><h3>Bluetooth audio codecs</h3><p>PipeWire negotiates a codec supported by both your system and headset. Availability depends on installed codec plugins and the connected device.</p></div><div className="codec-list">{([
        ["SBC", "Medium quality", "Low stutter risk", "Broad compatibility"], ["SBC-XQ", "High quality", "Range sensitive", "Higher bitrate"], ["AAC", "Medium–high quality", "Device dependent", "Common on phones"], ["aptX", "High quality", "Low latency", "Device support required"], ["aptX HD", "Very high quality", "Range sensitive", "Higher bitrate"], ["aptX LL", "Medium quality", "Very low latency", "Gaming / video"], ["aptX LL Duplex", "Low quality", "Very low latency", "Two-way audio"], ["FastStream", "Medium quality", "Low latency", "Two-way audio"], ["FastStream Duplex", "Medium quality", "Low latency", "Two-way audio"], ["LDAC", "Very high quality", "Stutter possible at range", "High bitrate modes"], ["LC3", "High efficiency", "Low latency", "LE Audio devices"], ["LC3plus H3", "High quality", "Device dependent", "Hardware support required"], ["Opus 0.5", "High quality", "Low latency", "Stereo profile"], ["Opus 0.5 5.1", "High quality", "Device dependent", "5.1 profile"], ["Opus 0.5 7.1", "High quality", "Device dependent", "7.1 profile"], ["Opus 0.5 Duplex", "Medium quality", "Low latency", "Two-way audio"], ["Opus 0.5 Pro", "Very high quality", "Device dependent", "Pro profile"],
      ] as const).map(([codec, quality, stability, detail]) => <div className="codec-row" key={codec}><strong>{codec}</strong><span>{quality}</span><span>{stability}</span><small>{detail}</small></div>)}</div><small className="codec-note">Codec names describe common tradeoffs. Actual sound quality, delay and connection stability vary by headset, radio conditions and system support. Meridian lets PipeWire choose the negotiated codec automatically.</small></section>
    </>}
    {network && <div className="device-dialog-scrim"><form ref={joinDialog} className="device-dialog wireless-dialog" role="dialog" aria-modal="true" aria-label="Connect to Wi-Fi" onSubmit={e => { e.preventDefault(); void connect(); }}><span className="wireless-dialog-icon"><Icon name="wifi" width="24" height="24" /></span><h3>Join {network.ssid}</h3><p>{security(network)} · {signal(network.signal)} signal</p>{security(network) !== "Open network" ? <><label>Password<input type={showPassword ? "text" : "password"} aria-label="Wi-Fi password" autoComplete="new-password" value={password} disabled={busy} onChange={e => setPassword(e.target.value)} data-autofocus /></label><label className="wireless-checkbox"><input type="checkbox" checked={showPassword} onChange={e => setShowPassword(e.target.checked)} />Show password</label></> : <p>This network is open. Traffic may be visible to other people on the network.</p>}{error && <p role="alert" className="settings-error">{error}</p>}{operation && <p role="status">{operation}</p>}<div className="settings-actions"><Button type="button" disabled={busy} onClick={closeJoin}>Cancel</Button><Button variant="primary" className="primary" disabled={busy} type="submit">{busy ? "Connecting…" : "Join"}</Button></div></form></div>}
    {profile && !confirmation && <div className="device-dialog-scrim"><form ref={configDialog} className="device-dialog wireless-dialog wireless-config" role="dialog" aria-modal="true" aria-label={`Configure ${profile.name}`} onSubmit={e => { e.preventDefault(); void saveConfig(); }}><h3>{profile.name}</h3><p>Saved network configuration</p>{configLoading && <p role="status">Loading configuration…</p>}{config && <>
      <div className="settings-row"><label htmlFor="wifi-autoconnect">Connect automatically</label><Toggle id="wifi-autoconnect" label="Connect automatically" checked={config.autoconnect} disabled={busy} onChange={autoconnect => updateConfig({ autoconnect })} /></div>
      <div className="settings-row"><label htmlFor="wifi-metered">Metered connection</label><Select id="wifi-metered" value={config.metered} disabled={busy} onChange={e => updateConfig({ metered: e.target.value })}><option value="unknown">Automatic</option><option value="yes">Yes · limited data</option><option value="no">No · unlimited data</option>{!["unknown", "yes", "no"].includes(config.metered) && <option value={config.metered}>{config.metered}</option>}</Select></div>
      <div className="settings-row"><label htmlFor="wifi-ipv4">IPv4 configuration</label><Select id="wifi-ipv4" value={config.method} disabled={busy} onChange={e => updateConfig({ method: e.target.value, addresses: "", gateway: "" })}><option value="auto">Automatic (DHCP)</option><option value="manual">Manual</option>{!["auto", "manual"].includes(config.method) && <option value={config.method}>{config.method} (existing)</option>}</Select></div>
      {config.method === "manual" && <div className="wireless-config-fields"><label>IPv4 addresses<input aria-label="IPv4 addresses" placeholder="192.168.1.20/24" required value={config.addresses} disabled={busy} onChange={e => updateConfig({ addresses: e.target.value })} /></label><label>Gateway<input aria-label="IPv4 gateway" placeholder="192.168.1.1" value={config.gateway} disabled={busy} onChange={e => updateConfig({ gateway: e.target.value })} /></label><small>Use commas to separate multiple addresses. Include a prefix, such as /24.</small></div>}
      <div className="settings-row"><label htmlFor="wifi-auto-dns">Automatic DNS</label><Toggle id="wifi-auto-dns" label="Automatic DNS" checked={!config.ignore_auto_dns} disabled={busy} onChange={automatic => updateConfig({ ignore_auto_dns: !automatic })} /></div>
      <label className="wireless-dns">{config.ignore_auto_dns ? "DNS servers" : "Additional DNS servers"}<input aria-label="DNS servers" placeholder="1.1.1.1, 8.8.8.8" value={config.dns} disabled={busy} onChange={e => updateConfig({ dns: e.target.value })} /><small>IPv4 addresses, separated by commas.</small></label>
      <p>Changes take effect when you reconnect. IPv6 settings are managed by your existing network profile.</p>
    </>}{error && <p role="alert" className="settings-error">{error}</p>}<div className="wireless-config-footer"><Button className="wireless-forget" type="button" disabled={busy || configLoading} onClick={() => { setError(""); setConfirmation({ name: profile.name, uuid: profile.uuid }); }}>Forget network…</Button><div className="settings-actions"><Button type="button" disabled={busy} onClick={closeConfig}>Cancel</Button><Button variant="primary" className="primary" type="submit" disabled={busy || !config || !["auto", "manual"].includes(config.method) || !["unknown", "yes", "no"].includes(config.metered)}>{busy ? "Saving…" : "Save changes"}</Button></div></div></form></div>}
    {confirmation && <div className="device-dialog-scrim"><section ref={forgetDialog} className="device-dialog wireless-dialog" role="dialog" aria-modal="true" aria-label={`Forget ${confirmation.name}`}><h3>Forget {confirmation.name}?</h3><p>{confirmation.device ? "This removes the pairing. You’ll need to pair this device again to reconnect." : "This removes the saved configuration and disconnects this network if active. You’ll need to join again to reconnect."}</p>{error && <p role="alert" className="settings-error">{error}</p>}<div className="settings-actions"><Button disabled={busy} onClick={() => setConfirmation(null)}>Cancel</Button><Button variant="primary" className="primary" disabled={busy} onClick={() => confirmation.device ? void deviceAction(confirmation.device, "forget") : void profileAction({ uuid: confirmation.uuid!, name: confirmation.name, device: "" }, "forget")}>{busy ? "Forgetting…" : "Forget"}</Button></div></section></div>}
  </div>;
}
