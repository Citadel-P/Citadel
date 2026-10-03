use std::fs;
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsSnapshot {
    pub cgroup_memory_current_bytes: Option<u64>,
    pub cgroup_memory_peak_bytes: Option<u64>,
    pub cgroup_memory_max_bytes: Option<u64>,
    pub cgroup_pids_current: Option<u64>,
    pub process_rss_kib: Option<u64>,
    pub process_threads: Option<u64>,
    pub process_file_descriptors: Option<usize>,
}

#[must_use]
pub fn snapshot() -> DiagnosticsSnapshot {
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    DiagnosticsSnapshot {
        cgroup_memory_current_bytes: read_u64("/sys/fs/cgroup/memory.current"),
        cgroup_memory_peak_bytes: read_u64("/sys/fs/cgroup/memory.peak"),
        cgroup_memory_max_bytes: read_u64("/sys/fs/cgroup/memory.max"),
        cgroup_pids_current: read_u64("/sys/fs/cgroup/pids.current"),
        process_rss_kib: status_value(&status, "VmRSS:"),
        process_threads: status_value(&status, "Threads:"),
        process_file_descriptors: fs::read_dir("/proc/self/fd")
            .ok()
            .map(|entries| entries.filter_map(Result::ok).count()),
    }
}

fn read_u64(path: impl AsRef<Path>) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn status_value(status: &str, name: &str) -> Option<u64> {
    status
        .lines()
        .find(|line| line.starts_with(name))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_status_values() {
        let status = "Name:\tcitadel\nVmRSS:\t  1234 kB\nThreads:\t7\n";
        assert_eq!(status_value(status, "VmRSS:"), Some(1234));
        assert_eq!(status_value(status, "Threads:"), Some(7));
    }
}
