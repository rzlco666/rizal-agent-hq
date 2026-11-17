use serde::{Deserialize, Serialize};
use std::path::Path;
use sysinfo::{Disks, System};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_percent: f64,
    pub mem_percent: f64,
    pub disk_percent: f64,
    pub uptime_secs: u64,
}

pub fn parse_meminfo(input: &str) -> Option<f64> {
    let values: std::collections::HashMap<&str, u64> = input
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            let kib = value.split_whitespace().next()?.parse().ok()?;
            Some((key, kib))
        })
        .collect();
    let total = *values.get("MemTotal")?;
    let available = *values.get("MemAvailable")?;
    if total == 0 || available > total {
        return None;
    }
    Some((total - available) as f64 * 100.0 / total as f64)
}

pub fn probe_system(path: &Path) -> Option<SystemMetrics> {
    let mut system = System::new_all();
    system.refresh_cpu_all();
    let total_memory = system.total_memory();
    if total_memory == 0 {
        return None;
    }
    let mem_percent = system.used_memory() as f64 * 100.0 / total_memory as f64;
    let disks = Disks::new_with_refreshed_list();
    let disk = disks
        .list()
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())?;
    let total_space = disk.total_space();
    if total_space == 0 {
        return None;
    }
    Some(SystemMetrics {
        cpu_percent: system.global_cpu_usage() as f64,
        mem_percent,
        disk_percent: (total_space - disk.available_space()) as f64 * 100.0 / total_space as f64,
        uptime_secs: System::uptime(),
    })
}
