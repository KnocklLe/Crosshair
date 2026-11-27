//! Real resource usage of the main process (Settings → Performance).
//!
//! The WebView2 runtime renders the interface in separate `msedgewebview2`
//! processes; they are not included here and the UI says so.

use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProcessUsage {
    pub working_set_bytes: u64,
    pub private_bytes: u64,
    /// Average CPU since the previous sample, all cores = 100 %.
    pub cpu_percent: f32,
    pub uptime_seconds: u64,
    pub logical_cpus: u32,
}

pub struct Sampler {
    last: Mutex<Option<(Instant, u64)>>,
}

impl Default for Sampler {
    fn default() -> Self {
        Self { last: Mutex::new(None) }
    }
}

#[cfg(windows)]
fn read_process() -> (u64, u64, u64) {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    unsafe {
        let process = GetCurrentProcess();
        let mut mem = PROCESS_MEMORY_COUNTERS_EX {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
            ..Default::default()
        };
        let (ws, private) = if GetProcessMemoryInfo(process, &mut mem as *mut _ as *mut PROCESS_MEMORY_COUNTERS, mem.cb).is_ok() {
            (mem.WorkingSetSize as u64, mem.PrivateUsage as u64)
        } else {
            (0, 0)
        };
        let (mut c, mut e, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        let cpu = if GetProcessTimes(process, &mut c, &mut e, &mut k, &mut u).is_ok() {
            let to_u64 = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
            to_u64(k) + to_u64(u) // 100 ns units
        } else {
            0
        };
        (ws, private, cpu)
    }
}

#[cfg(not(windows))]
fn read_process() -> (u64, u64, u64) {
    (0, 0, 0)
}

impl Sampler {
    pub fn sample(&self, started: Instant) -> ProcessUsage {
        let (ws, private, cpu_100ns) = read_process();
        let now = Instant::now();
        let cpus = std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1);
        let mut last = self.last.lock().unwrap_or_else(|p| p.into_inner());
        let cpu_percent = match *last {
            Some((t, prev)) => {
                let wall = now.duration_since(t).as_secs_f64();
                let used = cpu_100ns.saturating_sub(prev) as f64 / 10_000_000.0;
                if wall > 0.0 {
                    ((used / wall) / cpus as f64 * 100.0) as f32
                } else {
                    0.0
                }
            }
            None => 0.0,
        };
        *last = Some((now, cpu_100ns));
        ProcessUsage {
            working_set_bytes: ws,
            private_bytes: private,
            cpu_percent: cpu_percent.clamp(0.0, 100.0),
            uptime_seconds: now.duration_since(started).as_secs(),
            logical_cpus: cpus,
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn memory_is_reported() {
        let s = Sampler::default();
        let first = s.sample(Instant::now());
        assert!(first.working_set_bytes > 0);
        assert_eq!(first.cpu_percent, 0.0, "first sample has no reference");
        let second = s.sample(Instant::now());
        assert!(second.cpu_percent >= 0.0 && second.cpu_percent <= 100.0);
    }
}
