mod cgroup;

use bollard::query_parameters::ListContainersOptions;
use bollard::Docker;
use make_colors::{make_colors_rgb, make_colors_hex_with_attrs};
use regex::Regex;
use std::collections::HashSet;
use std::env;
use std::sync::OnceLock;
use tabled::builder::Builder;
use tabled::settings::{Alignment, Modify, Padding, Style};
use tabled::settings::object::Columns;
use tokio::time::{sleep, Duration};

fn print_version() {
    let v = env!("CARGO_PKG_VERSION");
    println!("{} - v{}", make_colors_rgb("dof", (242, 205, 205), None), make_colors_rgb(v, (243, 139, 168), None));
    println!("{} - {}", make_colors_rgb("Docker Usage Utility", (180, 190, 254), None), make_colors_rgb("A better 'docker ps' alternative", (249, 226, 175), None));
}

fn print_help() {
    print_version();
    println!();
    println!("{}", make_colors_rgb("USAGE:", (203, 166, 247), None));
    println!("    dof [OPTIONS]");
    println!();
    println!("{}", make_colors_rgb("OPTIONS:", (203, 166, 247), None));
    println!("    {}, -r       {}", make_colors_rgb("--running", (166, 227, 161), None), make_colors_rgb("Show only running containers", (205, 214, 244), None));
    println!("    {}           {}", make_colors_rgb("--no-trunc", (166, 227, 161), None), make_colors_rgb("Show full container IDs without truncation", (205, 214, 244), None));
    println!("    {}, -v       {}", make_colors_rgb("--version", (166, 227, 161), None), make_colors_rgb("Print version information", (205, 214, 244), None));
    println!("    {}, -h       {}", make_colors_rgb("--help", (166, 227, 161), None), make_colors_rgb("Print help information", (205, 214, 244), None));
    println!();
    println!("{}", make_colors_rgb("EXAMPLES:", (203, 166, 247), None));
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

fn colorize_ports(ports_str: &str) -> String {
    let re = Regex::new(r"(\d+)").unwrap();
    re.replace_all(ports_str, |caps: &regex::Captures| {
        make_colors_rgb(&caps[1], (249, 226, 175), None)
    })
    .to_string()
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

fn system_memory_bytes() -> u64 {
    static MEM: OnceLock<u64> = OnceLock::new();
    *MEM.get_or_init(|| {
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
    })
}

fn cpu_gauge(percent: f64, width: usize) -> String {
    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width.saturating_sub(filled);

    let filled_str = if filled > 0 {
        make_colors_rgb(&"\u{2588}".repeat(filled), (166, 227, 161), None)
    } else {
        String::new()
    };
    let empty_str = if empty > 0 {
        make_colors_rgb(&"\u{2591}".repeat(empty), (49, 50, 68), None)
    } else {
        String::new()
    };

    format!("{}{} {:.1}%", filled_str, empty_str, percent)
}

fn build_table(
    containers: Vec<bollard::models::ContainerSummary>,
    stats_map: std::collections::HashMap<String, (f64, String)>,
    args: &Args,
) -> tabled::Table {
    let mut builder = Builder::new();

    // Header
    let header = [
        make_colors_rgb("ID", (203, 166, 247), None),
        make_colors_rgb("Name", (203, 166, 247), None),
        make_colors_rgb("CPU", (203, 166, 247), None),
        make_colors_rgb("Memory", (203, 166, 247), None),
        make_colors_rgb("Health", (203, 166, 247), None),
        make_colors_rgb("Status", (203, 166, 247), None),
        make_colors_rgb("Ports", (203, 166, 247), None),
    ];
    builder.push_record(header);

    for container in containers.iter() {
        let id = container.id.clone().unwrap_or_default();
        let is_running = container.state.as_ref()
            .map(|s| s.to_string().to_lowercase() == "running")
            .unwrap_or(false);

        let (cpu_str, mem_str) = if let Some((cpu_pct, mem)) = stats_map.get(&id) {
            (cpu_gauge(*cpu_pct, 8), mem.clone())
        } else {
            (
                if is_running { cpu_gauge(0.0, 8) } else { "-".to_string() },
                if is_running { "0B/0B".to_string() } else { "-".to_string() }
            )
        };

        let (status_text, is_running_status) = format_status(&container.state);
        let (health_symbol, is_healthy, is_unhealthy) = format_health(&container.status);

        let health_colored = if is_healthy {
            make_colors_rgb(&health_symbol, (166, 227, 161), None)
        } else if is_unhealthy {
            make_colors_rgb(&health_symbol, (243, 139, 168), None)
        } else if container.status.as_ref().map(|s| s.to_lowercase().contains("starting")).unwrap_or(false) {
            make_colors_rgb(&health_symbol, (249, 226, 175), None)
        } else {
            make_colors_rgb(&health_symbol, (108, 112, 134), None)
        };

        // Status styling: Running = green fg, Stopped = pink bg + dark fg
        let status_colored = if is_running_status {
            make_colors_rgb(&status_text, (166, 227, 161), None) // green
        } else {
            // Stopped: background #f38ba8 (243, 139, 168), foreground #11111b (17, 17, 27)
            make_colors_hex_with_attrs("Stopped", "#11111b", Some("#f38ba8"), &["bold"]).unwrap()
        };

        let ports_text = colorize_ports(&format_ports(&container.ports));

        builder.push_record([
            make_colors_rgb(&format_id(&container.id, args.no_trunc), (180, 190, 254), None),
            make_colors_rgb(&format_names(&container.names), (148, 226, 213), None),
            cpu_str,
            make_colors_rgb(&mem_str, (137, 180, 250), None),
            health_colored,
            status_colored,
            make_colors_rgb(&ports_text, (205, 214, 244), None),
        ]);
    }

    let mut table = builder.build();
    table.with(Style::modern_rounded());
    table.with(Modify::new(Columns::new(4..5)).with(Alignment::center()));
    table.with(Modify::new(Columns::new(5..6)).with(Alignment::center()));
    table.with(Padding::new(1, 1, 0, 0));

    table
}

#[tokio::main]
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
        std::collections::HashMap::new()
    } else {
        let cgroup_map = {
            let ids_ref: Vec<&str> = running_ids.iter().map(|s| s.as_str()).collect();
            cgroup::build_cgroup_map(&ids_ref)
        };

        let mut t0: std::collections::HashMap<String, cgroup::CgroupSnapshot> = std::collections::HashMap::new();
        for (id, path) in &cgroup_map {
            if let Some(snap) = cgroup::read_snapshot(path) {
                t0.insert(id.clone(), snap);
            }
        }

        sleep(Duration::from_millis(100)).await;

        let mut result = std::collections::HashMap::new();
        for (id, path) in &cgroup_map {
            if let (Some(snap0), Some(snap1)) = (t0.get(id), cgroup::read_snapshot(path)) {
                let cpu_delta = snap1.cpu_usage_usec.saturating_sub(snap0.cpu_usage_usec);
                let time_delta_ms = 100;
                let cpu_percent = (cpu_delta as f64 / (time_delta_ms * 1000) as f64) * 100.0;

                let limit = if snap1.memory_limit == u64::MAX {
                    sys_mem
                } else {
                    snap1.memory_limit
                };
                let mem_str = format!("{}/{}", human_bytes(snap1.memory_bytes), human_bytes(limit));

                result.insert(id.clone(), (cpu_percent, mem_str));
            }
        }

        result
    };

    print_version();
    println!();

    let table = build_table(containers, stats_map, &args);
    println!("{}", table);
}
