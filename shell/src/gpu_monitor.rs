//! Lightweight per-device GPU utilization. No external monitoring process is started.
//! Prefer driver-reported sysfs busy%, NVIDIA NVML, then DRM fdinfo engine time.
//! DRM fdinfo provides observed client load rather than an authoritative global GPU counter.
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Default)]
pub struct GpuMonitor {
    previous: HashMap<String, HashMap<String, EngineCounters>>,
}

#[derive(Debug, Clone, Default)]
struct EngineCounters {
    nanos: HashMap<String, u64>,
    cycles: HashMap<String, (u64, u64)>,
}

#[derive(Debug, Clone, Default)]
pub struct GpuReading {
    pub percent: Option<f64>,
    pub used: Option<f64>,
    pub total: Option<f64>,
    pub backend: &'static str,
}

#[derive(Debug, Clone)]
struct GpuDevice {
    card: String,
    pci: Option<String>,
    driver: String,
    path: PathBuf,
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap_or_default().trim().to_string()
}

fn card_number(name: &str) -> Option<usize> {
    name.strip_prefix("card")
        .and_then(|n| (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).then_some(n))?
        .parse()
        .ok()
}

fn devices() -> Vec<GpuDevice> {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else { return vec![] };
    let mut devices: Vec<_> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            card_number(&name)?;
            let path = entry.path().join("device");
            if !path.exists() {
                return None;
            }
            let properties = read(path.join("uevent"));
            let prop = |key: &str| properties.lines().find_map(|line| line.strip_prefix(key)).map(str::to_owned);
            Some(GpuDevice {
                card: name,
                pci: prop("PCI_SLOT_NAME="),
                driver: prop("DRIVER=").unwrap_or_default(),
                path,
            })
        })
        .collect();
    devices.sort_by_key(|d| card_number(&d.card));
    devices
}

fn parse_fdinfo(contents: &str) -> Option<(String, EngineCounters)> {
    let mut client = None;
    let mut counters = EngineCounters::default();
    for line in contents.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        match key {
            "drm-client-id" => client = value.split_whitespace().next().map(str::to_owned),
            _ if key.starts_with("drm-engine-") => {
                if let Some(ns) = value.strip_suffix("ns").and_then(|v| v.trim().parse().ok()) {
                    counters.nanos.insert(key.trim_start_matches("drm-engine-").into(), ns);
                }
            }
            _ => {}
        }
    }
    // Xe reports cycles and total-cycles instead of time in nanoseconds.
    for line in contents.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let Some(engine) = key.strip_prefix("drm-cycles-") else { continue };
        let cycles = value.split_whitespace().next().and_then(|v| v.parse::<u64>().ok());
        let total = contents.lines().find_map(|other| {
            let (k, v) = other.split_once(':')?;
            (k == format!("drm-total-cycles-{engine}"))
                .then(|| v.split_whitespace().next()?.parse::<u64>().ok())
                .flatten()
        });
        if let (Some(cycles), Some(total)) = (cycles, total) {
            counters.cycles.insert(engine.to_owned(), (cycles, total));
        }
    }
    client.filter(|_| !counters.nanos.is_empty() || !counters.cycles.is_empty()).map(|id| (id, counters))
}

fn fdinfo_device(contents: &str, devices: &[GpuDevice]) -> Option<String> {
    let pci = contents.lines().find_map(|line| line.strip_prefix("drm-pdev:"))?.trim();
    devices.iter().find(|device| device.pci.as_deref() == Some(pci)).map(|device| device.card.clone())
}

fn collect_fdinfo(devices: &[GpuDevice]) -> HashMap<String, HashMap<String, EngineCounters>> {
    let mut result: HashMap<String, HashMap<String, EngineCounters>> = HashMap::new();
    let Ok(processes) = fs::read_dir("/proc") else { return result };
    // One DRM client can expose multiple file descriptors. Keep it once per GPU.
    let mut seen: HashSet<(String, String)> = HashSet::new();
    for process in processes.flatten() {
        if !process.file_name().to_string_lossy().bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let Ok(fds) = fs::read_dir(process.path().join("fdinfo")) else { continue };
        for fd in fds.flatten() {
            let info = read(fd.path());
            if !info.contains("drm-client-id:") {
                continue;
            }
            let Some(card) = fdinfo_device(&info, devices) else { continue };
            let Some((client, counters)) = parse_fdinfo(&info) else { continue };
            if seen.insert((card.clone(), client.clone())) {
                result.entry(card).or_default().insert(client, counters);
            }
        }
    }
    result
}

