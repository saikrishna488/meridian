//! Monitor configuration through the compositor's output-management protocol.
use meridian_protocol::{DisplayConfig, DisplayMode, DisplayOutput};
use std::collections::HashMap;
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_output::Transform, wl_registry::WlRegistry};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum, event_created_child};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1 as ConfigHead,
    zwlr_output_configuration_v1::{self as config, ZwlrOutputConfigurationV1 as Config},
    zwlr_output_head_v1::{self as head, ZwlrOutputHeadV1 as Head},
    zwlr_output_manager_v1::{self as manager, ZwlrOutputManagerV1 as Manager},
    zwlr_output_mode_v1::{self as mode, ZwlrOutputModeV1 as Mode},
};
#[derive(Default)]
struct State {
    heads: HashMap<Head, DisplayOutput>,
    modes: HashMap<Mode, (Head, DisplayMode)>,
    current: HashMap<Head, Mode>,
    serial: u32,
    result: Option<bool>,
}
impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<Manager, ()> for State {
    fn event(state: &mut Self, _: &Manager, event: manager::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match event {
            manager::Event::Head { head } => {
                state.heads.insert(
                    head,
                    DisplayOutput {
                        name: String::new(),
                        description: String::new(),
                        enabled: false,
                        x: 0,
                        y: 0,
                        scale: 1.,
                        transform: 0,
                        current: None,
                        modes: vec![],
                    },
                );
            }
            manager::Event::Done { serial } => state.serial = serial,
            _ => {}
        }
    }
    event_created_child!(State, Manager, [manager::EVT_HEAD_OPCODE => (Head, ())]);
}
impl Dispatch<Head, ()> for State {
    fn event(state: &mut Self, proxy: &Head, event: head::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        let Some(output) = state.heads.get_mut(proxy) else {
            return;
        };
        match event {
            head::Event::Name { name } => output.name = name,
            head::Event::Description { description } => output.description = description,
            head::Event::Enabled { enabled } => output.enabled = enabled != 0,
            head::Event::Position { x, y } => {
                output.x = x;
                output.y = y;
            }
            head::Event::Scale { scale } => output.scale = scale,
            head::Event::Transform { transform } => {
                output.transform = match transform {
                    WEnum::Value(v) => v as u32,
                    WEnum::Unknown(v) => v,
                }
            }
            head::Event::CurrentMode { mode } => {
                state.current.insert(proxy.clone(), mode);
            }
            head::Event::Mode { mode } => {
                state
                    .modes
                    .insert(mode, (proxy.clone(), DisplayMode { width: 0, height: 0, refresh: 0, preferred: false }));
            }
            head::Event::Finished => {
                state.heads.remove(proxy);
            }
            _ => {}
        }
    }
    event_created_child!(State, Head, [head::EVT_MODE_OPCODE => (Mode, ())]);
}
impl Dispatch<Mode, ()> for State {
    fn event(state: &mut Self, proxy: &Mode, event: mode::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        let Some((_, value)) = state.modes.get_mut(proxy) else {
            return;
        };
        match event {
            mode::Event::Size { width, height } => {
                value.width = width;
                value.height = height;
            }
            mode::Event::Refresh { refresh } => value.refresh = refresh,
            mode::Event::Preferred => value.preferred = true,
            mode::Event::Finished => {
                state.modes.remove(proxy);
            }
            _ => {}
        }
    }
}
impl Dispatch<Config, ()> for State {
    fn event(state: &mut Self, _: &Config, event: config::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        state.result = match event {
            config::Event::Succeeded => Some(true),
            config::Event::Failed | config::Event::Cancelled => Some(false),
            _ => state.result,
        };
    }
}
impl Dispatch<ConfigHead, ()> for State {
    fn event(
        _: &mut Self,
        _: &ConfigHead,
        _: <ConfigHead as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
fn connect() -> Result<(wayland_client::EventQueue<State>, Manager, State), String> {
    let connection = Connection::connect_to_env().map_err(|e| e.to_string())?;
    let (globals, mut queue) = registry_queue_init::<State>(&connection).map_err(|e| e.to_string())?;
    let manager = globals
        .bind(&queue.handle(), 1..=2, ())
        .map_err(|_| "This compositor does not support monitor configuration".to_string())?;
    let mut state = State::default();
    queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
    queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
    Ok((queue, manager, state))
}
fn outputs(state: &State) -> Vec<DisplayOutput> {
    let mut result: Vec<_> = state
        .heads
        .iter()
        .map(|(head, output)| {
            let mut output = output.clone();
            output.modes =
                state.modes.values().filter(|(owner, _)| owner == head).map(|(_, mode)| mode.clone()).collect();
            output.modes.sort_by_key(|m| (std::cmp::Reverse(m.width * m.height), std::cmp::Reverse(m.refresh)));
            output.current = state.current.get(head).and_then(|id| state.modes.get(id)).map(|(_, m)| m.clone());
            output
        })
        .collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}
pub fn list() -> Result<Vec<DisplayOutput>, String> {
    let (_, _, state) = connect()?;
    Ok(outputs(&state))
}
pub fn snapshot(outputs: &[DisplayOutput]) -> Vec<DisplayConfig> {
    outputs
        .iter()
        .map(|o| {
            let m = o.current.as_ref().or_else(|| o.modes.iter().find(|m| m.preferred)).or(o.modes.first());
            DisplayConfig {
                name: o.name.clone(),
                enabled: o.enabled,
                x: o.x,
                y: o.y,
                scale: o.scale,
                transform: o.transform,
                width: m.map_or(0, |m| m.width),
                height: m.map_or(0, |m| m.height),
                refresh: m.map_or(0, |m| m.refresh),
            }
        })
        .collect()
}
pub fn validate(edits: &[DisplayConfig], current: &[DisplayOutput]) -> Result<(), String> {
    if edits.len() != current.len() || !edits.iter().any(|e| e.enabled) {
        return Err("Keep at least one monitor enabled and refresh the monitor list after hot-plugging".into());
    }
    let mut seen = std::collections::HashSet::new();
    for edit in edits {
        let output = current.iter().find(|o| o.name == edit.name).ok_or("Monitor disconnected; refresh Displays")?;
        if !seen.insert(&edit.name)
            || !edit.scale.is_finite()
            || !(0.5..=3.).contains(&edit.scale)
            || edit.transform > 7
            || edit.x.unsigned_abs() > 32768
            || edit.y.unsigned_abs() > 32768
        {
            return Err("Invalid monitor arrangement or scale".into());
        }
        if edit.enabled
            && !output
                .modes
                .iter()
                .any(|m| m.width == edit.width && m.height == edit.height && m.refresh == edit.refresh)
        {
            return Err("Choose a resolution advertised by this monitor".into());
        }
    }
    Ok(())
}
pub fn apply(edits: &[DisplayConfig]) -> Result<(), String> {
    let (mut queue, manager, mut state) = connect()?;
    validate(edits, &outputs(&state))?;
    for testing in [true, false] {
        let configuration = manager.create_configuration(state.serial, &queue.handle(), ());
        for (head, output) in &state.heads {
            let edit = edits.iter().find(|e| e.name == output.name).ok_or("Monitor disconnected")?;
            if !edit.enabled {
                configuration.disable_head(head);
                continue;
            }
            let config = configuration.enable_head(head, &queue.handle(), ());
            let mode = state
                .modes
                .iter()
                .find(|(_, (owner, m))| {
                    owner == head && m.width == edit.width && m.height == edit.height && m.refresh == edit.refresh
                })
                .map(|(id, _)| id)
                .ok_or("Resolution is no longer available")?;
            config.set_mode(mode);
            config.set_position(edit.x, edit.y);
            config.set_scale(edit.scale);
            config.set_transform(Transform::try_from(edit.transform).map_err(|_| "Invalid orientation")?);
        }
        state.result = None;
        if testing {
            configuration.test();
        } else {
            configuration.apply();
        }
        queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
        while state.result.is_none() {
            queue.blocking_dispatch(&mut state).map_err(|e| e.to_string())?;
        }
        configuration.destroy();
        if state.result != Some(true) {
            return Err("The compositor rejected this display configuration".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_dark_desktop_unsupported_modes_and_invalid_scaling() {
        let mode = DisplayMode { width: 1920, height: 1080, refresh: 60000, preferred: true };
        let output = DisplayOutput {
            name: "HDMI-A-1".into(),
            description: "Monitor".into(),
            enabled: true,
            x: 0,
            y: 0,
            scale: 1.,
            transform: 0,
            current: Some(mode.clone()),
            modes: vec![mode],
        };
        let mut edits = snapshot(std::slice::from_ref(&output));
        assert!(validate(&edits, std::slice::from_ref(&output)).is_ok());
        edits[0].enabled = false;
        assert!(validate(&edits, std::slice::from_ref(&output)).is_err());
        edits[0].enabled = true;
        edits[0].width = 800;
        assert!(validate(&edits, std::slice::from_ref(&output)).is_err());
        edits[0].width = 1920;
        edits[0].scale = f64::NAN;
        assert!(validate(&edits, &[output]).is_err());
    }
}
