//! PipeWire device discovery and WirePlumber defaults, with bounded commands.
use meridian_protocol::{
    AudioAction, AudioDevice, AudioState, BluetoothAudioMode, BluetoothAudioQuality, BluetoothAudioState, ErrorBody,
    ErrorCode,
};
use serde_json::Value;
use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf, time::Duration};
fn error(message: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, message.to_string())
}
fn run(args: &[&str]) -> Result<String, ErrorBody> {
    crate::settings_devices::run_timeout(args, None, Duration::from_secs(5))
}
fn devices(data: &str) -> Result<Vec<AudioDevice>, ErrorBody> {
    let objects: Vec<Value> = serde_json::from_str(data).map_err(|_| error("Could not read PipeWire audio devices"))?;
    let default = |key: &str| -> Option<String> {
        objects
            .iter()
            .filter(|o| o["type"] == "PipeWire:Interface:Metadata" && o["props"]["metadata.name"] == "default")
            .filter_map(|o| o["metadata"].as_array())
            .flatten()
            .find(|m| m["subject"] == 0 && m["key"] == key)
            .and_then(|m| {
                m["value"].get("name").and_then(Value::as_str).map(str::to_owned).or_else(|| {
                    m["value"]
                        .as_str()
                        .and_then(|s| serde_json::from_str::<Value>(s).ok())
                        .and_then(|v| v["name"].as_str().map(str::to_owned))
                })
            })
    };
    let output = default("default.audio.sink");
    let input = default("default.audio.source");
    let mut result = vec![];
    for object in objects.iter().filter(|o| o["type"] == "PipeWire:Interface:Node") {
        let props = &object["info"]["props"];
        let kind = match props["media.class"].as_str() {
            Some("Audio/Sink") => "output",
            Some("Audio/Source") => "input",
            _ => continue,
        };
        let Some(id) = object["id"].as_u64().and_then(|id| u32::try_from(id).ok()) else { continue };
        let Some(name) = props["node.name"].as_str() else { continue };
        // Monitor ports and application streams are not physical input choices.
        if kind == "input" && (name.ends_with(".monitor") || props["device.class"] == "monitor") {
            continue;
        }
        let description = props["node.description"].as_str().or_else(|| props["node.nick"].as_str()).unwrap_or(name);
        result.push(AudioDevice {
            id,
            name: name.into(),
            description: description.into(),
            kind: kind.into(),
            default: (if kind == "output" { &output } else { &input }).as_deref() == Some(name),
        });
    }
    result.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.description.to_lowercase().cmp(&b.description.to_lowercase())));
    Ok(result)
}
fn volume(text: &str) -> Option<(u8, bool)> {
    let mut parts = text.trim().strip_prefix("Volume:")?.split_whitespace();
    let level: f64 = parts.next()?.parse().ok()?;
    if !level.is_finite() || level < 0. {
        return None;
    }
    Some(((level * 100.).round().clamp(0., 100.) as u8, parts.any(|part| part == "[MUTED]")))
}
pub fn list() -> Result<AudioState, ErrorBody> {
    let devices = devices(&run(&["pw-dump", "-N"])?)?;
    let read = |kind: &str| {
        devices
            .iter()
            .find(|d| d.kind == kind && d.default)
            .and_then(|d| run(&["wpctl", "get-volume", &d.id.to_string()]).ok())
            .and_then(|s| volume(&s))
    };
    let output = read("output");
    let input = read("input");
    Ok(AudioState {
        devices,
        output_volume: output.map(|v| v.0),
        input_volume: input.map(|v| v.0),
        output_muted: output.map(|v| v.1),
        input_muted: input.map(|v| v.1),
    })
}
pub fn action(choice: AudioAction) -> Result<AudioState, ErrorBody> {
    if choice.action == "virtual-create" || choice.action == "virtual-remove" {
        if choice.id != 0 || !choice.name.is_empty() || choice.percent.is_some() {
            return Err(error("Invalid virtual output request"));
        }
        let modules = run(&["pactl", "list", "short", "modules"])?;
        let matching: Vec<&str> = modules
            .lines()
            .filter(|line| line.contains("module-null-sink") && line.contains("sink_name=meridian_virtual_output"))
            .filter_map(|line| line.split_whitespace().next())
            .collect();
        if choice.action == "virtual-create" && matching.is_empty() {
            run(&[
                "pactl",
                "load-module",
                "module-null-sink",
                "sink_name=meridian_virtual_output",
                "sink_properties=device.description=Meridian-Virtual-Output",
            ])?;
        } else if choice.action == "virtual-remove" {
            for id in matching {
                run(&["pactl", "unload-module", id])?;
            }
        }
        return list();
    }
    if !["default", "mute", "unmute", "volume"].contains(&choice.action.as_str())
        || choice.percent.is_some_and(|p| p > 100)
        || (choice.action == "volume") != choice.percent.is_some()
    {
        return Err(error("Invalid audio control"));
    }
    // Resolve again before using a transient PipeWire id, and verify its name.
    let current = devices(&run(&["pw-dump", "-N"])?)?;
    if !current.iter().any(|d| d.id == choice.id && d.name == choice.name) {
        return Err(error("This audio device disconnected. Refresh and choose another device."));
    }
    let id = choice.id.to_string();
    match choice.action.as_str() {
        "default" => {
            run(&["wpctl", "set-default", &id])?;
        }
        "mute" => {
            run(&["wpctl", "set-mute", &id, "1"])?;
        }
        "unmute" => {
            run(&["wpctl", "set-mute", &id, "0"])?;
        }
        "volume" => {
            run(&["wpctl", "set-volume", &id, &format!("{}%", choice.percent.unwrap())])?;
        }
        _ => unreachable!(),
    }
    list()
}

