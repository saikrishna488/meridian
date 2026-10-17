//! Read-only Linux metrics. Called on a worker, never on GTK's main loop.
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use meridian_protocol::{ErrorBody, ErrorCode, ProcessSample, PulseSnapshot, ResourceSample};

#[derive(Default)]
pub struct Sampler {
    previous: Option<Previous>,
}
struct Previous {
    time: Instant,
    cpu: (u64, u64),
    processes: HashMap<u32, (u64, u64)>, // PID -> (start time, CPU ticks), guards PID reuse.
    io: HashMap<String, (u64, u64)>,
}
fn text(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap_or_default().trim().to_owned()
}
fn number(path: impl AsRef<Path>) -> Option<u64> {
    text(path).parse().ok()
}
fn values(line: &str) -> Vec<u64> {
    line.split_whitespace().filter_map(|s| s.parse().ok()).collect()
}
fn cpu_times(line: &str) -> Option<(u64, u64)> {
    let v = values(line);
    if v.len() < 4 {
        return None;
    }
    // guest and guest_nice are already included in user/nice.
    Some((v.iter().take(8).sum(), v[3] + v.get(4).copied().unwrap_or(0)))
}
fn usage(now: (u64, u64), old: (u64, u64)) -> Option<f64> {
    let total = now.0.checked_sub(old.0)?;
    let idle = now.1.checked_sub(old.1)?;
    (total > 0).then(|| (100.0 * total.saturating_sub(idle) as f64 / total as f64).clamp(0.0, 100.0))
}
fn rate(now: u64, old: u64, seconds: f64) -> Option<f64> {
    (seconds > 0.0).then(|| now.checked_sub(old).map(|n| n as f64 / seconds)).flatten()
}
fn resource(id: &str, kind: &str, name: String, detail: String) -> ResourceSample {
    ResourceSample {
        id: id.into(),
        kind: kind.into(),
        name,
        detail,
        percent: None,
        used: None,
        total: None,
        read_rate: None,
        write_rate: None,
    }
}
fn process_stat(pid: u32, stat: &str, page_size: u64) -> Option<(ProcessSample, u64, u64)> {
    let start = stat.find('(')?;
    let end = stat.rfind(')')?;
    let fields: Vec<_> = stat.get(end + 2..)?.split_whitespace().collect();
    // fields[0] is field 3 (state); names can contain spaces and parentheses.
    let ticks = fields.get(11)?.parse::<u64>().ok()? + fields.get(12)?.parse::<u64>().ok()?;
    let born = fields.get(19)?.parse().ok()?;
    let pages = fields.get(21)?.parse::<u64>().ok()?;
    Some((
        ProcessSample {
            pid,
            name: stat[start + 1..end].into(),
            state: fields[0].into(),
            cpu: None,
            memory: pages.saturating_mul(page_size) as f64,
        },
        born,
        ticks,
    ))
}

