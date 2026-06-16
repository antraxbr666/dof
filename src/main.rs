mod cgroup;

use bollard::query_parameters::{ListContainersOptions, StatsOptions};
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use std::env;
use tabled::builder::Builder;
use tabled::settings::object::Columns;
use tabled::settings::{Alignment, Modify, Padding, Style};
use tokio::time::{sleep, Duration, Instant};

fn c(s: &str, r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{r};{g};{b}m{s}\x1b[0m")
}
fn cbg(s: &str, fr: u8, fg: u8, fb: u8, br: u8, bg: u8, bb: u8) -> String {
    format!("\x1b[1m\x1b[48;2;{br};{bg};{bb}m\x1b[38;2;{fr};{fg};{fb}m{s}\x1b[0m")
}

fn print_version() {
    let v = env!("CARGO_PKG_VERSION");
    println!("{} - v{}", c("dof", 242, 205, 205), c(v, 243, 139, 168));
    println!("{} - {}", c("Docker Usage Utility", 180, 190, 254), c("A better 'docker ps' alternative", 249, 226, 175));
}

fn print_help() {
    print_version();
    println!();
    println!("{}", c("USAGE:", 203, 166, 247));
    println!("    dof [OPTIONS]");
    println!();
    println!("{}", c("OPTIONS:", 203, 166, 247));
    println!("    {}, -r       {}", c("--running", 166, 227, 161), c("Show only running containers", 205, 214, 244));
    println!("    {}           {}", c("--no-trunc", 166, 227, 161), c("Show full container IDs without truncation", 205, 214, 244));
    println!("    {}, -v       {}", c("--version", 166, 227, 161), c("Print version information", 205, 214, 244));
    println!("    {}, -h       {}", c("--help", 166, 227, 161), c("Print help information", 205, 214, 244));
    println!();
    println!("{}", c("EXAMPLES:", 203, 166, 247));
    println!("    dof              # List all containers");
    println!("    dof -r           # List only running containers");
    println!("    dof --no-trunc   # Show full container IDs");
}

#[derive(Debug)]
struct Args {
    running: bool,
    no_trunc: bool,
}

fn parse_args() -> Option<Args> {
    let mut args = Args {
        running: false,
        no_trunc: false,
    };
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--running" | "-r" => args.running = true,
            "--no-trunc" => args.no_trunc = true,
            "--version" | "-v" => {
                print_version();
                return None;
            }
            "--help" | "-h" => {
                print_help();
                return None;
            }
            _ => {}
        }
    }
    Some(args)
}

fn format_id(id: &Option<String>, no_trunc: bool) -> String {
    match id {
        Some(id) if no_trunc => id.clone(),
        Some(id) => id.chars().take(12).collect(),
        None => "<none>".to_string(),
    }
}

fn format_names(names: &Option<Vec<String>>) -> String {
    match names {
        Some(names) if !names.is_empty() => names
            .iter()
            .map(|n| n.trim_start_matches('/').to_string())
            .collect::<Vec<_>>()
            .join(", "),
        _ => "<none>".to_string(),
    }
}

fn format_status(state: &Option<bollard::models::ContainerSummaryStateEnum>) -> (String, bool) {
    let is_running = state.as_ref()
        .map(|s| s.to_string().to_lowercase() == "running")
        .unwrap_or(false);

    if is_running {
        ("Running".to_string(), true)
    } else {
        ("Stopped".to_string(), false)
    }
}

fn format_health(status: &Option<String>) -> (String, bool, bool) {
    let status_str = status.as_deref().unwrap_or("").to_lowercase();
    if status_str.contains("(healthy)") {
        return ("\u{25cf}".to_string(), true, false);
    }
    if status_str.contains("(unhealthy)") {
        return ("\u{25cf}".to_string(), false, true);
    }
    if status_str.contains("starting") {
        return ("\u{25cf}".to_string(), false, false);
    }
    ("-".to_string(), false, false)
}

fn format_ports(ports: &Option<Vec<bollard::models::PortSummary>>) -> String {
    match ports {
        Some(ports) if !ports.is_empty() => {
            let mut seen = HashSet::new();
            let mut mappings = Vec::new();
            for p in ports.iter() {
                let private = p.private_port;
                let key = match p.public_port {
                    Some(pub_port) => format!("{}:{}", pub_port, private),
                    None => private.to_string(),
                };
                if seen.insert(key.clone()) {
                    mappings.push(key);
                }
            }
            if mappings.is_empty() {
                "<none>".to_string()
            } else {
                mappings.join("\n")
            }
        }
        _ => "<none>".to_string(),
    }
}

