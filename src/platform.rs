/// A minimal record from the OS's socket table:
/// just what we need to look up the owning process.
#[derive(Debug, Clone)]
pub struct SocketEntry {
    pub pid: u32,
    pub port: u16,
    pub proto: Proto,
    pub state: SocketState,
    pub local_addr: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Proto {
    Tcp,
    Tcp6,
    Udp,
    Udp6,
}

impl std::fmt::Display for Proto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Proto::Tcp => write!(f, "TCP"),
            Proto::Tcp6 => write!(f, "TCP6"),
            Proto::Udp => write!(f, "UDP"),
            Proto::Udp6 => write!(f, "UDP6"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SocketState {
    Listen,
    Established,
    Other(String),
}

impl std::fmt::Display for SocketState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SocketState::Listen => write!(f, "LISTEN"),
            SocketState::Established => write!(f, "ESTABLISHED"),
            SocketState::Other(s) => write!(f, "{}", s),
        }
    }
}

/// Entry point: find all processes listening on or connected to `port`.
pub fn find_port(port: u16) -> Result<Vec<SocketEntry>, String> {
    #[cfg(target_os = "linux")]
    return linux::find_port(port);

    #[cfg(target_os = "macos")]
    return macos::find_port(port);

    #[cfg(target_os = "windows")]
    return Ok(windows::all_socket_entries()?.into_iter().filter(|e| e.port == port).collect());

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    Err("Unsupported platform. portmortem supports Linux, macOS, and Windows.".to_string())
}

// ── Linux (/proc/net) ──────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::collections::HashMap;
    use std::fs;

    pub fn find_port(port: u16) -> Result<Vec<SocketEntry>, String> {
        let mut results = Vec::new();

        // Build inode→pid map from /proc/*/fd
        let inode_to_pid = build_inode_map();

        for (filename, proto) in &[
            ("/proc/net/tcp", Proto::Tcp),
            ("/proc/net/tcp6", Proto::Tcp6),
            ("/proc/net/udp", Proto::Udp),
            ("/proc/net/udp6", Proto::Udp6),
        ] {
            if let Ok(content) = fs::read_to_string(filename) {
                for line in content.lines().skip(1) {
                    if let Some(entry) =
                        parse_proc_net_line(line, proto.clone(), port, &inode_to_pid)
                    {
                        results.push(entry);
                    }
                }
            }
        }

        // Deduplicate by (pid, port, proto)
        results.dedup_by_key(|e| (e.pid, e.port, e.proto.to_string()));
        Ok(results)
    }

    fn parse_proc_net_line(
        line: &str,
        proto: Proto,
        target_port: u16,
        inode_to_pid: &HashMap<u64, u32>,
    ) -> Option<SocketEntry> {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // /proc/net/tcp columns:
        // sl local_address rem_address st tx_queue:rx_queue tr:tm->when retrnsmt uid timeout inode
        if cols.len() < 10 {
            return None;
        }

        let local = cols[1]; // "0100007F:1F90"
        let state_hex = cols[3];
        let inode_str = cols[9];

        let port = parse_hex_port(local)?;
        if port != target_port {
            return None;
        }

        let inode: u64 = inode_str.parse().ok()?;
        let pid = *inode_to_pid.get(&inode)?;

        let state = match state_hex {
            "0A" => SocketState::Listen,
            "01" => SocketState::Established,
            other => SocketState::Other(format!("0x{}", other)),
        };

        Some(SocketEntry { pid, port, proto, state, local_addr: format_local_addr(local) })
    }

    fn parse_hex_port(addr: &str) -> Option<u16> {
        // Format: "XXXXXXXX:PPPP" where PPPP is port in hex
        let port_str = addr.split(':').nth(1)?;
        u16::from_str_radix(port_str, 16).ok()
    }

    fn format_local_addr(raw: &str) -> String {
        // "0100007F:1F90" → "127.0.0.1:8080"
        let parts: Vec<&str> = raw.split(':').collect();
        if parts.len() != 2 {
            return raw.to_string();
        }
        let ip_hex = parts[0];
        let port_hex = parts[1];
        let port = u16::from_str_radix(port_hex, 16).unwrap_or(0);

        // IPv4 in /proc/net/tcp is little-endian 32-bit hex
        if ip_hex.len() == 8 {
            if let Ok(n) = u32::from_str_radix(ip_hex, 16) {
                let a = n & 0xFF;
                let b = (n >> 8) & 0xFF;
                let c = (n >> 16) & 0xFF;
                let d = (n >> 24) & 0xFF;

                if a == 0 && b == 0 && c == 0 && d == 0 {
                    return format!("0.0.0.0:{}", port);
                }
                return format!("{}.{}.{}.{}:{}", a, b, c, d, port);
            }
        }
        raw.to_string()
    }

    /// Walk /proc/<pid>/fd/* and map socket inodes → PIDs.
    /// This requires read access to /proc/<pid>/fd.
    fn build_inode_map() -> HashMap<u64, u32> {
        let mut map = HashMap::new();
        let proc = match fs::read_dir("/proc") {
            Ok(d) => d,
            Err(_) => return map,
        };

        for entry in proc.flatten() {
            let fname = entry.file_name();
            let pid_str = fname.to_string_lossy();
            let pid: u32 = match pid_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            let fd_path = format!("/proc/{}/fd", pid);
            let fds = match fs::read_dir(&fd_path) {
                Ok(d) => d,
                Err(_) => continue,
            };

            for fd in fds.flatten() {
                if let Ok(target) = fs::read_link(fd.path()) {
                    let t = target.to_string_lossy();
                    // Symlinks look like "socket:[12345678]"
                    if let Some(inode_str) =
                        t.strip_prefix("socket:[").and_then(|s| s.strip_suffix(']'))
                    {
                        if let Ok(inode) = inode_str.parse::<u64>() {
                            map.insert(inode, pid);
                        }
                    }
                }
            }
        }
        map
    }
}