fn fdinfo_utilization(
    current: &HashMap<String, EngineCounters>,
    previous: &HashMap<String, EngineCounters>,
    elapsed: Duration,
) -> Option<f64> {
    let nanos = elapsed.as_nanos();
    if nanos == 0 {
        return None;
    }
    // An engine may have multiple clients. Sum each engine across clients,
    // then take the busiest engine rather than summing independent engines.
    let mut engines: HashMap<&str, f64> = HashMap::new();
    for (client, counters) in current {
        let Some(old) = previous.get(client) else { continue };
        for (engine, now) in &counters.nanos {
            if let Some(delta) = old.nanos.get(engine).and_then(|old| now.checked_sub(*old)) {
                *engines.entry(engine).or_default() += delta as f64 / nanos as f64;
            }
        }
        for (engine, (cycles, total)) in &counters.cycles {
            if let Some((old_cycles, old_total)) = old.cycles.get(engine)
                && let (Some(work), Some(capacity)) = (cycles.checked_sub(*old_cycles), total.checked_sub(*old_total))
                && capacity > 0
            {
                *engines.entry(engine).or_default() += work as f64 / capacity as f64;
            }
        }
    }
    engines.values().copied().reduce(f64::max).map(|value| (value * 100.0).clamp(0.0, 100.0))
}

// NVML is loaded dynamically: non-NVIDIA machines require no NVIDIA libraries.
#[repr(C)]
#[derive(Default)]
struct NvmlUtilization {
    gpu: u32,
    memory: u32,
}
#[repr(C)]
#[derive(Default)]
struct NvmlMemory {
    total: u64,
    free: u64,
    used: u64,
}

struct Nvml {
    library: *mut c_void,
    shutdown: unsafe extern "C" fn() -> c_int,
    by_bus: unsafe extern "C" fn(*const c_char, *mut *mut c_void) -> c_int,
    utilization: unsafe extern "C" fn(*mut c_void, *mut NvmlUtilization) -> c_int,
    memory: Option<unsafe extern "C" fn(*mut c_void, *mut NvmlMemory) -> c_int>,
}
impl Nvml {
    fn open() -> Option<Self> {
        // SAFETY: dlsym symbols have documented NVIDIA NVML signatures and
        // are called only while the library handle remains loaded.
        unsafe {
            let name = CString::new("libnvidia-ml.so.1").ok()?;
            let library = libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
            if library.is_null() {
                return None;
            }
            let symbol = |name: &CStr| libc::dlsym(library, name.as_ptr());
            let init = symbol(c"nvmlInit_v2");
            let shutdown = symbol(c"nvmlShutdown");
            let by_bus = symbol(c"nvmlDeviceGetHandleByPciBusId_v2");
            let utilization = symbol(c"nvmlDeviceGetUtilizationRates");
            let memory = symbol(c"nvmlDeviceGetMemoryInfo");
            if init.is_null() || shutdown.is_null() || by_bus.is_null() || utilization.is_null() {
                libc::dlclose(library);
                return None;
            }
            let init: unsafe extern "C" fn() -> c_int = std::mem::transmute(init);
            if init() != 0 {
                libc::dlclose(library);
                return None;
            }
            Some(Self {
                library,
                shutdown: std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> c_int>(shutdown),
                by_bus: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(*const c_char, *mut *mut c_void) -> c_int,
                >(by_bus),
                utilization: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(*mut c_void, *mut NvmlUtilization) -> c_int,
                >(utilization),
                memory: (!memory.is_null()).then(|| {
                    std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*mut c_void, *mut NvmlMemory) -> c_int>(
                        memory,
                    )
                }),
            })
        }
    }
    fn reading(&self, bus: &str) -> Option<GpuReading> {
        let bus = CString::new(bus).ok()?;
        let mut device = std::ptr::null_mut();
        // SAFETY: the resolved NVML functions are invoked with initialized
        // out-parameters; a successful lookup yields a device owned by NVML.
        unsafe {
            if (self.by_bus)(bus.as_ptr(), &mut device) != 0 || device.is_null() {
                return None;
            }
            let mut utilization = NvmlUtilization::default();
            if (self.utilization)(device, &mut utilization) != 0 {
                return None;
            }
            let mut reading = GpuReading {
                percent: Some(f64::from(utilization.gpu.min(100))),
                backend: "NVML",
                ..Default::default()
            };
            if let Some(memory) = self.memory {
                let mut usage = NvmlMemory::default();
                if memory(device, &mut usage) == 0 {
                    reading.used = Some(usage.used as f64);
                    reading.total = Some(usage.total as f64);
                }
            }
            Some(reading)
        }
    }
}
impl Drop for Nvml {
    fn drop(&mut self) {
        // SAFETY: the NVML library is initialized and only used on this thread.
        unsafe {
            (self.shutdown)();
            libc::dlclose(self.library);
        }
    }
}