fn bluetooth_quality_config() -> PathBuf {
    gtk::glib::user_config_dir().join("wireplumber/wireplumber.conf.d/90-meridian-bluetooth.conf")
}

fn active_bluetooth_codec() -> (Option<String>, Option<String>) {
    let Ok(data) = run(&["pw-dump", "-N"]) else { return (None, None) };
    let Ok(objects) = serde_json::from_str::<Vec<Value>>(&data) else { return (None, None) };
    objects
        .iter()
        .filter(|o| o["type"] == "PipeWire:Interface:Node")
        .filter_map(|o| {
            let props = &o["info"]["props"];
            if props["media.class"] != "Audio/Sink" {
                return None;
            }
            Some((
                props["api.bluez5.codec"].as_str()?.to_owned(),
                props["node.description"].as_str().or_else(|| props["node.name"].as_str())?.to_owned(),
            ))
        })
        .next()
        .map(|(codec, name)| (Some(codec), Some(name)))
        .unwrap_or((None, None))
}

pub fn bluetooth_audio() -> BluetoothAudioState {
    let mode = std::fs::read_to_string(bluetooth_quality_config())
        .ok()
        .filter(|text| text.contains("bluez5.a2dp.ldac.quality = \"hq\""))
        .map_or(BluetoothAudioMode::Auto, |_| BluetoothAudioMode::HighestQuality);
    let (active_codec, active_device) = active_bluetooth_codec();
    BluetoothAudioState { mode, active_codec, active_device }
}

pub fn set_bluetooth_audio(choice: BluetoothAudioQuality) -> Result<BluetoothAudioState, ErrorBody> {
    let quality = match choice.mode {
        BluetoothAudioMode::Auto => "auto",
        BluetoothAudioMode::HighestQuality => "hq",
    };
    let path = bluetooth_quality_config();
    let parent = path.parent().ok_or_else(|| error("WirePlumber configuration folder is unavailable"))?;
    std::fs::create_dir_all(parent).map_err(error)?;
    let temporary = path.with_extension(format!("conf.tmp-{}", std::process::id()));
    let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temporary).map_err(error)?;
    let config = format!(
        "# Meridian Bluetooth audio preference. Managed by Meridian Settings.\n\
         monitor.bluez.rules = [\n\
           {{\n\
             matches = [ {{ device.name = \"~bluez_card.*\" }} ]\n\
             actions = {{ update-props = {{ bluez5.a2dp.ldac.quality = \"{quality}\" }} }}\n\
           }}\n\
         ]\n"
    );
    if let Err(e) = file.write_all(config.as_bytes()).and_then(|_| file.sync_all()) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error(e));
    }
    std::fs::rename(&temporary, &path).map_err(error)?;
    crate::settings_devices::run_timeout(
        &["systemctl", "--user", "restart", "wireplumber.service"],
        None,
        Duration::from_secs(10),
    )
    .map_err(|_| error("Preference saved, but WirePlumber could not restart. Restart it from your user session to apply the change."))?;
    Ok(bluetooth_audio())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_only_audio_devices_and_matches_defaults_by_name() {
        let data = serde_json::json!([
            {"id":50,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Audio/Sink","node.name":"speakers","node.description":"Built-in speakers"}}},
            {"id":51,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Audio/Source","node.name":"mic"}}},
            {"id":52,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Stream/Output/Audio","node.name":"browser"}}},
            {"id":53,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Audio/Source","node.name":"speakers.monitor"}}},
            {"type":"PipeWire:Interface:Metadata","props":{"metadata.name":"default"},"metadata":[{"subject":0,"key":"default.audio.sink","value":{"name":"speakers"}},{"subject":0,"key":"default.audio.source","value":"{\"name\":\"mic\"}"}]}
        ]).to_string();
        let result = devices(&data).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|d| d.default));
        assert!(devices("invalid JSON").is_err());
        assert_eq!(volume("Volume: 0.42 [MUTED]"), Some((42, true)));
        assert_eq!(volume("Volume: 1.50"), Some((100, false)));
        assert_eq!(volume("Volume: NaN"), None);
        assert_eq!(volume("Volume: -1"), None);
    }
}