// ── macOS (lsof) ──────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::process::Command;

    pub fn find_port(port: u16) -> Result<Vec<SocketEntry>, String> {
        // -F field format: p=pid, f=fd, a=access, P=protocol, T=TCP-state, n=name(addr)
        // Field order from lsof: p → f → a → P → T → n  (NOT p → P → T → n → f)
        let output = Command::new("lsof")
            .args(["-nP", &format!("-i:{}", port), "-F", "fpPTn"])
            .output()
            .map_err(|e| format!("lsof not found: {}", e))?;

        // lsof exits non-zero when nothing is found — that's not an error for us
        if output.stdout.is_empty() {
            return Ok(vec![]);
        }

        if std::env::var("PORTMORTEM_DEBUG").is_ok() {
            eprintln!("=== lsof raw output ===");
            eprintln!("{}", String::from_utf8_lossy(&output.stdout));
            eprintln!("=== end ===");
        }

        parse_lsof_output(&String::from_utf8_lossy(&output.stdout), port)
    }

    /// lsof -F output structure (one field per line):
    ///
    ///   p<pid>          ← process block starts
    ///   f<fd>           ← file block starts  (e.g. "f23u")
    ///   a<mode>         ← access mode (r/w/u)
    ///   P<proto>        ← protocol name (TCP, UDP, IP, ...)
    ///   T ST=<state>    ← TCP state (only for TCP sockets)
    ///   n<addr>         ← local->remote address (e.g. "*:9999->*:*")
    ///   f<fd>           ← next file block (or next p<pid>)
    ///
    /// We accumulate fields into a pending record and flush it when we see
    /// the *next* 'f' line or the end of input — never on the *current* 'f'.
    fn parse_lsof_output(output: &str, port: u16) -> Result<Vec<SocketEntry>, String> {
        let mut results = Vec::new();

        // Pending record fields
        let mut pid: Option<u32> = None;
        let mut proto: Option<Proto> = None;
        let mut state = SocketState::Listen;
        let mut addr = String::new();
        let mut in_file_block = false;

        let flush = |results: &mut Vec<SocketEntry>,
                     pid: Option<u32>,
                     proto: Option<Proto>,
                     state: SocketState,
                     addr: String| {
            if let (Some(p), Some(pr)) = (pid, proto) {
                results.push(SocketEntry { pid: p, port, proto: pr, state, local_addr: addr });
            }
        };

        for line in output.lines() {
            if line.is_empty() {
                continue;
            }

            let (tag, value) = line.split_at(1);

            match tag {
                "p" => {
                    // New process block — flush whatever we had
                    if in_file_block {
                        flush(&mut results, pid, proto.take(), state.clone(), addr.clone());
                        in_file_block = false;
                        state = SocketState::Listen;
                        addr = String::new();
                    }
                    pid = value.parse().ok();
                }
                "f" => {
                    // New file descriptor block — flush previous file block
                    if in_file_block {
                        flush(&mut results, pid, proto.take(), state.clone(), addr.clone());
                        state = SocketState::Listen;
                        addr = String::new();
                    }
                    in_file_block = true;
                }
                "P" => {
                    proto = Some(match value {
                        "TCP" => Proto::Tcp,
                        "TCP6" => Proto::Tcp6,
                        "UDP" => Proto::Udp,
                        "UDP6" => Proto::Udp6,
                        // nc on macOS may report "IP" or "IPv6" for raw sockets
                        other if other.contains('6') => Proto::Udp6,
                        _ => Proto::Udp,
                    });
                }
                "T" => {
                    // "ST=LISTEN", "ST=ESTABLISHED", etc.
                    if let Some(s) = value.strip_prefix("ST=") {
                        state = match s {
                            "LISTEN" => SocketState::Listen,
                            "ESTABLISHED" => SocketState::Established,
                            other => SocketState::Other(other.to_string()),
                        };
                    }
                }
                "n" => {
                    // "*:9999->*:*" — take just the local side
                    addr = value.split("->").next().unwrap_or(value).to_string();
                }
                _ => {}
            }
        }

        // Flush final pending record
        if in_file_block {
            flush(&mut results, pid, proto, state, addr);
        }

        // Deduplicate (same pid+port can appear via both IPv4 and IPv6 sockets)
        results.dedup_by_key(|e| (e.pid, e.proto.to_string()));
        Ok(results)
    }
}

