//! Lazy capability probes: query only the Settings domain currently on screen.
use gtk::{gio, glib};
use meridian_protocol::{CapabilityState as S, SettingsFeature, SystemCapability};
use std::time::Duration;
fn state(state: S, message: impl Into<String>) -> SystemCapability {
    SystemCapability { state, message: message.into(), adapters: vec![] }
}
pub(crate) fn classify(message: &str) -> S {
    let text = message.to_ascii_lowercase();
    if text.contains("permission") || text.contains("not authorized") || text.contains("accessdenied") {
        S::PermissionDenied
    } else if text.contains("not supported") || text.contains("does not support") {
        S::Unsupported
    } else if text.contains("unavailable") || text.contains("serviceunknown") || text.contains("not running") {
        S::ServiceUnavailable
    } else {
        S::TemporarilyUnavailable
    }
}
async fn bluez() -> SystemCapability {
    let result = async {
        let bus = crate::settings_devices::system_bus().await?;
        bus.call_future(
            Some("org.bluez"),
            "/",
            "org.freedesktop.DBus.ObjectManager",
            "GetManagedObjects",
            None,
            None,
            gio::DBusCallFlags::NONE,
            5000,
        )
        .await
    }
    .await;
    match result {
        Err(e) => state(
            classify(&e.to_string()),
            "Bluetooth service unavailable. Check BlueZ and system permissions, then retry.",
        ),
        Ok(reply) => {
            let objects = reply.child_value(0);
            let mut adapters = 0;
            let mut powered = 0;
            let mut radios = vec![];
            for i in 0..objects.n_children() {
                let interfaces = objects.child_value(i).child_value(1);
                for j in 0..interfaces.n_children() {
                    let entry = interfaces.child_value(j);
                    if entry.child_value(0).str() == Some("org.bluez.Adapter1") {
                        adapters += 1;
                        let properties = glib::VariantDict::new(Some(&entry.child_value(1)));
                        radios.push(meridian_protocol::RadioAdapter {
                            path: objects.child_value(i).child_value(0).str().unwrap_or_default().into(),
                            name: properties
                                .lookup::<String>("Alias")
                                .ok()
                                .flatten()
                                .unwrap_or_else(|| "Bluetooth adapter".into()),
                            enabled: properties.lookup::<bool>("Powered").ok().flatten().unwrap_or(false),
                        });
                        if glib::VariantDict::new(Some(&entry.child_value(1))).lookup::<bool>("Powered").ok().flatten()
                            == Some(true)
                        {
                            powered += 1;
                        }
                    }
                }
            }
            let mut result = if adapters == 0 {
                state(
                    S::HardwareUnavailable,
                    "No Bluetooth adapter detected. Connect an adapter or enable USB passthrough in your VM.",
                )
            } else if powered == 0 {
                state(
                    S::Disabled,
                    "Bluetooth is off. If it cannot be enabled, check the radio or airplane-mode switch.",
                )
            } else {
                state(S::Ready, format!("{powered} of {adapters} Bluetooth adapters enabled"))
            };
            result.adapters = radios;
            result
        }
    }
}
fn probe(feature: SettingsFeature) -> Result<SystemCapability, meridian_protocol::ErrorBody> {
    use crate::settings_devices::run_timeout as run;
    Ok(match feature {
        SettingsFeature::Wifi | SettingsFeature::Network => {
            let output =
                run(&["nmcli", "--terse", "--fields", "TYPE,STATE", "device", "status"], None, Duration::from_secs(5))?;
            if matches!(feature, SettingsFeature::Network) {
                if output.lines().any(|line| line.ends_with(":connected")) {
                    state(S::Ready, "An active network connection is available.")
                } else {
                    state(S::TemporarilyUnavailable, "No active network connection. Check Ethernet or Wi-Fi.")
                }
            } else if !output.lines().any(|line| line.starts_with("wifi:")) {
                state(
                    S::HardwareUnavailable,
                    "No Wi-Fi adapter detected. Ethernet and virtual network connections remain available.",
                )
            } else {
                let radio =
                    run(&["nmcli", "--terse", "--fields", "WIFI-HW,WIFI", "radio"], None, Duration::from_secs(5))?;
                if radio.trim() == "enabled:enabled" {
                    state(S::Ready, "Wi-Fi adapter ready")
                } else {
                    state(
                        S::Disabled,
                        "Wi-Fi is disabled or radio blocked. Check airplane mode and the hardware switch.",
                    )
                }
            }
        }
        SettingsFeature::Displays => match meridian_services::displays::list() {
            Ok(_) => state(S::Ready, "Display configuration available"),
            Err(e) => state(classify(&e), e),
        },
        SettingsFeature::Brightness => {
            if std::fs::read_dir("/sys/class/backlight").is_ok_and(|mut d| d.next().is_some()) {
                state(S::Ready, "Built-in backlight available")
            } else {
                state(S::Unsupported, "No built-in backlight. External brightness requires a supported DDC/CI monitor.")
            }
        }
        SettingsFeature::Storage => {
            run(&["udisksctl", "status"], None, Duration::from_secs(5))?;
            state(S::Ready, "Storage operations available through UDisks2")
        }
        SettingsFeature::Bluetooth => unreachable!(),
    })
}
pub async fn query(feature: SettingsFeature) -> SystemCapability {
    if matches!(feature, SettingsFeature::Bluetooth) {
        return bluez().await;
    }
    match crate::files::blocking(move || probe(feature)).await {
        Ok(value) => value,
        Err(e) => state(classify(&e.message), e.message),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failures_remain_distinct() {
        assert_eq!(classify("Permission denied"), S::PermissionDenied);
        assert_eq!(classify("NetworkManager is not running"), S::ServiceUnavailable);
        assert_eq!(classify("Compositor does not support configuration"), S::Unsupported);
        assert_eq!(classify("Device disconnected"), S::TemporarilyUnavailable);
    }
}
