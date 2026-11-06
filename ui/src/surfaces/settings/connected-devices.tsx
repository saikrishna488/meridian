import { Badge, Container, EmptyState } from "../../meridian-ui";
import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "../../meridian-ui";
import { Select } from "../../meridian-ui";
import { SearchField } from "../../meridian-ui";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import type { ManagedDevice } from "../../lib/generated/ManagedDevice";
import type { BluetoothDevice } from "../../lib/generated/BluetoothDevice";

type Peripheral = { id: string; category: string; name: string; status: string; detail: string; properties: Array<[string, string]> };
const categories = ["Keyboards", "Mice", "Touchpads", "Bluetooth devices", "USB devices", "Storage", "Printers", "Scanners", "Other peripherals"] as const;

function classify(device: ManagedDevice): string | null {
  if (["Keyboards", "Mice", "Touchpads", "Printers", "Scanners", "Storage"].includes(device.category)) return device.category;
  if (device.category === "USB devices") {
    // Avoid virtual USB root hubs and generic hubs cluttering the list.
    const name = device.name.toLowerCase();
    if (/^(usb \d+|usb root hub|root hub|generic usb hub)$/i.test(device.name) || name.includes("root hub")) return null;
    return "USB devices";
  }
  return null;
}
function bluetoothCategory(icon: string): string {
  if (/input-keyboard|keyboard/i.test(icon)) return "Keyboards";
  if (/input-mouse|mouse|trackball/i.test(icon)) return "Mice";
  return "Bluetooth devices";
}
export function ConnectedDevices() {
  const [hardware, setHardware] = useState<ManagedDevice[]>([]);
  const [printers, setPrinters] = useState<Peripheral[]>([]);
  const [inventoryNotice, setInventoryNotice] = useState("");
  const [bluetooth, setBluetooth] = useState<BluetoothDevice[]>([]);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("All devices");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const mounted = useRef(true);
  const pending = useRef(false);
  async function refresh() {
    if (pending.current) return;
    pending.current = true; setLoading(true); setError("");
    try {
      const [devices, paired, inventory] = await Promise.allSettled([
        request({ method: "settings.devices" }), request({ method: "settings.bluetooth_devices" }), request({ method: "settings.printers" }),
      ]);
      if (!mounted.current) return;
      if (devices.status === "fulfilled") setHardware(devices.value);
      if (paired.status === "fulfilled") setBluetooth(paired.value);
      const warnings: string[] = [];
      if (devices.status === "rejected") warnings.push(`Wired devices: ${errorMessage(devices.reason)}`);
      if (paired.status === "rejected") warnings.push(`Bluetooth: ${errorMessage(paired.reason)}`);
      if (inventory.status === "fulfilled") {
        setPrinters([
          ...inventory.value.printers.map((name, index): Peripheral => ({ id: `printer:${index}:${name}`, category: "Printers", name, status: "Configured", detail: "Print service", properties: [] })),
          ...inventory.value.scanners.map((name, index): Peripheral => ({ id: `scanner:${index}:${name}`, category: "Scanners", name, status: "Detected", detail: "Scanner service", properties: [] })),
        ]);
        setInventoryNotice([!inventory.value.cups_available && "Print service is unavailable.", !inventory.value.sane_available && "Scanner discovery is unavailable."].filter(Boolean).join(" "));
      } else { warnings.push(`Printers and scanners: ${errorMessage(inventory.reason)}`); }
      setError(warnings.join(" "));
    } catch (cause) {
      if (mounted.current) setError(errorMessage(cause));
    } finally {
      pending.current = false;
      if (mounted.current) setLoading(false);
    }
  }
  useEffect(() => {
    mounted.current = true; void refresh();
    return () => { mounted.current = false; };
  }, []);

  const items = useMemo(() => {
    const devices: Peripheral[] = hardware.flatMap(device => {
      const category = classify(device);
      if (!category) return [];
      return [{ id: device.id, category, name: device.name, status: device.status,
        detail: device.id.startsWith("usb:") ? "USB" : device.id.startsWith("input:") ? "Input device" : device.category,
        properties: device.properties }];
    });
    for (const device of bluetooth) {
      if (!device.connected && !device.paired) continue;
      devices.push({ id: `bt:${device.address}`, category: bluetoothCategory(device.icon),
        name: device.name || device.address, status: device.connected ? "Connected" : "Paired",
        detail: `Bluetooth · Wireless${device.battery == null ? "" : ` · Battery ${device.battery}%`}`,
        properties: [["Address", device.address], ["Trusted", device.trusted ? "Yes" : "No"]] });
    }
    for (const device of printers) {
      if (!devices.some(existing => existing.category === device.category && existing.name.toLowerCase() === device.name.toLowerCase())) devices.push(device);
    }
    // Bluetooth USB adapters appear as USB hardware as well; leave the
    // adapter visible but don't duplicate the same physical input device.
    return devices.filter(device => {
      const text = `${device.name} ${device.category} ${device.detail} ${device.id}`.toLowerCase();
      return (!query || text.includes(query.trim().toLowerCase()))
        && (filter === "All devices" || device.category === filter);
    }).sort((a, b) => categories.indexOf(a.category as typeof categories[number]) - categories.indexOf(b.category as typeof categories[number])
      || a.name.localeCompare(b.name));
  }, [hardware, bluetooth, printers, query, filter]);

  const grouped = categories.map(category => ({ category, items: items.filter(item => item.category === category) }))
    .filter(group => group.items.length);
  return <div className="connected-devices" aria-busy={loading}>
    <p className="section-caption">Connected hardware, including wired USB peripherals, input devices, Bluetooth accessories, printers, and physical storage. Disk operations remain in Storage.</p>
    <div className="settings-actions management-toolbar">
      <SearchField aria-label="Search connected devices" value={query} onValueChange={setQuery} placeholder="Search devices" />
      <Select aria-label="Device category" value={filter} onChange={event => setFilter(event.target.value)}>
        <option>All devices</option>{categories.map(category => <option key={category}>{category}</option>)}
      </Select>
      <Button disabled={loading} onClick={() => void refresh()}>{loading ? "Refreshing…" : "Refresh"}</Button>
    </div>
    <p role="status" className="section-caption">{loading ? "Detecting peripherals…" : `${items.length} devices shown`}</p>
    {error && <div role="alert" className="settings-error"><p>{error}</p><Button disabled={loading} onClick={() => void refresh()}>Retry discovery</Button></div>}
    {inventoryNotice && <p role="status" className="settings-notice">{inventoryNotice}</p>}
    {grouped.map(group => <Container key={group.category} role="group" className="settings-card connected-device-group" aria-label={group.category}>
      <h3>{group.category}</h3>
      {group.items.map(device => <details className="connected-device-row" key={device.id}>
        <summary><span className="connected-device-name"><strong>{device.name}</strong><small>{device.detail}</small></span>
          <span className="connected-device-status"><Badge tone={device.status === "Connected" ? "green" : device.status === "Paired" ? "blue" : "white"}>{device.status}</Badge></span></summary>
        <div className="connected-device-meta">
          {device.properties.filter(([key, value]) => !!key && !!value).map(([key, value]) =>
            <div key={key}><span>{key}</span><span>{value}</span></div>)}
        </div>
      </details>)}
    </Container>)}
    {!loading && !items.length && <EmptyState title={error ? "Device discovery is incomplete" : "No matching devices"} description={error ? "Some device services are unavailable. Use Retry discovery to refresh this list." : "Connect a device or change the filter."} />}
  </div>;
}
