use crate::process::{format_duration, ProcessInfo};
use colored::Colorize;

/// Pretty-printed human output — the main UX.
pub fn as_human(info: &ProcessInfo) {
    // ── Header ──────────────────────────────────────────────────────────
    println!(
        "\n{} {} {} {}",
        "●".red().bold(),
        format!("Port {}", info.port).bold(),
        "is held by".dimmed(),
        format!("PID {}", info.pid).yellow().bold(),
    );

    // ── Process identity ────────────────────────────────────────────────
    if let Some(ref path) = info.binary_path {
        println!("  {}   {}", label("Binary"), path.cyan());
    }

    if let Some(ref cmd) = info.cmdline {
        // Truncate long command lines
        let display = if cmd.len() > 120 { format!("{}…", &cmd[..119]) } else { cmd.clone() };
        println!("  {}      {}", label("Cmd"), display.white());
    }

    if let Some(ref user) = info.username {
        println!("  {}     {}", label("User"), user.white());
    }

    // ── Timing ──────────────────────────────────────────────────────────
    if let Some(ref dur) = info.started_ago {
        println!("  {}  {} ago", label("Started"), format_duration(*dur).green());
    }

    // ── Working directory ────────────────────────────────────────────────
    if let Some(ref cwd) = info.cwd {
        println!("  {}      {}", label("Cwd"), cwd.dimmed());
    }

    // ── Socket details ───────────────────────────────────────────────────
    println!(
        "  {}   {} / {}  ({})",
        label("Socket"),
        info.proto.to_string().blue(),
        info.local_addr.dimmed(),
        format!("{}", info.state).dimmed(),
    );

    // ── Extra ports ──────────────────────────────────────────────────────
    if !info.extra_ports.is_empty() {
        let port_list: Vec<String> = info
            .extra_ports
            .iter()
            .map(|ep| {
                format!("{} {}", ep.port.to_string().yellow(), format!("({})", ep.state).dimmed())
            })
            .collect();
        println!("  {}  {}", label("Also on"), port_list.join("  "));
    }
    println!();
}

fn label(s: &str) -> String {
    format!("{:>7}", s).dimmed().to_string()
}

/// Minimal JSON output — one object per call, newline-delimited.
pub fn as_json(info: &ProcessInfo) {
    // Hand-rolled to avoid serde dependency in v0.1.
    // Clean enough for jq piping.
    let binary = json_str_opt(info.binary_path.as_deref());
    let cmdline = json_str_opt(info.cmdline.as_deref());
    let cwd = json_str_opt(info.cwd.as_deref());
    let username = json_str_opt(info.username.as_deref());
    let started_secs =
        info.started_ago.map(|d| d.as_secs().to_string()).unwrap_or_else(|| "null".to_string());

    let extra: Vec<String> = info
        .extra_ports
        .iter()
        .map(|ep| {
            format!(r#"{{"port":{},"proto":"{}","state":"{}"}}"#, ep.port, ep.proto, ep.state)
        })
        .collect();

    println!(
        r#"{{"pid":{},"port":{},"proto":"{}","state":"{}","local_addr":{},"binary":{},"cmdline":{},"cwd":{},"user":{},"started_ago_secs":{},"extra_ports":[{}]}}"#,
        info.pid,
        info.port,
        info.proto,
        info.state,
        json_str_opt(Some(&info.local_addr)),
        binary,
        cmdline,
        cwd,
        username,
        started_secs,
        extra.join(",")
    );
}

fn json_str_opt(s: Option<&str>) -> String {
    match s {
        None => "null".to_string(),
        Some(v) => format!(r#""{}""#, v.replace('"', r#"\""#).replace('\\', "\\\\")),
    }
}
