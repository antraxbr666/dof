use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CGROUP_BASE: &str = "/sys/fs/cgroup/system.slice";

pub struct CgroupSnapshot {
    pub cpu_usage_usec: u64,
    pub memory_bytes: u64,
    pub memory_limit: u64,
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

fn read_snapshot_v1(container_id: &str) -> Option<CgroupSnapshot> {
    let path_sets = [
        (
            "/sys/fs/cgroup/cpuacct/docker/{id}",
            "/sys/fs/cgroup/memory/docker/{id}",
            "/sys/fs/cgroup/cpu/docker/{id}",
        ),
        (
            "/sys/fs/cgroup/cpuacct/system.slice/docker-{id}.scope",
            "/sys/fs/cgroup/memory/system.slice/docker-{id}.scope",
            "/sys/fs/cgroup/cpu/system.slice/docker-{id}.scope",
        ),
    ];

    for (cpuacct_tmpl, mem_tmpl, cpu_tmpl) in &path_sets {
        let cpuacct_dir = cpuacct_tmpl.replace("{id}", container_id);
        let mem_dir = mem_tmpl.replace("{id}", container_id);
        let cpu_dir = cpu_tmpl.replace("{id}", container_id);

        let usage_ns = fs::read_to_string(format!("{}/cpuacct.usage", cpuacct_dir)).ok()?;
        let cpu_usec = usage_ns.trim().parse::<u64>().ok()? / 1000;

        let mem_current = fs::read_to_string(format!("{}/memory.usage_in_bytes", mem_dir)).ok()?;
        let mem_limit_raw = fs::read_to_string(format!("{}/memory.limit_in_bytes", mem_dir)).ok()?;
        let mem_bytes = mem_current.trim().parse::<u64>().ok()?;
        let mem_limit = mem_limit_raw.trim().parse::<u64>().unwrap_or(u64::MAX);

        let quota_ratio = (|| -> Option<f64> {
            let q = fs::read_to_string(format!("{}/cpu.cfs_quota_us", cpu_dir)).ok()?;
            let p = fs::read_to_string(format!("{}/cpu.cfs_period_us", cpu_dir)).ok()?;
            let quota: i64 = q.trim().parse().ok()?;
            let period: u64 = p.trim().parse().ok()?;
            if quota <= 0 || period == 0 {
                return None;
            }
            Some(quota as f64 / period as f64)
        })();

        return Some(CgroupSnapshot {
            cpu_usage_usec: cpu_usec,
            memory_bytes: mem_bytes,
            memory_limit: mem_limit,
            cpu_quota_ratio: quota_ratio,
        });
    }
    None
}

pub fn read_snapshot(path: &PathBuf, container_id: &str) -> Option<CgroupSnapshot> {
    (|| {
        Some(CgroupSnapshot {
            cpu_usage_usec: read_cpu_usage_usec(path)?,
            memory_bytes: read_memory_current(path)?,
            memory_limit: read_memory_limit(path)?,
            cpu_quota_ratio: read_cpu_quota(path),
        })
    })()
    .or_else(|| read_snapshot_v1(container_id))
}

pub fn build_cgroup_map(ids: &[&str]) -> HashMap<String, PathBuf> {
    let all_dirs = find_cgroup_dirs();
    let mut result = HashMap::new();

    for id in ids {
        if let Some(path) = all_dirs.get(*id) {
            result.insert(id.to_string(), path.clone());
            continue;
        }

        let matches: Vec<&String> = all_dirs.keys().filter(|full_id| full_id.starts_with(*id)).collect();
        match matches.len() {
            0 => {}
            1 => {
                result.insert(id.to_string(), all_dirs[matches[0]].clone());
            }
            _ => {
                eprintln!(
                    "Warning: ambiguous container ID '{}' matches {} cgroup directories, using first match",
                    id,
                    matches.len()
                );
                if let Some(first) = matches.first() {
                    result.insert(id.to_string(), all_dirs[*first].clone());
                }
            }
        }
    }

    result
}
