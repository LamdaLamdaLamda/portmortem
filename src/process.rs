use crate::platform::{Proto, SocketEntry, SocketState};
use std::time::{Duration, SystemTime};

/// Everything we know about the process owning a port.
#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub port: u16,
    pub proto: Proto,
    pub state: SocketState,
    pub local_addr: String,

    // Enriched fields
    pub binary_path: Option<String>,
    pub cmdline: Option<String>,
    pub cwd: Option<String>,
    pub username: Option<String>,
    pub started_ago: Option<Duration>,
    pub extra_ports: Vec<ExtraPort>,
}

#[derive(Debug)]
pub struct ExtraPort {
    pub port: u16,
    pub proto: String,
    pub state: String,
}

pub fn enrich(entry: &SocketEntry, collect_extra_ports: bool) -> Result<ProcessInfo, String> {
    let pid = entry.pid;

    let binary_path = read_binary_path(pid);
    let cmdline = read_cmdline(pid);
    let cwd = read_cwd(pid);
    let username = read_username(pid);
    let started_ago = read_start_time(pid);
    let extra_ports = if collect_extra_ports { read_extra_ports(pid, entry.port) } else { vec![] };

    Ok(ProcessInfo {
        pid,
        port: entry.port,
        proto: entry.proto.clone(),
        state: entry.state.clone(),
        local_addr: entry.local_addr.clone(),
        binary_path,
        cmdline,
        cwd,
        username,
        started_ago,
        extra_ports,
    })
}

// ── Linux: read from /proc ─────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn read_binary_path(pid: u32) -> Option<String> {
    std::fs::read_link(format!("/proc/{}/exe", pid)).ok().map(|p| p.to_string_lossy().into_owned())
}

