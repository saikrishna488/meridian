//! Native C++ font, graphics and visual-performance engines.
//! Initialize before GTK/threads: WebKit subprocesses inherit the scoped
//! Fontconfig profile. Configuration never overwrites system or user fonts.conf.
use meridian_protocol::{
    EngineConfig, EngineState, ErrorBody, ErrorCode, FontRenderProfile, GraphicsAcceleration, RenderPerformance,
};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    hash::{Hash, Hasher},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::OnceLock,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RenderPlan {
    pub hint_style: i32,
    pub acceleration: i32,
    pub animations: i32,
    pub transparency: i32,
    pub icon_size: i32,
}
#[repr(C)]
#[derive(Default)]
struct Metrics {
    total: u64,
    available: u64,
    pressure: f64,
    threads: u32,
}
unsafe extern "C" {
    fn meridian_cpu_activate() -> i32;
    fn meridian_audio_config(output: *mut c_char, capacity: usize) -> i32;
    fn meridian_font_xml(profile: i32, base: *const c_char, output: *mut c_char, capacity: usize) -> i32;
    fn meridian_font_activate() -> i32;
    fn meridian_font_versions(output: *mut c_char, capacity: usize) -> i32;
    fn meridian_graphics_plan(
        font: i32,
        acceleration: i32,
        performance: i32,
        animations: i32,
        transparency: i32,
    ) -> RenderPlan;
    fn meridian_performance_sample(output: *mut Metrics) -> i32;
    fn meridian_graphics_preview(
        plan: RenderPlan,
        scale: i32,
        write: extern "C" fn(*const u8, usize, *mut c_void) -> i32,
        context: *mut c_void,
    ) -> i32;
}
struct Active {
    config: EngineConfig,
    font_loaded: bool,
    status: String,
    cpu_status: String,
}
static ACTIVE: OnceLock<Active> = OnceLock::new();
static BACKENDS: OnceLock<String> = OnceLock::new();
fn path() -> PathBuf {
    glib::user_config_dir().join("meridian/engines.json")
}
fn read_at(path: &Path) -> EngineConfig {
    crate::config::read_versioned(path).unwrap_or_default()
}
pub fn config() -> EngineConfig {
    let mut value = read_at(&path());
    value.font_profile = FontRenderProfile::Crisp;
    value.acceleration = if value.gpu_enabled { GraphicsAcceleration::Accelerated } else { GraphicsAcceleration::Automatic };
    value.performance = if value.cpu_enabled { RenderPerformance::Efficient } else if value.gpu_enabled { RenderPerformance::Quality } else { RenderPerformance::Balanced };
    value.animations = !value.cpu_enabled;
    value.transparency = !value.cpu_enabled;
    value
}
fn font_number(config: &EngineConfig) -> i32 {
    if config.font_enabled { 1 } else { 0 }
}
pub(crate) fn plan(config: &EngineConfig) -> RenderPlan {
    let acceleration = if config.gpu_enabled { 1 } else { 0 };
    let performance = if config.cpu_enabled { 2 } else if config.gpu_enabled { 0 } else { 1 };
    // C ABI takes only integers and returns an owned, fixed-layout value.
    unsafe {
        meridian_graphics_plan(
            font_number(config),
            acceleration,
            performance,
            config.animations.into(),
            config.transparency.into(),
        )
    }
}
fn font_xml(config: &EngineConfig, base: &str) -> Result<Vec<u8>, String> {
    let base = CString::new(base).map_err(|_| "Invalid Fontconfig base path")?;
    let mut buffer = vec![0u8; 16384];
    let length =
        unsafe { meridian_font_xml(font_number(config), base.as_ptr(), buffer.as_mut_ptr().cast(), buffer.len()) };
    if length < 0 {
        return Err("Could not generate the native font profile".into());
    }
    buffer.truncate(length as usize);
    Ok(buffer)
}

