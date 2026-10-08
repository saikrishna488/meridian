//! Device integrations. Fixed argument vectors, password on stdin, bounded waits.
use gtk::{gio, glib};
use meridian_protocol::{
    BluetoothAction, BluetoothDevice, DisplayConfig, DisplayDecision, DisplayTrial, ErrorBody, ErrorCode,
    MonitorBrightness, MonitorBrightnessRequest, WifiConnect, WifiNetwork,
};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
fn error(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}
pub fn monitor_brightness(choice: MonitorBrightnessRequest) -> Result<MonitorBrightness, ErrorBody> {
    if choice.connector.is_empty()
        || choice.connector.len() > 64
        || !choice.connector.chars().all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
    {
        return Err(error("Invalid monitor connector"));
    }
    if choice.percent.is_some_and(|value| value > 100) {
        return Err(error("Brightness must be between 0 and 100"));
    }
    let outputs = meridian_services::displays::list().map_err(error)?;
    if !outputs.iter().any(|o| o.name == choice.connector && o.enabled) {
        return Err(error("Monitor is disconnected or disabled"));
    }
    if choice.connector.starts_with("eDP")
        || choice.connector.starts_with("LVDS")
        || choice.connector.starts_with("DSI")
    {
        return Err(error("Use the built-in display brightness control for this monitor"));
    }
    let (edid, bus) = connector_edid(&choice.connector).ok_or_else(|| error("Monitor EDID is unavailable"))?;
    let selector = edid.iter().map(|b| format!("{b:02x}")).collect::<String>();
    if bus.is_none()
        && outputs
            .iter()
            .filter(|o| o.enabled && o.name != choice.connector)
            .any(|o| connector_edid(&o.name).is_some_and(|(other, _)| other == edid))
    {
        return Err(error("This monitor cannot be distinguished safely from another connected display"));
    }
    let selection = if let Some(bus) = &bus {
        vec!["--bus".to_string(), bus.clone()]
    } else {
        vec!["--edid".to_string(), selector]
    };
    let current = read_ddc_brightness(&selection)?;
    if let Some(percent) = choice.percent {
        let target = (u32::from(percent) * u32::from(current.1) + 50) / 100;
        let mut args = vec!["ddcutil".to_string()];
        args.extend(selection);
        args.extend(["setvcp".into(), "10".into(), target.to_string()]);
        let refs: Vec<_> = args.iter().map(String::as_str).collect();
        run_timeout(&refs, None, Duration::from_secs(12))?;
    }
    Ok(MonitorBrightness {
        percent: Some(choice.percent.unwrap_or_else(|| (u32::from(current.0) * 100 / u32::from(current.1)) as u8)),
    })
}
fn connector_edid(connector: &str) -> Option<(Vec<u8>, Option<String>)> {
    for root in ["/sys/class/drm", "/run/host/sys/class/drm"] {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            if !entry.file_name().to_string_lossy().ends_with(&format!("-{connector}")) {
                continue;
            }
            let bytes = std::fs::read(entry.path().join("edid")).ok()?;
            if bytes.len() < 128 || bytes[..8] != [0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00] {
                continue;
            }
            let bus = std::fs::canonicalize(entry.path().join("ddc"))
                .ok()
                .and_then(|path| path.file_name()?.to_str().map(str::to_owned))
                .and_then(|name| {
                    name.strip_prefix("i2c-")
                        .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                        .map(str::to_owned)
                });
            return Some((bytes[..128].to_vec(), bus));
        }
    }
    None
}
fn read_ddc_brightness(selection: &[String]) -> Result<(u16, u16), ErrorBody> {
    let mut args = vec!["ddcutil".to_string()];
    args.extend(selection.iter().cloned());
    args.extend(["getvcp".into(), "10".into(), "--terse".into()]);
    let refs: Vec<_> = args.iter().map(String::as_str).collect();
    let output = run_timeout(&refs, None, Duration::from_secs(12))?;
    output
        .lines()
        .find_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() >= 5 && fields[0] == "VCP" && fields[1].eq_ignore_ascii_case("10") && fields[2] == "C" {
                let current = fields[3].parse::<u16>().ok()?;
                let max = fields[4].parse::<u16>().ok()?;
                (max > 0 && current <= max).then_some((current, max))
            } else {
                None
            }
        })
        .ok_or_else(|| error("This monitor does not support DDC/CI brightness control"))
}
fn run(args: &[&str], input: Option<&str>) -> Result<String, ErrorBody> {
    run_timeout(args, input, Duration::from_secs(40))
}
fn run_timeout(args: &[&str], input: Option<&str>, timeout: Duration) -> Result<String, ErrorBody> {
    let mut child = Command::new(args[0])
        .args(&args[1..])
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| error(format!("{} is unavailable", args[0])))?;
    let stdout = child.stdout.take().ok_or_else(|| error("No command output"))?;
    let stderr = child.stderr.take().ok_or_else(|| error("No command error stream"))?;
    let output = std::thread::spawn(move || {
        let mut bytes = vec![];
        stdout.take(1024 * 1024).read_to_end(&mut bytes).map(|_| bytes)
    });
    let errors = std::thread::spawn(move || {
        let mut bytes = vec![];
        stderr.take(1024 * 1024).read_to_end(&mut bytes).map(|_| ())
    });
    if let Some(mut stdin) = child.stdin.take()
        && let Some(input) = input
        && let Err(e) = stdin.write_all(input.as_bytes())
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error(e));
    }
    let deadline = Instant::now() + timeout;
    while child.try_wait().map_err(error)?.is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error("Device operation timed out"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let status = child.wait().map_err(error)?;
    let bytes = output.join().map_err(|_| error("Could not read command output"))?.map_err(error)?;
    let _ = errors.join();
    if !status.success() {
        return Err(error(
            "The system service could not complete this operation. Check the password, device availability, and permissions.",
        ));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
fn fields(line: &str) -> Vec<String> {
    let mut result = vec![String::new()];
    let mut escaped = false;
    for c in line.chars() {
        if escaped {
            result.last_mut().unwrap().push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == ':' {
            result.push(String::new());
        } else {
            result.last_mut().unwrap().push(c);
        }
    }
    result
}
pub fn wifi_networks() -> Result<Vec<WifiNetwork>, ErrorBody> {
    let output = run(
        &[
            "nmcli",
            "--terse",
            "--escape",
            "yes",
            "--fields",
            "IN-USE,SSID,BSSID,SIGNAL,SECURITY,DEVICE",
            "device",
            "wifi",
            "list",
            "--rescan",
            "yes",
        ],
        None,
    )?;
    let mut networks: Vec<_> = output
        .lines()
        .filter_map(|line| {
            let f = fields(line);
            if f.len() != 6 || f[1].is_empty() {
                return None;
            }
            Some(WifiNetwork {
                connected: f[0] == "*",
                ssid: f[1].clone(),
                bssid: f[2].clone(),
                signal: f[3].parse().unwrap_or(0),
                security: f[4].clone(),
                device: f[5].clone(),
            })
        })
        .collect();
    networks.sort_by_key(|n| (!n.connected, std::cmp::Reverse(n.signal)));
    networks.truncate(200);
    Ok(networks)
}
pub fn wifi_connect(choice: WifiConnect) -> Result<(), ErrorBody> {
    if !mac(&choice.bssid)
        || choice.device.is_empty()
        || choice.device.len() > 64
        || !choice.device.chars().all(|c| c.is_ascii_alphanumeric() || "_.:-".contains(c))
        || choice.password.len() > 256
        || choice.password.contains(['\n', '\r', '\0'])
    {
        return Err(error("Invalid Wi-Fi connection details"));
    }
    if !wifi_networks()?.iter().any(|n| n.bssid == choice.bssid && n.device == choice.device) {
        return Err(error("Network is no longer available. Scan again."));
    }
    let password = format!("{}\n", choice.password);
    run(
        &["nmcli", "--ask", "--wait", "30", "device", "wifi", "connect", &choice.bssid, "ifname", &choice.device],
        Some(&password),
    )?;
    Ok(())
}
fn mac(s: &str) -> bool {
    s.len() == 17
        && s.split(':').count() == 6
        && s.split(':').all(|b| b.len() == 2 && b.chars().all(|c| c.is_ascii_hexdigit()))
}
async fn objects() -> Result<glib::Variant, ErrorBody> {
    let bus = gio::bus_get_future(gio::BusType::System).await.map_err(error)?;
    bus.call_future(
        Some("org.bluez"),
        "/",
        "org.freedesktop.DBus.ObjectManager",
        "GetManagedObjects",
        None,
        None,
        gio::DBusCallFlags::NONE,
        10000,
    )
    .await
    .map_err(error)
}
fn prop<T: glib::variant::FromVariant>(dict: &glib::Variant, key: &str) -> Option<T> {
    glib::VariantDict::new(Some(dict)).lookup::<T>(key).ok().flatten()
}
fn interface(object: &glib::Variant, name: &str) -> Option<glib::Variant> {
    let interfaces = object.child_value(1);
    (0..interfaces.n_children()).find_map(|i| {
        let e = interfaces.child_value(i);
        (e.child_value(0).str() == Some(name)).then(|| e.child_value(1))
    })
}
pub async fn bluetooth_devices() -> Result<Vec<BluetoothDevice>, ErrorBody> {
    let managed = objects().await?;
    let data = managed.child_value(0);
    let mut result = vec![];
    for i in 0..data.n_children() {
        let object = data.child_value(i);
        if let Some(properties) = interface(&object, "org.bluez.Device1") {
            result.push(BluetoothDevice {
                path: object.child_value(0).str().unwrap_or_default().into(),
                name: prop::<String>(&properties, "Alias").unwrap_or_else(|| "Bluetooth device".into()),
                address: prop::<String>(&properties, "Address").unwrap_or_default(),
                paired: prop::<bool>(&properties, "Paired").unwrap_or(false),
                connected: prop::<bool>(&properties, "Connected").unwrap_or(false),
            });
        }
    }
    result.sort_by_key(|d| (!d.connected, !d.paired, d.name.clone()));
    Ok(result)
}
pub async fn bluetooth_scan() -> Result<(), ErrorBody> {
    let data = objects().await?.child_value(0);
    let path = (0..data.n_children())
        .find_map(|i| {
            let object = data.child_value(i);
            interface(&object, "org.bluez.Adapter1").map(|_| object.child_value(0).str().unwrap_or_default().to_owned())
        })
        .ok_or_else(|| error("No Bluetooth adapter"))?;
    let bus = gio::bus_get_future(gio::BusType::System).await.map_err(error)?;
    bus.call_future(
        Some("org.bluez"),
        &path,
        "org.bluez.Adapter1",
        "StartDiscovery",
        None,
        None,
        gio::DBusCallFlags::NONE,
        10000,
    )
    .await
    .map_err(error)?;
    glib::timeout_future(Duration::from_secs(6)).await;
    let _ = bus
        .call_future(
            Some("org.bluez"),
            &path,
            "org.bluez.Adapter1",
            "StopDiscovery",
            None,
            None,
            gio::DBusCallFlags::NONE,
            10000,
        )
        .await;
    Ok(())
}
pub async fn bluetooth_action(choice: BluetoothAction, parent: Option<gtk::Window>) -> Result<(), ErrorBody> {
    let devices = bluetooth_devices().await?;
    let device =
        devices.iter().find(|d| d.path == choice.path).ok_or_else(|| error("Device is no longer available"))?;
    let method = match choice.action.as_str() {
        "connect" => "Connect",
        "disconnect" => "Disconnect",
        "pair" => "Pair",
        _ => return Err(error("Unknown Bluetooth action")),
    };
    let bus = gio::bus_get_future(gio::BusType::System).await.map_err(error)?;
    if choice.action == "pair" {
        return crate::bluetooth_agent::pair(&bus, &device.path, &device.name, parent.as_ref()).await;
    }
    bus.call_future(
        Some("org.bluez"),
        &device.path,
        "org.bluez.Device1",
        method,
        None,
        None,
        gio::DBusCallFlags::NONE,
        30000,
    )
    .await
    .map_err(|_| error("Bluetooth could not connect to this device. Check that it is powered on and nearby."))?;
    Ok(())
}
struct Trial {
    token: String,
    before: Vec<DisplayConfig>,
    after: Vec<DisplayConfig>,
}
static TRIAL: std::sync::Mutex<Option<Trial>> = std::sync::Mutex::new(None);
static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn restore(before: &[DisplayConfig]) -> Result<(), ErrorBody> {
    let current = meridian_services::displays::list().map_err(error)?;
    let mut edits = meridian_services::displays::snapshot(&current);
    for e in &mut edits {
        if let Some(old) = before.iter().find(|old| old.name == e.name) {
            *e = old.clone();
        }
    }
    if !edits.iter().any(|e| e.enabled)
        && let Some(e) = edits.first_mut()
    {
        e.enabled = true;
    }
    meridian_services::displays::apply(&edits).map_err(error)
}
pub fn display_apply(edits: Vec<DisplayConfig>) -> Result<DisplayTrial, ErrorBody> {
    let mut pending = TRIAL.lock().map_err(error)?;
    if pending.is_some() {
        return Err(error("Keep or revert the current display changes first"));
    }
    let before = meridian_services::displays::snapshot(&meridian_services::displays::list().map_err(error)?);
    meridian_services::displays::apply(&edits).map_err(error)?;
    let token = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed).to_string();
    *pending = Some(Trial { token: token.clone(), before, after: edits });
    let id = token.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(15));
        if let Ok(mut pending) = TRIAL.lock()
            && pending.as_ref().is_some_and(|t| t.token == id)
            && let Some(trial) = pending.take()
            && let Err(e) = restore(&trial.before)
        {
            log::error!("Display rollback failed: {e:?}");
        }
    });
    Ok(DisplayTrial { token, seconds: 15 })
}
fn take_trial(pending: &mut Option<Trial>, token: &str, keep: bool) -> Result<Option<Trial>, ErrorBody> {
    match pending.as_ref() {
        None if !keep => Ok(None),
        Some(trial) if trial.token == token => Ok(pending.take()),
        _ => Err(error("This display trial has already ended")),
    }
}
pub fn display_decide(choice: DisplayDecision) -> Result<(), ErrorBody> {
    let mut pending = TRIAL.lock().map_err(error)?;
    let Some(trial) = take_trial(&mut pending, &choice.token, choice.keep)? else {
        return Ok(());
    };
    if choice.keep {
        let directory = glib::user_config_dir().join("meridian");
        let saved = (|| {
            std::fs::create_dir_all(&directory).map_err(error)?;
            let temporary = directory.join("displays.json.tmp");
            std::fs::write(&temporary, serde_json::to_vec_pretty(&trial.after).map_err(error)?).map_err(error)?;
            std::fs::rename(temporary, directory.join("displays.json")).map_err(error)
        })();
        if let Err(e) = saved {
            let _ = restore(&trial.before);
            return Err(e);
        }
    } else {
        restore(&trial.before)?;
    }
    Ok(())
}
pub fn apply_saved_displays() -> Result<(), ErrorBody> {
    let path = glib::user_config_dir().join("meridian/displays.json");
    if !path.exists() {
        return Ok(());
    }
    let edits: Vec<DisplayConfig> = serde_json::from_slice(&std::fs::read(path).map_err(error)?).map_err(error)?;
    restore(&edits)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_decisions_reject_stale_tokens_and_reverting_twice_is_safe() {
        let mut pending = Some(Trial { token: "new".into(), before: vec![], after: vec![] });
        assert!(take_trial(&mut pending, "old", true).is_err());
        assert!(pending.is_some());
        assert!(take_trial(&mut pending, "new", false).unwrap().is_some());
        assert!(take_trial(&mut pending, "new", false).unwrap().is_none());
        assert!(take_trial(&mut pending, "new", true).is_err());
    }
    #[test]
    fn command_runner_keeps_secrets_on_stdin_and_drains_large_output() {
        let result = run(&["/usr/bin/python3", "-c", "import sys; secret=sys.stdin.readline().strip(); assert secret=='test-secret'; assert secret not in sys.argv; sys.stdout.write('x'*200000)"], Some("test-secret\n")).unwrap();
        assert_eq!(result.len(), 200000);
    }
    #[test]
    fn parses_nmcli_escaped_fields_and_rejects_invalid_addresses() {
        assert_eq!(
            fields("*:Office\\:Guest:AA\\:BB\\:CC\\:DD\\:EE\\:FF:80:WPA2:wlan0"),
            vec!["*", "Office:Guest", "AA:BB:CC:DD:EE:FF", "80", "WPA2", "wlan0"]
        );
        assert!(mac("AA:BB:CC:DD:EE:FF"));
        assert!(!mac("--help"));
    }
}