// ── Windows (IP Helper API) ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub(crate) use windows::all_socket_entries;

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::net::Ipv6Addr;

    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID,
        MIB_UDP6ROW_OWNER_PID, MIB_UDPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
    };
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};

    const MIB_TCP_STATE_LISTEN: u32 = 2;
    const MIB_TCP_STATE_ESTAB: u32 = 5;
    const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
    const NO_ERROR: u32 = 0;

    /// Every TCP/UDP, IPv4/IPv6 socket currently known to the system, with owning PID.
    /// Shared by `find_port` (filters by port) and `process::read_extra_ports` (filters by pid).
    pub(crate) fn all_socket_entries() -> Result<Vec<SocketEntry>, String> {
        let mut results = Vec::new();
        results.extend(tcp4_entries()?);
        results.extend(tcp6_entries()?);
        results.extend(udp4_entries()?);
        results.extend(udp6_entries()?);
        Ok(results)
    }

    fn tcp_state(raw: u32) -> SocketState {
        match raw {
            MIB_TCP_STATE_LISTEN => SocketState::Listen,
            MIB_TCP_STATE_ESTAB => SocketState::Established,
            other => SocketState::Other(other.to_string()),
        }
    }

    /// Ports in these structures are 16 bits, stored network-byte-order in the
    /// low half of a DWORD (the upper 16 bits are unspecified/uninitialized).
    fn extract_port(raw: u32) -> u16 {
        u16::from_be((raw & 0xFFFF) as u16)
    }

    /// `dwLocalAddr` is a raw copy of an `in_addr`'s bytes — read them in
    /// memory order (no endian conversion), same as the `inet_ntoa` example
    /// in Microsoft's own docs for this struct.
    fn format_ipv4(addr: u32, port_raw: u32) -> String {
        let b = addr.to_ne_bytes();
        format!("{}.{}.{}.{}:{}", b[0], b[1], b[2], b[3], extract_port(port_raw))
    }

    fn format_ipv6(addr: [u8; 16], port_raw: u32) -> String {
        format!("[{}]:{}", Ipv6Addr::from(addr), extract_port(port_raw))
    }

    /// Calls a `GetExtended*Table`-shaped function with the grow-and-retry
    /// pattern Microsoft's own docs use: first call learns the required size,
    /// then a real call fills a buffer of that size.
    fn fetch_table(mut call: impl FnMut(*mut c_void, *mut u32) -> u32) -> Result<Vec<u8>, String> {
        let mut size: u32 = 0;
        let _ = call(std::ptr::null_mut(), &mut size);

        for _ in 0..5 {
            let mut buf = vec![0u8; size as usize];
            match call(buf.as_mut_ptr() as *mut c_void, &mut size) {
                NO_ERROR => return Ok(buf),
                ERROR_INSUFFICIENT_BUFFER => continue, // size was updated, retry
                other => return Err(format!("IP Helper API call failed (error {})", other)),
            }
        }
        Err("failed to read socket table after multiple attempts".to_string())
    }

    fn tcp4_entries() -> Result<Vec<SocketEntry>, String> {
        let buf = fetch_table(|ptr, size| unsafe {
            GetExtendedTcpTable(ptr, size, 0, AF_INET as u32, TCP_TABLE_OWNER_PID_ALL, 0)
        })?;

        let num_entries = unsafe { *(buf.as_ptr() as *const u32) } as usize;
        let rows_ptr = unsafe { buf.as_ptr().add(size_of::<u32>()) as *const MIB_TCPROW_OWNER_PID };
        let rows = unsafe { std::slice::from_raw_parts(rows_ptr, num_entries) };

        Ok(rows
            .iter()
            .map(|r| SocketEntry {
                pid: r.dwOwningPid,
                port: extract_port(r.dwLocalPort),
                proto: Proto::Tcp,
                state: tcp_state(r.dwState),
                local_addr: format_ipv4(r.dwLocalAddr, r.dwLocalPort),
            })
            .collect())
    }

    fn tcp6_entries() -> Result<Vec<SocketEntry>, String> {
        let buf = fetch_table(|ptr, size| unsafe {
            GetExtendedTcpTable(ptr, size, 0, AF_INET6 as u32, TCP_TABLE_OWNER_PID_ALL, 0)
        })?;

        let num_entries = unsafe { *(buf.as_ptr() as *const u32) } as usize;
        let rows_ptr =
            unsafe { buf.as_ptr().add(size_of::<u32>()) as *const MIB_TCP6ROW_OWNER_PID };
        let rows = unsafe { std::slice::from_raw_parts(rows_ptr, num_entries) };

        Ok(rows
            .iter()
            .map(|r| SocketEntry {
                pid: r.dwOwningPid,
                port: extract_port(r.dwLocalPort),
                proto: Proto::Tcp6,
                state: tcp_state(r.dwState),
                local_addr: format_ipv6(r.ucLocalAddr, r.dwLocalPort),
            })
            .collect())
    }

    fn udp4_entries() -> Result<Vec<SocketEntry>, String> {
        let buf = fetch_table(|ptr, size| unsafe {
            GetExtendedUdpTable(ptr, size, 0, AF_INET as u32, UDP_TABLE_OWNER_PID, 0)
        })?;

        let num_entries = unsafe { *(buf.as_ptr() as *const u32) } as usize;
        let rows_ptr = unsafe { buf.as_ptr().add(size_of::<u32>()) as *const MIB_UDPROW_OWNER_PID };
        let rows = unsafe { std::slice::from_raw_parts(rows_ptr, num_entries) };

        Ok(rows
            .iter()
            .map(|r| SocketEntry {
                pid: r.dwOwningPid,
                port: extract_port(r.dwLocalPort),
                proto: Proto::Udp,
                // UDP is connectionless — no real state, matches the macOS
                // lsof path where UDP sockets default to Listen too.
                state: SocketState::Listen,
                local_addr: format_ipv4(r.dwLocalAddr, r.dwLocalPort),
            })
            .collect())
    }

    fn udp6_entries() -> Result<Vec<SocketEntry>, String> {
        let buf = fetch_table(|ptr, size| unsafe {
            GetExtendedUdpTable(ptr, size, 0, AF_INET6 as u32, UDP_TABLE_OWNER_PID, 0)
        })?;

        let num_entries = unsafe { *(buf.as_ptr() as *const u32) } as usize;
        let rows_ptr =
            unsafe { buf.as_ptr().add(size_of::<u32>()) as *const MIB_UDP6ROW_OWNER_PID };
        let rows = unsafe { std::slice::from_raw_parts(rows_ptr, num_entries) };

        Ok(rows
            .iter()
            .map(|r| SocketEntry {
                pid: r.dwOwningPid,
                port: extract_port(r.dwLocalPort),
                proto: Proto::Udp6,
                state: SocketState::Listen,
                local_addr: format_ipv6(r.ucLocalAddr, r.dwLocalPort),
            })
            .collect())
    }
}