/// # Safety
/// Call once on the main thread before GTK, logging, or any threads start.
/// Process environment changes are inherited by WebKit's rendering processes.
pub unsafe fn initialize_process() {
    if ACTIVE.get().is_some() {
        return;
    }
    let config = config();
    // A shell restarted from an older Meridian process can inherit its scoped profile.
    // Disabling restores that profile's original base rather than keeping it active.
    if !config.font_enabled {
        let scoped = glib::user_config_dir().join("meridian/rendering");
        if std::env::var_os("FONTCONFIG_FILE").is_some_and(|value| PathBuf::from(value).starts_with(&scoped)) {
            unsafe {
                if let Some(base) = std::env::var_os("MERIDIAN_BASE_FONTCONFIG_FILE") { std::env::set_var("FONTCONFIG_FILE", base); }
                else { std::env::remove_var("FONTCONFIG_FILE"); }
                std::env::remove_var("MERIDIAN_BASE_FONTCONFIG_FILE");
            }
        }
    }
    let prepare = || -> Result<(), String> {
        let directory = glib::user_config_dir().join("meridian/rendering");
        let inherited = std::env::var("FONTCONFIG_FILE").unwrap_or_else(|_| "/etc/fonts/fonts.conf".into());
        let base = if Path::new(&inherited).starts_with(&directory) {
            std::env::var("MERIDIAN_BASE_FONTCONFIG_FILE").unwrap_or_else(|_| "/etc/fonts/fonts.conf".into())
        } else {
            inherited
        };

        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        // Content-addressed files keep a second shell/diagnostic process from
        // mutating a profile still in use by an existing WebKit process.
        let contents = font_xml(&config, &base)?;
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        contents.hash(&mut hash);
        let file = directory.join(format!("fonts-{}-{:016x}.conf", font_number(&config), hash.finish()));
        if file.exists() {
            if std::fs::read(&file).map_err(|e| e.to_string())? != contents {
                return Err(
                    "Existing scoped font profile was modified; preserve and repair it before restarting".into()
                );
            }
        } else {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            let temporary = directory.join(format!("fonts-{}-{stamp}.tmp", std::process::id()));
            let write = || -> std::io::Result<()> {
                let mut output =
                    std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temporary)?;
                output.write_all(&contents)?;
                output.sync_all()?;
                std::fs::rename(&temporary, &file)?;
                std::fs::File::open(&directory)?.sync_all()
            };
            if let Err(error) = write() {
                let _ = std::fs::remove_file(&temporary);
                return Err(error.to_string());
            }
        }
        // SAFETY: initialize_process's caller guarantees a single-threaded process.
        unsafe {
            std::env::set_var("MERIDIAN_BASE_FONTCONFIG_FILE", &base);
            std::env::set_var("FONTCONFIG_FILE", &file);
        }
        if unsafe { meridian_font_activate() } != 0 {
            unsafe {
                std::env::set_var("FONTCONFIG_FILE", &base);
            }
            return Err("Fontconfig could not load the profile; using the existing font configuration".into());
        }
        Ok(())
    };
    let cpu_status = if !config.cpu_enabled { "Off: system timer policy retained".into() }
        else if unsafe { meridian_cpu_activate() } == 0 { "On: bounded timer coalescing and reduced visual workload".into() }
        else { "Timer tuning unavailable; reduced visual workload still applies".into() };
    let result = if config.font_enabled { prepare() } else { Err("Off: platform font rendering retained".into()) };
    let font_loaded = result.is_ok();
    let status =
        result.err().unwrap_or_else(|| "Grayscale antialiasing and the selected hinting profile are loaded".into());
    let _ = ACTIVE.set(Active { config, font_loaded, status, cpu_status });
}

pub(crate) fn font_sandbox_paths() -> Vec<PathBuf> {
    ["FONTCONFIG_FILE", "MERIDIAN_BASE_FONTCONFIG_FILE"]
        .into_iter()
        .filter_map(|key| std::env::var_os(key).map(PathBuf::from))
        .filter_map(|path| path.canonicalize().ok())
        .filter(|path| path.is_file())
        .collect()
}

pub(crate) fn apply_gtk_font_settings() {
    if !ACTIVE.get().is_some_and(|active| active.font_loaded) {
        return;
    }
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_xft_antialias(1);
        settings.set_gtk_xft_rgba(Some("none"));
        settings.set_gtk_xft_hinting(1);
        settings.set_gtk_xft_hintstyle(Some("hintfull"));
    }
}

