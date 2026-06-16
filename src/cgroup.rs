use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CGROUP_BASE: &str = "/sys/fs/cgroup/system.slice";

pub struct CgroupSnapshot {
    pub cpu_usage_usec: u64,
    pub memory_bytes: u64,
    pub memory_limit: u64,
    // ponytail: only for --cpus quota normalization; file is in-memory sysfs
    pub cpu_quota_ratio: Option<f64>,
}

fn find_cgroup_dirs() -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    let Ok(entries) = fs::read_dir(CGROUP_BASE) else {
        return map;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(rest) = name.strip_prefix("docker-") {
            if let Some(id) = rest.strip_suffix(".scope") {
                map.insert(id.to_string(), entry.path());
            }
        }
    }
    map
}

fn read_cpu_usage_usec(path: &PathBuf) -> Option<u64> {
    let content = fs::read_to_string(path.join("cpu.stat")).ok()?;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("usage_usec ") {
            return val.trim().parse().ok();
        }
    }
    None
}

fn read_memory_current(path: &PathBuf) -> Option<u64> {
    let content = fs::read_to_string(path.join("memory.current")).ok()?;
    content.trim().parse().ok()
}

fn read_memory_limit(path: &PathBuf) -> Option<u64> {
    let content = fs::read_to_string(path.join("memory.max")).ok()?;
    let trimmed = content.trim();
    if trimmed == "max" {
        Some(u64::MAX)
    } else {
        trimmed.parse().ok()
    }
}

// ponytail: reads cpu.max for --cpus normalization; returns None when no limit ("max")
fn read_cpu_quota(path: &PathBuf) -> Option<f64> {
    let content = fs::read_to_string(path.join("cpu.max")).ok()?;
    let mut parts = content.split_whitespace();
    let quota_str = parts.next()?;
    let period_str = parts.next()?;
    if quota_str == "max" {
        return None;
    }
    let quota: f64 = quota_str.parse().ok()?;
    let period: f64 = period_str.parse().ok()?;
    if period <= 0.0 {
        return None;
    }
    Some(quota / period)
}

pub fn read_snapshot(path: &PathBuf) -> Option<CgroupSnapshot> {
    Some(CgroupSnapshot {
        cpu_usage_usec: read_cpu_usage_usec(path)?,
        memory_bytes: read_memory_current(path)?,
        memory_limit: read_memory_limit(path)?,
        cpu_quota_ratio: read_cpu_quota(path),
    })
}

// ponytail: prefix match is safe; Docker API guarantees short IDs are unique
pub fn build_cgroup_map(ids: &[&str]) -> HashMap<String, PathBuf> {
    let all_dirs = find_cgroup_dirs();
    let mut result = HashMap::new();
    for id in ids {
        if let Some(path) = all_dirs.get(*id) {
            result.insert(id.to_string(), path.clone());
            continue;
        }
        for (full_id, path) in &all_dirs {
            if full_id.starts_with(*id) {
                result.insert(id.to_string(), path.clone());
                break;
            }
        }
    }
    result
}
