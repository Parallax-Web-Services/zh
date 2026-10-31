//! Read-only local telemetry for the first Positronic resource map.
//! Linux/WSL2 only for now. No remote execution or user data collection.

use serde::Serialize;
use std::{fs, process::Command, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub collected_at_ms: u128,
    pub node: Node,
    pub cpu: Cpu,
    pub memory: Memory,
    pub storage: Option<Storage>,
    pub network: Vec<NetworkInterface>,
    pub gpu: Gpu,
    pub fabric: Fabric,
}

#[derive(Debug, Serialize)]
pub struct Node {
    pub name: String,
    pub operating_system: String,
    pub is_wsl: bool,
    pub uptime_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct Cpu {
    pub logical_cores: usize,
    pub usage_percent: Option<f64>,
    pub load_one_minute: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct Memory {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct Storage {
    pub mount: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct NetworkInterface {
    pub name: String,
    pub link_state: String,
}

#[derive(Debug, Serialize)]
pub struct Gpu {
    pub status: &'static str,
    pub note: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Fabric {
    pub mode: &'static str,
    pub connected_peers: Vec<String>,
    pub orchestration_status: &'static str,
}

pub struct Monitor {
    previous_cpu: Mutex<Option<(u64, u64)>>,
}

impl Monitor {
    pub fn new() -> Self {
        Self { previous_cpu: Mutex::new(None) }
    }

    pub fn snapshot(&self) -> Snapshot {
        let cpu_now = cpu_times();
        let usage_percent = match self.previous_cpu.lock() {
            Ok(mut guard) => {
                let percentage = match (*guard, cpu_now) {
                    (Some((previous_total, previous_idle)), Some((total, idle))) => {
                        let diff_total = total.saturating_sub(previous_total);
                        let diff_idle = idle.saturating_sub(previous_idle);
                        (diff_total > 0).then(|| {
                            (100.0 * (diff_total.saturating_sub(diff_idle) as f64)
                                / diff_total as f64).clamp(0.0, 100.0)
                        })
                    }
                    _ => None,
                };
                *guard = cpu_now;
                percentage
            }
            Err(_) => None,
        };

        let (total_memory, available_memory) = memory_bytes();
        Snapshot {
            schema_version: 1,
            collected_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            node: Node {
                name: hostname(),
                operating_system: std::env::consts::OS.to_owned(),
                is_wsl: is_wsl(),
                uptime_seconds: uptime_seconds(),
            },
            cpu: Cpu {
                logical_cores: std::thread::available_parallelism()
                    .map(|x| x.get()).unwrap_or(1),
                usage_percent,
                load_one_minute: load_one_minute(),
            },
            memory: Memory {
                total_bytes: total_memory,
                available_bytes: available_memory,
                used_bytes: total_memory.saturating_sub(available_memory),
            },
            storage: root_storage(),
            network: network_interfaces(),
            gpu: Gpu {
                status: "unavailable",
                note: "GPU discovery is not implemented in this alpha; this is not a claim that no GPU exists.",
            },
            fabric: Fabric {
                mode: "local_only",
                connected_peers: vec![],
                orchestration_status: "not_implemented",
            },
        }
    }
}

fn cpu_times() -> Option<(u64, u64)> {
    let text = fs::read_to_string("/proc/stat").ok()?;
    let first = text.lines().find(|line| line.starts_with("cpu "))?;
    let numbers: Vec<u64> = first.split_whitespace()
        .skip(1).filter_map(|part| part.parse().ok()).collect();
    if numbers.len() < 4 { return None; }
    // idle and iowait are both idle for the purpose of this overview.
    let idle = numbers[3].saturating_add(*numbers.get(4).unwrap_or(&0));
    Some((numbers.iter().sum(), idle))
}

fn memory_bytes() -> (u64, u64) {
    let data = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0;
    let mut available = 0;
    for line in data.lines() {
        let mut fields = line.split_whitespace();
        match fields.next() {
            Some("MemTotal:") => total = fields.next().and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0) * 1024,
            Some("MemAvailable:") => available = fields.next().and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0) * 1024,
            _ => (),
        }
    }
    (total, available.min(total))
}

fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .map(|v| v.trim().to_owned())
        .ok().filter(|v| !v.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "local-machine".to_owned())
}

fn is_wsl() -> bool {
    fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

fn uptime_seconds() -> Option<u64> {
    fs::read_to_string("/proc/uptime").ok()?
        .split_whitespace().next()?.parse::<f64>().ok()
        .map(|v| v as u64)
}

fn load_one_minute() -> Option<f64> {
    fs::read_to_string("/proc/loadavg").ok()?
        .split_whitespace().next()?.parse().ok()
}

fn root_storage() -> Option<Storage> {
    let output = Command::new("df").args(["-Pk", "/"]).output().ok()?;
    if !output.status.success() { return None; }
    let text = String::from_utf8_lossy(&output.stdout);
    let data = text.lines().nth(1)?;
    let fields: Vec<&str> = data.split_whitespace().collect();
    if fields.len() < 6 { return None; }
    let total_bytes = fields[1].parse::<u64>().ok()?.saturating_mul(1024);
    let available_bytes = fields[3].parse::<u64>().ok()?.saturating_mul(1024);
    Some(Storage {
        mount: "/".into(),
        total_bytes,
        available_bytes,
        used_bytes: total_bytes.saturating_sub(available_bytes),
    })
}

fn network_interfaces() -> Vec<NetworkInterface> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" { continue; }
            let state = fs::read_to_string(entry.path().join("operstate"))
                .map(|s| s.trim().to_string()).unwrap_or_else(|_| "unknown".into());
            result.push(NetworkInterface { name, link_state: state });
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_is_valid_json() {
        let snapshot = Monitor::new().snapshot();
        let data = serde_json::to_value(snapshot).unwrap();
        assert_eq!(data["schema_version"], 1);
        assert_eq!(data["fabric"]["mode"], "local_only");
        assert_eq!(data["fabric"]["connected_peers"].as_array().unwrap().len(), 0);
    }
}