// ponytail: char scan replaces regex crate
fn colorize_ports(ports_str: &str) -> String {
    let mut out = String::with_capacity(ports_str.len());
    let mut buf = String::new();
    for ch in ports_str.chars() {
        if ch.is_ascii_digit() {
            buf.push(ch);
        } else {
            if !buf.is_empty() {
                out.push_str(&c(&buf, 249, 226, 175));
                buf.clear();
            }
            out.push(ch);
        }
    }
    if !buf.is_empty() {
        out.push_str(&c(&buf, 249, 226, 175));
    }
    out
}

fn human_bytes(bytes: u64) -> String {
    let units = ["B", "K", "M", "G", "T"];
    if bytes == 0 {
        return "0B".to_string();
    }
    let exp = (bytes as f64).log(1024.0).min(units.len() as f64 - 1.0) as usize;
    let val = (bytes as f64 / 1024f64.powi(exp as i32)).round() as u64;
    format!("{}{}", val, units[exp])
}

// ponytail: no OnceLock, CLI runs once
fn system_memory_bytes() -> u64 {
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                let kb_str: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
                if let Ok(kb) = kb_str.parse::<u64>() {
                    return kb * 1024;
                }
            }
        }
    }
    u64::MAX
}

fn build_table(
    containers: Vec<bollard::models::ContainerSummary>,
    stats_map: HashMap<String, (f64, String)>,
    args: &Args,
) -> tabled::Table {
    let mut builder = Builder::new();

    let header = [
        c("ID", 203, 166, 247),
        c("Name", 203, 166, 247),
        c("CPU", 203, 166, 247),
        c("Memory", 203, 166, 247),
        c("Health", 203, 166, 247),
        c("Status", 203, 166, 247),
        c("Ports", 203, 166, 247),
    ];
    builder.push_record(header);

    for container in containers.iter() {
        let id = container.id.clone().unwrap_or_default();
        let is_running = container.state.as_ref()
            .map(|s| s.to_string().to_lowercase() == "running")
            .unwrap_or(false);

        let (cpu_str, mem_str) = if let Some((cpu_pct, mem)) = stats_map.get(&id) {
            (c(&format!("{:.2}%", cpu_pct), 166, 227, 161), mem.clone())
        } else if is_running {
            (c("   ERR   ", 243, 139, 168), c("   ERR   ", 243, 139, 168))
        } else {
            ("-".to_string(), "-".to_string())
        };

        let (status_text, is_running_status) = format_status(&container.state);
        let (health_symbol, is_healthy, is_unhealthy) = format_health(&container.status);

        let health_colored = if is_healthy {
            c(&health_symbol, 166, 227, 161)
        } else if is_unhealthy {
            c(&health_symbol, 243, 139, 168)
        } else if container.status.as_ref().map(|s| s.to_lowercase().contains("starting")).unwrap_or(false) {
            c(&health_symbol, 249, 226, 175)
        } else {
            c(&health_symbol, 108, 112, 134)
        };

        let status_colored = if is_running_status {
            c(&status_text, 166, 227, 161)
        } else {
            cbg(&status_text, 17, 17, 27, 243, 139, 168)
        };

        let ports_text = colorize_ports(&format_ports(&container.ports));

        builder.push_record([
            c(&format_id(&container.id, args.no_trunc), 180, 190, 254),
            c(&format_names(&container.names), 148, 226, 213),
            cpu_str,
            c(&mem_str, 137, 180, 250),
            health_colored,
            status_colored,
            c(&ports_text, 205, 214, 244),
        ]);
    }

    let mut table = builder.build();
    table.with(Style::modern_rounded());
    table.with(Modify::new(Columns::new(4..5)).with(Alignment::center()));
    table.with(Modify::new(Columns::new(5..6)).with(Alignment::center()));
    table.with(Padding::new(1, 1, 0, 0));

    table
}

