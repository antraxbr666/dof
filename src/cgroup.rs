use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CGROUP_BASE: &str = "/sys/fs/cgroup/system.slice";

pub struct CgroupSnapshot {
    pub cpu_usage_usec: u64,
    pub memory_bytes: u64,
    pub memory_limit: u64,
}

fn find_cgroup_dirs() -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    let Ok(entries) = fs::read_dir(CGROUP_BASE) else {
        return map;
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        // docker-<64-char-id>.scope
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
            return val.trim().parse::<u64>().ok();
        }
    }
    None
}

fn read_memory_current(path: &PathBuf) -> Option<u64> {
    let content = fs::read_to_string(path.join("memory.current")).ok()?;
    content.trim().parse::<u64>().ok()
}

fn read_memory_limit(path: &PathBuf) -> Option<u64> {
    let content = fs::read_to_string(path.join("memory.max")).ok()?;
    let trimmed = content.trim();
    if trimmed == "max" {
        Some(u64::MAX)
    } else {
        trimmed.parse::<u64>().ok()
    }
}

pub fn read_snapshot(path: &PathBuf) -> Option<CgroupSnapshot> {
    Some(CgroupSnapshot {
        cpu_usage_usec: read_cpu_usage_usec(path)?,
        memory_bytes: read_memory_current(path)?,
        memory_limit: read_memory_limit(path)?,
    })
}

pub fn build_cgroup_map(ids: &[&str]) -> HashMap<String, PathBuf> {
    let all_dirs = find_cgroup_dirs();
    let mut result = HashMap::new();

    for id in ids {
        // Tenta match exato
        if let Some(path) = all_dirs.get(*id) {
            result.insert(id.to_string(), path.clone());
            continue;
        }
        // Tenta match pelo prefixo (ID curto do Docker API)
        for (full_id, path) in &all_dirs {
            if full_id.starts_with(*id) {
                result.insert(id.to_string(), path.clone());
                break;
            }
        }
    }

    result
}