pub fn active_config() -> EngineConfig {
    ACTIVE.get().map(|active| active.config.clone()).unwrap_or_default()
}
fn versions() -> String {
    BACKENDS
        .get_or_init(|| {
            let mut buffer = [0u8; 256];
            if unsafe { meridian_font_versions(buffer.as_mut_ptr().cast(), buffer.len()) } < 0 {
                return "Native font backend unavailable".into();
            }
            unsafe { CStr::from_ptr(buffer.as_ptr().cast()) }.to_string_lossy().into_owned()
        })
        .clone()
}
pub fn state() -> EngineState {
    let config = config();
    let mut active = active_config();
    // Visual workload settings are applied live through the shared UI bridge.
    active.audio_enabled = config.audio_enabled;
    active.performance = config.performance;
    active.animations = config.animations;
    active.transparency = config.transparency;
    let render = plan(&active);
    let mut metrics = Metrics::default();
    let sampled = unsafe { meridian_performance_sample(&mut metrics) } == 0;
    EngineState {
        restart_required: active.font_enabled != config.font_enabled || active.gpu_enabled != config.gpu_enabled || active.cpu_enabled != config.cpu_enabled,
        cpu_status: ACTIVE.get().map(|value| value.cpu_status.clone()).unwrap_or_else(|| "Not initialized".into()),
        audio_status: audio_status(&config),
        font_config_loaded: ACTIVE.get().is_some_and(|active| active.font_loaded),
        font_status: ACTIVE
            .get()
            .map(|active| active.status.clone())
            .unwrap_or_else(|| "Font profile has not been initialized in this process".into()),
        backends: versions(),
        animations_enabled: render.animations != 0,
        transparency_enabled: render.transparency != 0,
        icon_raster_size: render.icon_size as u32,
        memory_total_mib: sampled.then_some(metrics.total as f64 / 1024.0),
        memory_available_mib: sampled.then_some(metrics.available as f64 / 1024.0),
        memory_pressure_avg10: (sampled && metrics.pressure >= 0.0 && metrics.pressure.is_finite())
            .then_some(metrics.pressure),
        cpu_threads: metrics.threads,
        config,
        active,
    }
}
pub fn set(mut config: EngineConfig) -> Result<EngineState, ErrorBody> {
    // Fixed presets behind four switches; legacy fields no longer expose knobs.
    config.font_profile = FontRenderProfile::Crisp;
    config.acceleration = if config.gpu_enabled { GraphicsAcceleration::Accelerated } else { GraphicsAcceleration::Automatic };
    config.performance = if config.cpu_enabled { RenderPerformance::Efficient } else if config.gpu_enabled { RenderPerformance::Quality } else { RenderPerformance::Balanced };
    config.animations = !config.cpu_enabled;
    config.transparency = !config.cpu_enabled;
    let previous = self::config();
    if config.audio_enabled != previous.audio_enabled || config.audio_enabled {
        apply_audio(config.audio_enabled)?;
    }
    if let Err(error) = crate::config::save_versioned(&path(), &config) {
        if let Err(rollback) = apply_audio(previous.audio_enabled) { log::error!("Audio engine rollback failed: {}", rollback.message); }
        return Err(error);
    }
    Ok(state())
}

fn audio_contents() -> Result<Vec<u8>, ErrorBody> {
    let mut output = [0u8; 1024];
    let size = unsafe { meridian_audio_config(output.as_mut_ptr().cast(), output.len()) };
    if size < 0 { return Err(ErrorBody::new(ErrorCode::Failed, "Audio engine could not prepare its profile")); }
    Ok(output[..size as usize].to_vec())
}
fn audio_paths() -> Vec<PathBuf> {
    ["client.conf.d", "client-rt.conf.d", "pipewire-pulse.conf.d"].into_iter()
        .map(|directory| glib::user_config_dir().join("pipewire").join(directory).join("90-meridian-audio.conf")).collect()
}
fn audio_status(config: &EngineConfig) -> String {
    if !config.audio_enabled { return "Off. Reopen playback apps; PulseAudio clients need an audio-service restart or reboot to restore the platform profile.".into(); }
    let Ok(expected) = audio_contents() else { return "Audio profile unavailable".into(); };
    if audio_paths().iter().all(|path| read_audio_file(path).ok().as_ref() == Some(&expected)) {
        "High-quality resampling configured. Reopen playback apps; PulseAudio clients need an audio-service restart or reboot.".into()
    } else { "Audio profile missing or modified. Preserve external changes, then turn the engine off and on to repair it.".into() }
}
fn read_audio_file(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > 4096 {
        return Err(std::io::Error::other("Audio profile must be a small regular file; preserve external changes before toggling"));
    }
    let mut contents = Vec::new();
    std::fs::File::open(path)?.take(4097).read_to_end(&mut contents)?;
    if contents.len() > 4096 { return Err(std::io::Error::other("Audio profile exceeds the size limit")); }
    Ok(contents)
}
fn write_audio_file(path: &Path, contents: &[u8]) -> Result<(), ErrorBody> {
    use gtk::prelude::*;
    let error = |value: String| ErrorBody::new(ErrorCode::Failed, value);
    let parent = path.parent().ok_or_else(|| error("Audio configuration directory unavailable".into()))?;
    std::fs::create_dir_all(parent).map_err(|e| error(e.to_string()))?;
    gtk::gio::File::for_path(path).replace_contents(contents, None, false,
        gtk::gio::FileCreateFlags::PRIVATE | gtk::gio::FileCreateFlags::REPLACE_DESTINATION, gtk::gio::Cancellable::NONE)
        .map_err(|e| error(e.to_string()))?;
    Ok(())
}
fn apply_audio(enabled: bool) -> Result<(), ErrorBody> {
    let expected = audio_contents()?;
    let paths = audio_paths();
    let mut previous = Vec::new();
    for path in &paths {
        match read_audio_file(path) {
            Ok(contents) if contents == expected => previous.push(Some(contents)),
            Ok(_) => return Err(ErrorBody::new(ErrorCode::Failed, "Meridian's audio profile was modified externally; preserve it before changing the engine")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => previous.push(None),
            Err(error) => return Err(ErrorBody::new(ErrorCode::Failed, error.to_string())),
        }
    }
    for (index, path) in paths.iter().enumerate() {
        let result = if enabled { write_audio_file(path, &expected) }
            else if previous[index].is_some() { std::fs::remove_file(path).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string())) }
            else { Ok(()) };
        if let Err(error) = result {
            for (path, old) in paths[..=index].iter().zip(&previous) {
                let restored = if let Some(contents) = old { write_audio_file(path, contents) }
                    else { match std::fs::remove_file(path) { Ok(()) => Ok(()), Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()), Err(e) => Err(ErrorBody::new(ErrorCode::Failed,e.to_string())) } };
                if let Err(rollback) = restored { log::error!("Audio profile rollback failed: {}", rollback.message); }
            }
            return Err(error);
        }
    }
    Ok(())
}