#[cfg(target_os = "linux")]
fn read_cmdline(pid: u32) -> Option<String> {
    let raw = std::fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    // NUL-delimited args
    let s: String = raw
        .split(|&b| b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(target_os = "linux")]
fn read_cwd(pid: u32) -> Option<String> {
    std::fs::read_link(format!("/proc/{}/cwd", pid)).ok().map(|p| p.to_string_lossy().into_owned())
}

#[cfg(target_os = "linux")]
fn read_username(pid: u32) -> Option<String> {
    let status = std::fs::read_to_string(format!("/proc/{}/status", pid)).ok()?;
    let uid_line = status.lines().find(|l| l.starts_with("Uid:"))?;
    let uid: u32 = uid_line.split_whitespace().nth(1)?.parse().ok()?;
    uid_to_username(uid)
}

#[cfg(target_os = "linux")]
fn read_start_time(pid: u32) -> Option<Duration> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    // Field 22 (0-indexed) is starttime in clock ticks since boot
    // We need to get btime from /proc/stat and clock ticks/sec from sysconf
    let fields: Vec<&str> = stat.split_whitespace().collect();
    let start_ticks: u64 = fields.get(21)?.parse().ok()?;

    let boot_time = read_boot_time()?;
    let ticks_per_sec = ticks_per_second();

    let start_secs = boot_time + start_ticks / ticks_per_sec;
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs();

    if now > start_secs {
        Some(Duration::from_secs(now - start_secs))
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn read_boot_time() -> Option<u64> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("btime ") {
            return rest.trim().parse().ok();
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn ticks_per_second() -> u64 {
    // Safe default; real value from sysconf(_SC_CLK_TCK) is almost always 100
    // on modern Linux. We avoid libc dependency by hardcoding here.
    100
}

#[cfg(target_os = "linux")]
fn read_extra_ports(pid: u32, exclude_port: u16) -> Vec<ExtraPort> {
    let mut extras = Vec::new();

    // Build inode set for this PID
    let fd_path = format!("/proc/{}/fd", pid);
    let mut inodes = std::collections::HashSet::new();
    if let Ok(fds) = std::fs::read_dir(&fd_path) {
        for fd in fds.flatten() {
            if let Ok(target) = std::fs::read_link(fd.path()) {
                let t = target.to_string_lossy();
                if let Some(inode_str) =
                    t.strip_prefix("socket:[").and_then(|s| s.strip_suffix(']'))
                {
                    if let Ok(inode) = inode_str.parse::<u64>() {
                        inodes.insert(inode);
                    }
                }
            }
        }
    }

    // Scan all net files for matching inodes
    for (file, proto_label) in &[
        ("/proc/net/tcp", "TCP"),
        ("/proc/net/tcp6", "TCP6"),
        ("/proc/net/udp", "UDP"),
        ("/proc/net/udp6", "UDP6"),
    ] {
        if let Ok(content) = std::fs::read_to_string(file) {
            for line in content.lines().skip(1) {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() < 10 {
                    continue;
                }
                let inode: u64 = match cols[9].parse() {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                if !inodes.contains(&inode) {
                    continue;
                }

                let port = match parse_hex_port(cols[1]) {
                    Some(p) => p,
                    None => continue,
                };
                if port == exclude_port {
                    continue;
                }

                let state_str = match cols[3] {
                    "0A" => "LISTEN",
                    "01" => "ESTABLISHED",
                    other => other,
                };

                extras.push(ExtraPort {
                    port,
                    proto: proto_label.to_string(),
                    state: state_str.to_string(),
                });
            }
        }
    }

    extras.sort_by_key(|e| e.port);
    extras.dedup_by_key(|e| e.port);
    extras
}

#[cfg(target_os = "linux")]
fn parse_hex_port(addr: &str) -> Option<u16> {
    let port_str = addr.split(':').nth(1)?;
    u16::from_str_radix(port_str, 16).ok()
}

// ── macOS: read via /proc equiv (sysctl + lsof) ───────────────────────────

#[cfg(target_os = "macos")]
fn read_binary_path(pid: u32) -> Option<String> {
    let output = std::process::Command::new("lsof")
        .args(["-p", &pid.to_string(), "-Fn", "-a", "-d", "txt"])
        .output()
        .ok()?;

    let s = String::from_utf8_lossy(&output.stdout);

    for line in s.lines() {
        if let Some(path) = line.strip_prefix('n') {
            if path.starts_with('/') {
                return Some(path.to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn read_cmdline(pid: u32) -> Option<String> {
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok()?;

    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(target_os = "macos")]
fn read_cwd(pid: u32) -> Option<String> {
    let output = std::process::Command::new("lsof")
        .args(["-p", &pid.to_string(), "-Fn", "-a", "-d", "cwd"])
        .output()
        .ok()?;

    let s = String::from_utf8_lossy(&output.stdout);

    for line in s.lines() {
        if let Some(path) = line.strip_prefix('n') {
            return Some(path.to_string());
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn read_username(pid: u32) -> Option<String> {
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "user="])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(target_os = "macos")]
fn read_start_time(pid: u32) -> Option<Duration> {
    // ps -p <pid> -o etime= gives elapsed time as [[DD-]HH:]MM:SS
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "etime="])
        .output()
        .ok()?;
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_etime(&raw)
}

#[cfg(target_os = "macos")]
fn parse_etime(s: &str) -> Option<Duration> {
    // Format: [[DD-]HH:]MM:SS
    let parts: Vec<&str> = s.split(':').collect();

    let secs: u64 = match parts.len() {
        2 => {
            let mm: u64 = parts[0].parse().ok()?;
            let ss: u64 = parts[1].parse().ok()?;

            mm * 60 + ss
        }
        3 => {
            let hh_part = parts[0];
            let (dd, hh): (u64, u64) = if hh_part.contains('-') {
                let sub: Vec<&str> = hh_part.split('-').collect();
                (sub[0].parse().ok()?, sub[1].parse().ok()?)
            } else {
                (0, hh_part.parse().ok()?)
            };

            let mm: u64 = parts[1].parse().ok()?;
            let ss: u64 = parts[2].parse().ok()?;

            dd * 86400 + hh * 3600 + mm * 60 + ss
        }
        _ => return None,
    };
    Some(Duration::from_secs(secs))
}

#[cfg(target_os = "macos")]
fn read_extra_ports(pid: u32, exclude_port: u16) -> Vec<ExtraPort> {
    let output = match std::process::Command::new("lsof")
        .args(["-p", &pid.to_string(), "-i", "-nP", "-FnTP"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec![],
    };

    let s = String::from_utf8_lossy(&output.stdout);
    let mut extras = Vec::new();
    let mut proto = String::new();
    let mut state = String::new();

    for line in s.lines() {
        match line.chars().next() {
            Some('P') => proto = line[1..].to_string(),
            Some('T') => {
                if let Some(st) = line[1..].strip_prefix("ST=") {
                    state = st.to_string();
                }
            }
            Some('n') => {
                let addr = &line[1..];
                if let Some(port_str) = addr.split(':').last() {
                    if let Ok(port) = port_str.parse::<u16>() {
                        if port != exclude_port {
                            extras.push(ExtraPort {
                                port,
                                proto: proto.clone(),
                                state: state.clone(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    extras.sort_by_key(|e| e.port);
    extras.dedup_by_key(|e| e.port);
    extras
}

// ── Shared helpers ─────────────────────────────────────────────────────────

fn uid_to_username(uid: u32) -> Option<String> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;

    for line in passwd.lines() {
        let fields: Vec<&str> = line.split(':').collect();

        if fields.len() >= 3 {
            if let Ok(u) = fields[2].parse::<u32>() {
                if u == uid {
                    return Some(fields[0].to_string());
                }
            }
        }
    }
    None
}

/// Format a Duration as human-readable "3h 12min", "45min", "23s", etc.
pub fn format_duration(d: Duration) -> String {
    let total = d.as_secs();

    if total < 60 {
        return format!("{}s", total);
    }

    let mins = total / 60;

    if mins < 60 {
        return format!("{}min", mins);
    }

    let hours = mins / 60;
    let rem_mins = mins % 60;

    if hours < 24 {
        if rem_mins == 0 {
            return format!("{}h", hours);
        }
        return format!("{}h {}min", hours, rem_mins);
    }

    let days = hours / 24;
    let rem_hours = hours % 24;

    if rem_hours == 0 {
        return format!("{}d", days);
    }

    format!("{}d {}h", days, rem_hours)
}