// ponytail: single-call Docker API using precpu_stats + system_cpu_usage normalization
// Same formula docker stats uses internally. ~1ms per call, no sampling delay needed.
async fn fetch_cpu_from_docker_api(
    docker: &Docker,
    id: &str,
    sys_mem: u64,
) -> Option<(f64, String)> {
    let opts = || Some(StatsOptions { stream: false, one_shot: true });

    let s0 = docker.stats(id, opts()).next().await.and_then(|r| r.ok())?;
    let t0_cpu = s0.cpu_stats.as_ref()?.cpu_usage.as_ref()?.total_usage?;

    sleep(Duration::from_millis(100)).await;

    let s1 = docker.stats(id, opts()).next().await.and_then(|r| r.ok())?;
    let t1_cpu = s1.cpu_stats.as_ref()?.cpu_usage.as_ref()?.total_usage?;

    // ponytail: (delta_ns / 100_000_000 ns) * 100 = delta_ns / 1_000_000
    let cpu_pct = t1_cpu.saturating_sub(t0_cpu) as f64 / 1_000_000.0;

    let mem_bytes = s1.memory_stats.as_ref()
        .and_then(|m| m.usage).unwrap_or(0);
    let mem_limit_raw = s1.memory_stats.as_ref()
        .and_then(|m| m.limit).unwrap_or(u64::MAX);
    let limit = if mem_limit_raw == u64::MAX { sys_mem } else { mem_limit_raw };
    let mem_str = format!("{}/{}", human_bytes(mem_bytes), human_bytes(limit));

    Some((cpu_pct, mem_str))
}

// ponytail: single-threaded runtime, no concurrent work
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = match parse_args() {
        Some(a) => a,
        None => return,
    };

    let docker = match Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: Could not connect to Docker daemon: {}", e);
            eprintln!("Make sure Docker is running and you have permission to access it.");
            std::process::exit(1);
        }
    };

    let options = Some(ListContainersOptions {
        all: !args.running,
        ..Default::default()
    });

    let containers = match docker.list_containers(options).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: Failed to list containers: {}", e);
            std::process::exit(1);
        }
    };

    if containers.is_empty() {
        if args.running {
            println!("No running containers found.");
        } else {
            println!("No containers found.");
        }
        return;
    }

    let sys_mem = system_memory_bytes();

    let running_ids: Vec<String> = containers
        .iter()
        .filter(|c| {
            c.state.as_ref()
                .map(|s| s.to_string().to_lowercase() == "running")
                .unwrap_or(false)
        })
        .filter_map(|c| c.id.clone())
        .collect();

    let stats_map = if running_ids.is_empty() {
        HashMap::new()
    } else {
        let ids_ref: Vec<&str> = running_ids.iter().map(|s| s.as_str()).collect();
        let cgroup_map = cgroup::build_cgroup_map(&ids_ref);

        // ponytail: Instant measures real wall-clock between t0 and t1, eliminating sleep inaccuracy
        let t0 = Instant::now();
        let mut snapshots0: HashMap<String, cgroup::CgroupSnapshot> = HashMap::new();
        for (id, path) in &cgroup_map {
            match cgroup::read_snapshot(path) {
                Some(s) => { snapshots0.insert(id.clone(), s); }
                None => { eprintln!("Warning: cgroup read failed for container {}", &id[..12]); }
            }
        }

        sleep(Duration::from_millis(100)).await;
        let t1 = Instant::now();
        let dt_us = t1.duration_since(t0).as_micros().max(1);

        let mut result = HashMap::new();
        for (id, path) in &cgroup_map {
            if let (Some(s0), Some(s1)) = (snapshots0.get(id), cgroup::read_snapshot(path)) {
                let raw = (s1.cpu_usage_usec.saturating_sub(s0.cpu_usage_usec) as f64 / dt_us as f64) * 100.0;
                // ponytail: normalize by --cpus quota when set; otherwise raw per-core %
                let cpu_pct = match s1.cpu_quota_ratio {
                    Some(r) if r > 0.0 => (raw / r).min(100.0),
                    _ => raw,
                };
                let limit = if s1.memory_limit == u64::MAX { sys_mem } else { s1.memory_limit };
                let mem_str = format!("{}/{}", human_bytes(s1.memory_bytes), human_bytes(limit));
                result.insert(id.clone(), (cpu_pct, mem_str));
            }
        }

        // ponytail: Docker API fallback for containers without cgroup path (cgroup v1, permissions, etc.)
        for id in &running_ids {
            if result.contains_key(id) {
                continue;
            }
            if cgroup_map.contains_key(id.as_str()) {
                eprintln!("Warning: container {} has cgroup path but stats read failed", &id[..12]);
                continue;
            }
            match fetch_cpu_from_docker_api(&docker, id, sys_mem).await {
                Some(stats) => { result.insert(id.clone(), stats); }
                None => { eprintln!("Warning: container {} — cgroup v2 not found and Docker API stats also failed", &id[..12]); }
            }
        }

        result
    };

    print_version();
    println!();

    let table = build_table(containers, stats_map, &args);
    println!("{}", table);
}