impl GpuMonitor {
    pub fn sample(&mut self, elapsed: Option<Duration>) -> HashMap<String, GpuReading> {
        let devices = devices();
        let current = collect_fdinfo(&devices);
        let nvml = Nvml::open();
        let mut readings = HashMap::new();
        for device in &devices {
            let sysfs = read(device.path.join("gpu_busy_percent")).parse::<u64>().ok().map(|v| v.min(100) as f64);
            let mut result = GpuReading::default();
            if let Some(pci) = &device.pci
                && device.driver == "nvidia"
                && let Some(reading) = nvml.as_ref().and_then(|n| n.reading(pci))
            {
                result = reading;
            }
            if result.percent.is_none() {
                if let Some(usage) = sysfs {
                    result.percent = Some(usage);
                    result.backend = "sysfs";
                } else if let (Some(time), Some(now), Some(old)) =
                    (elapsed, current.get(&device.card), self.previous.get(&device.card))
                {
                    result.percent = fdinfo_utilization(now, old, time);
                    if result.percent.is_some() {
                        result.backend = "DRM fdinfo";
                    }
                }
            }
            readings.insert(device.card.clone(), result);
        }
        self.previous = current;
        readings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fdinfo_parses_engine_time_and_deduplicable_client_id() {
        let (client, counters) = parse_fdinfo(
            "drm-driver: i915\ndrm-client-id: 12\ndrm-engine-render: 500000000 ns\ndrm-engine-video: 90000000 ns\n",
        )
        .unwrap();
        assert_eq!(client, "12");
        assert_eq!(counters.nanos["render"], 500_000_000);
        assert!(parse_fdinfo("drm-client-id: 12\n").is_none());
    }
    #[test]
    fn fdinfo_samples_cpu_independently_and_clamps() {
        let mut before = HashMap::new();
        let mut after = HashMap::new();
        before.insert("1".into(), parse_fdinfo("drm-client-id: 1\ndrm-engine-render: 0 ns\n").unwrap().1);
        after.insert("1".into(), parse_fdinfo("drm-client-id: 1\ndrm-engine-render: 250000000 ns\n").unwrap().1);
        assert_eq!(fdinfo_utilization(&after, &before, Duration::from_secs(1)), Some(25.0));
        assert_eq!(fdinfo_utilization(&after, &before, Duration::ZERO), None);
    }
    #[test]
    fn xe_cycle_fraction_is_supported() {
        let old = parse_fdinfo("drm-client-id: 5\ndrm-cycles-rcs0: 10\ndrm-total-cycles-rcs0: 100\n").unwrap().1;
        let now = parse_fdinfo("drm-client-id: 5\ndrm-cycles-rcs0: 30\ndrm-total-cycles-rcs0: 140\n").unwrap().1;
        let old = HashMap::from([("5".into(), old)]);
        let now = HashMap::from([("5".into(), now)]);
        assert_eq!(fdinfo_utilization(&now, &old, Duration::from_secs(1)), Some(50.0));
    }
}