extern "C" fn collect(data: *const u8, length: usize, context: *mut c_void) -> i32 {
    // C++ invokes this synchronously with live buffers and a borrowed Vec.
    if length == 0 {
        return 0;
    }
    let output = unsafe { &mut *context.cast::<Vec<u8>>() };
    if length > 4 * 1024 * 1024 || output.len().saturating_add(length) > 4 * 1024 * 1024 {
        return -1;
    }
    // Avoid unwinding across the C ABI if the allocation fails.
    if output.try_reserve(length).is_err() {
        return -1;
    }
    output.extend_from_slice(unsafe { std::slice::from_raw_parts(data, length) });
    0
}
pub fn preview(scale: i32) -> Result<Vec<u8>, ErrorBody> {
    let mut bytes = Vec::new();
    let result =
        unsafe { meridian_graphics_preview(plan(&config()), scale, collect, (&mut bytes as *mut Vec<u8>).cast()) };
    if result != 0 {
        return Err(ErrorBody::new(ErrorCode::Failed, "Native rendering preview failed"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_switches_select_fixed_policies() {
        let mut settings = EngineConfig::default();
        let crisp = plan(&settings);
        settings.font_enabled = false;
        assert_ne!(crisp.hint_style, plan(&settings).hint_style);
        settings.cpu_enabled = true;
        let efficient = plan(&settings);
        assert_eq!((efficient.animations, efficient.transparency, efficient.icon_size), (0, 0, 96));
        settings.gpu_enabled = true;
        assert_eq!(plan(&settings).acceleration, 1);
    }
    #[test]
    fn native_font_configuration_escapes_include_path() {
        let xml = String::from_utf8(font_xml(&EngineConfig::default(), "/tmp/a&b<fonts>.conf").unwrap()).unwrap();
        assert!(xml.contains("/tmp/a&amp;b&lt;fonts&gt;.conf"));
        assert!(xml.contains("hintfull"));
        assert!(xml.contains("<const>none</const>"));
    }
    #[test]
    fn native_preview_renders_scaled_png_and_rejects_unbounded_sizes() {
        for scale in [1, 2, 3] {
            let bytes = preview(scale).unwrap();
            assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
            assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 600 * scale as u32);
            assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 180 * scale as u32);
        }
        assert!(preview(0).is_err());
        assert!(preview(4).is_err());
    }
    #[test]
    fn native_metrics_are_real_or_explicitly_unavailable() {
        let mut metrics = Metrics::default();
        if unsafe { meridian_performance_sample(&mut metrics) } == 0 {
            assert!(metrics.total > 0);
            assert!(metrics.available <= metrics.total);
            assert!(metrics.threads > 0);
        }
        assert!(unsafe { meridian_performance_sample(std::ptr::null_mut()) } < 0);
    }
}
