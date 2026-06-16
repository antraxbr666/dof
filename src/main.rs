mod cgroup;

use bollard::query_parameters::{ListContainersOptions, StatsOptions};
use bollard::Docker;
use futures_util::future::join_all;
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use std::env;
use tabled::builder::Builder;
use tabled::settings::object::Columns;
use tabled::settings::{Alignment, Modify, Padding, Style};
use tokio::time::{sleep, Duration, Instant};

// ponytail: inline ANSI helpers replace make_colors crate
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

fn cpu_gauge(percent: f64, width: usize) -> String {
    let clamped = percent.min(100.0);
    let filled = ((clamped / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width.saturating_sub(filled);

    let filled_str = if filled > 0 {
        if percent > 100.0 {
            c(&"\u{2588}".repeat(filled), 243, 139, 168)
        } else {
            c(&"\u{2588}".repeat(filled), 166, 227, 161)
        }
    } else {
        String::new()
    };
    let empty_str = if empty > 0 {
        c(&"\u{2591}".repeat(empty), 49, 50, 68)
    } else {
        String::new()
    };

    if percent > 100.0 {
        format!("{}{} >100%", filled_str, empty_str)
    } else {
        format!("{}{} {:.1}%", filled_str, empty_str, percent)
    }
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
            (cpu_gauge(*cpu_pct, 8), mem.clone())
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

fn get_cpu_total_usage(stats: &bollard::models::ContainerStatsResponse) -> u64 {
    stats.cpu_stats.as_ref()
        .and_then(|s| s.cpu_usage.as_ref())
        .and_then(|u| u.total_usage)
        .unwrap_or(0)
}

fn get_memory_usage(stats: &bollard::models::ContainerStatsResponse) -> (u64, u64) {
    let usage = stats.memory_stats.as_ref()
        .and_then(|m| m.usage)
        .unwrap_or(0);
    let limit = stats.memory_stats.as_ref()
        .and_then(|m| m.limit)
        .unwrap_or(u64::MAX);
    (usage, limit)
}

async fn fetch_docker_stats_batch(
    docker: &Docker,
    ids: &[String],
) -> HashMap<String, bollard::models::ContainerStatsResponse> {
    let futures: Vec<_> = ids.iter().map(|id| {
        let id = id.clone();
        async move {
            let options = StatsOptions {
                stream: false,
                one_shot: false,
            };
            let mut stream = docker.stats(&id, Some(options));
            stream.next()
                .await
                .and_then(|r| r.ok())
                .map(|stats| (id, stats))
        }
    }).collect();

    join_all(futures)
        .await
        .into_iter()
        .flatten()
        .collect()
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
        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get() as f64)
            .unwrap_or(1.0);

        let ids_ref: Vec<&str> = running_ids.iter().map(|s| s.as_str()).collect();
        let cgroup_map = cgroup::build_cgroup_map(&ids_ref);

        let cgroup_key_set: HashSet<&str> = cgroup_map.keys().map(|s| s.as_str()).collect();
        let api_ids: Vec<String> = running_ids.iter()
            .filter(|id| !cgroup_key_set.contains(id.as_str()))
            .cloned()
            .collect();

        if !api_ids.is_empty() {
            eprintln!("Warning: {} container(s) without cgroup path, using Docker API fallback", api_ids.len());
            for id in &api_ids {
                eprintln!("  - {} (Docker API stats)", &id[..12]);
            }
        }

        // ── t0 sampling ──
        let t0_instant = Instant::now();

        let mut t0: HashMap<String, cgroup::CgroupSnapshot> = HashMap::new();
        for (id, path) in &cgroup_map {
            if let Some(snap) = cgroup::read_snapshot(path, id) {
                t0.insert(id.clone(), snap);
            } else {
                let api_ids_set: HashSet<&str> = api_ids.iter().map(|s| s.as_str()).collect();
                if !api_ids_set.contains(id.as_str()) {
                    eprintln!("Warning: cgroup read failed for container {} at t0, marking for Docker API fallback", &id[..12]);
                }
            }
        }

        let t0_api = if !api_ids.is_empty() {
            fetch_docker_stats_batch(&docker, &api_ids).await
        } else {
            HashMap::new()
        };

        let elapsed = t0_instant.elapsed();
        let remaining = Duration::from_millis(100).saturating_sub(elapsed);
        sleep(remaining).await;
        let t1_instant = Instant::now();
        let actual_interval_us = t1_instant.duration_since(t0_instant).as_micros().max(1);

        // ── t1 sampling ──
        let mut t1: HashMap<String, cgroup::CgroupSnapshot> = HashMap::new();
        for (id, path) in &cgroup_map {
            if let Some(snap) = cgroup::read_snapshot(path, id) {
                t1.insert(id.clone(), snap);
            }
        }

        let t1_api = if !api_ids.is_empty() {
            fetch_docker_stats_batch(&docker, &api_ids).await
        } else {
            HashMap::new()
        };

        // ── compute ──
        let mut result: HashMap<String, (f64, String)> = HashMap::new();

        for id in cgroup_map.keys() {
            if let (Some(snap0), Some(snap1)) = (t0.get(id), t1.get(id)) {
                let cpu_delta = snap1.cpu_usage_usec.saturating_sub(snap0.cpu_usage_usec);
                let raw_pct = (cpu_delta as f64 / actual_interval_us as f64) * 100.0;

                let cpu_pct = match snap1.cpu_quota_ratio {
                    Some(ratio) if ratio > 0.0 => (raw_pct / ratio).min(100.0),
                    _ => (raw_pct / num_cpus).min(100.0),
                };

                let limit = if snap1.memory_limit == u64::MAX {
                    sys_mem
                } else {
                    snap1.memory_limit
                };
                let mem_str = format!("{}/{}", human_bytes(snap1.memory_bytes), human_bytes(limit));

                result.insert(id.clone(), (cpu_pct, mem_str));
            }
        }

        for id in &api_ids {
            if let (Some(s0), Some(s1)) = (t0_api.get(id), t1_api.get(id)) {
                let cpu_delta_ns = get_cpu_total_usage(s1)
                    .saturating_sub(get_cpu_total_usage(s0));
                let actual_interval_ns = (actual_interval_us as f64) * 1000.0;
                let raw_pct = (cpu_delta_ns as f64 / actual_interval_ns) * 100.0;
                let cpu_pct = (raw_pct / num_cpus).min(100.0);

                let (mem_bytes, mem_limit_raw) = get_memory_usage(s1);
                let limit = if mem_limit_raw == u64::MAX { sys_mem } else { mem_limit_raw };
                let mem_str = format!("{}/{}", human_bytes(mem_bytes), human_bytes(limit));

                result.insert(id.clone(), (cpu_pct, mem_str));
            } else {
                eprintln!("Warning: Docker API stats failed for container {}", &id[..12]);
            }
        }

        result
    };

    print_version();
    println!();

    let table = build_table(containers, stats_map, &args);
    println!("{}", table);
}