impl Sampler {
    pub fn sample(&mut self) -> Result<PulseSnapshot, ErrorBody> {
        let time = Instant::now();
        let stat = fs::read_to_string("/proc/stat")
            .map_err(|e| ErrorBody::new(ErrorCode::Failed, format!("Could not read system metrics: {e}")))?;
        let cpu = stat
            .lines()
            .next()
            .and_then(cpu_times)
            .ok_or_else(|| ErrorBody::new(ErrorCode::Failed, "CPU metrics unavailable"))?;
        // A resumed window needs a fresh baseline, not an average over its time closed.
        let old = self.previous.as_ref().filter(|p| time.duration_since(p.time).as_secs_f64() <= 5.0);
        let seconds = old.map(|p| time.duration_since(p.time).as_secs_f64()).unwrap_or(0.0);
        let delta = old.and_then(|p| cpu.0.checked_sub(p.cpu.0)).filter(|n| *n > 0);
        let mut resources = Vec::new();
        let model = text("/proc/cpuinfo")
            .lines()
            .find_map(|line| {
                line.strip_prefix("model name").and_then(|s| s.split_once(':')).map(|(_, s)| s.trim().to_owned())
            })
            .unwrap_or_else(|| std::env::consts::ARCH.into());
        let cores = stat
            .lines()
            .filter(|s| s.strip_prefix("cpu").is_some_and(|s| s.starts_with(|c: char| c.is_ascii_digit())))
            .count();
        let mut r = resource("cpu", "cpu", "CPU".into(), format!("{model} · {cores} logical processors"));
        r.percent = old.and_then(|p| usage(cpu, p.cpu));
        resources.push(r);

        let memory: HashMap<String, u64> = text("/proc/meminfo")
            .lines()
            .filter_map(|line| {
                let (name, value) = line.split_once(':')?;
                Some((name.into(), value.split_whitespace().next()?.parse::<u64>().ok()?.saturating_mul(1024)))
            })
            .collect();
        let mut r = resource("memory", "memory", "Memory".into(), "Physical RAM · used / total".into());
        if let (Some(total), Some(available)) = (memory.get("MemTotal"), memory.get("MemAvailable")) {
            r.total = Some(*total as f64);
            r.used = Some(total.saturating_sub(*available) as f64);
            r.percent = (*total > 0).then(|| r.used.unwrap() * 100.0 / *total as f64);
        }
        resources.push(r);

        let mut io = HashMap::new();
        if let Ok(entries) = fs::read_dir("/sys/block") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if ["loop", "ram", "zram"].iter().any(|prefix| name.starts_with(prefix)) {
                    continue;
                }
                let p = entry.path();
                let v = values(&text(p.join("stat")));
                let model = text(p.join("device/model"));
                let mut r = resource(
                    &format!("disk:{name}"),
                    "storage",
                    name.clone(),
                    if model.is_empty() { "Block device · read / write".into() } else { model },
                );
                r.total = number(p.join("size")).map(|n| n as f64 * 512.0);
                if v.len() >= 10 {
                    // Linux block counters use 512-byte sectors regardless of drive sector size.
                    let counters = (v[2].saturating_mul(512), v[6].saturating_mul(512));
                    if let Some(previous) = old.and_then(|p| p.io.get(&r.id)) {
                        r.read_rate = rate(counters.0, previous.0, seconds);
                        r.write_rate = rate(counters.1, previous.1, seconds);
                    }
                    io.insert(r.id.clone(), counters);
                }
                resources.push(r);
            }
        }
        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name == "lo" {
                    continue;
                }
                let p = entry.path();
                let mut r = resource(
                    &format!("net:{name}"),
                    "network",
                    name,
                    format!("{} · download / upload", text(p.join("operstate"))),
                );
                if let (Some(rx), Some(tx)) =
                    (number(p.join("statistics/rx_bytes")), number(p.join("statistics/tx_bytes")))
                {
                    if let Some(previous) = old.and_then(|p| p.io.get(&r.id)) {
                        r.read_rate = rate(rx, previous.0, seconds);
                        r.write_rate = rate(tx, previous.1, seconds);
                    }
                    io.insert(r.id.clone(), (rx, tx));
                }
                resources.push(r);
            }
        }
        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.strip_prefix("card").is_some_and(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())) {
                    continue;
                }
                let p = entry.path().join("device");
                let vendor = match text(p.join("vendor")).as_str() {
                    "0x1002" => "AMD",
                    "0x8086" => "Intel",
                    "0x10de" => "NVIDIA",
                    _ => "Graphics",
                };
                let mut r = resource(
                    &format!("gpu:{name}"),
                    "gpu",
                    format!("{vendor} GPU · {name}"),
                    format!("Device {}", text(p.join("device"))),
                );
                r.percent = number(p.join("gpu_busy_percent")).map(|n| n.min(100) as f64);
                r.used = number(p.join("mem_info_vram_used")).map(|n| n as f64);
                r.total = number(p.join("mem_info_vram_total")).map(|n| n as f64);
                if r.percent.is_none() {
                    r.detail = "Usage unavailable from this driver".into();
                }
                resources.push(r);
            }
        }
        if !resources.iter().any(|r| r.kind == "gpu") {
            resources.push(resource("gpu:none", "gpu", "GPU".into(), "No GPU metrics exposed by this system".into()));
        }
        resources.sort_by(|a, b| {
            let rank = |s: &str| match s {
                "cpu" => 0,
                "memory" => 1,
                "gpu" => 2,
                "storage" => 3,
                _ => 4,
            };
            rank(&a.kind).cmp(&rank(&b.kind)).then_with(|| a.id.cmp(&b.id))
        });
        // SAFETY: sysconf has no pointer arguments and does not mutate Rust data.
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        let page_size = if page_size > 0 { page_size as u64 } else { 4096 };
        let mut processes = Vec::new();
        let mut process_ticks = HashMap::new();
        let entries = fs::read_dir("/proc").map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))?;
        for entry in entries.flatten() {
            let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
                continue;
            };
            let Some((mut process, born, ticks)) = process_stat(pid, &text(entry.path().join("stat")), page_size)
            else {
                continue;
            };
            if let (Some(total), Some((start, previous))) = (delta, old.and_then(|p| p.processes.get(&pid)))
                && *start == born
            {
                process.cpu = ticks.checked_sub(*previous).map(|n| (100.0 * n as f64 / total as f64).clamp(0.0, 100.0));
            }
            process_ticks.insert(pid, (born, ticks));
            processes.push(process);
        }
        processes.sort_by_key(|p| p.pid);
        self.previous = Some(Previous { time, cpu, processes: process_ticks, io });
        Ok(PulseSnapshot { resources, processes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cpu_excludes_guest_and_handles_resets() {
        assert_eq!(cpu_times("cpu  10 2 3 80 5 0 0 0 7 1"), Some((100, 85)));
        assert_eq!(usage((200, 135), (100, 85)), Some(50.0));
        assert_eq!(usage((100, 85), (100, 85)), None);
        assert_eq!(usage((10, 5), (100, 85)), None);
        assert_eq!(rate(50, 100, 2.0), None);
        assert_eq!(rate(300, 100, 2.0), Some(100.0));
    }
    #[test]
    fn process_names_with_spaces_and_parentheses_keep_field_offsets() {
        let line = "123 (my (worker)) S 1 2 3 4 5 6 7 8 9 10 120 30 0 0 20 0 1 0 999 2000 7";
        let (p, born, ticks) = process_stat(123, line, 4096).unwrap();
        assert_eq!(p.name, "my (worker)");
        assert_eq!(born, 999);
        assert_eq!(ticks, 150);
        assert_eq!(p.memory, 28672.0);
        assert!(process_stat(1, "", 4096).is_none());
    }
    #[test]
    fn reads_real_linux_snapshot_without_requiring_a_gpu() {
        let mut sampler = Sampler::default();
        let snapshot = sampler.sample().unwrap();
        assert!(snapshot.resources.iter().any(|r| r.id == "cpu"));
        assert!(snapshot.processes.iter().any(|p| p.pid == std::process::id()));
        assert!(snapshot.resources.iter().find(|r| r.id == "cpu").unwrap().percent.is_none());
        let _ = sampler.sample().unwrap();
        sampler.previous.as_mut().unwrap().time = Instant::now() - std::time::Duration::from_secs(10);
        let resumed = sampler.sample().unwrap();
        assert!(resumed.resources.iter().find(|r| r.id == "cpu").unwrap().percent.is_none());
        assert!(resumed.processes.iter().all(|p| p.cpu.is_none()));
    }
}
