mod kill;
mod platform;
mod process;
mod render;

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use clap::Parser;
use colored::Colorize;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigint(_signum: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

#[derive(Parser)]
#[command(
    name = "portmortem",
    about = "Who's blocking your port, and why?",
    version,
    long_about = None
)]
struct Cli {
    /// Port number(s) to investigate
    #[arg(required_unless_present = "completion", num_args = 1..)]
    ports: Vec<u16>,

    /// Show all ports held by the same process(es)
    #[arg(short = 'a', long = "all-ports")]
    all_ports: bool,

    /// Output as JSON (for scripting)
    #[arg(short = 'j', long = "json")]
    json: bool,

    /// Kills binded process
    #[arg(short = 'k', long = "kill")]
    kill: bool,

    /// Re-run every SECONDS (human-readable output only)
    #[arg(short = 'w', long = "watch", value_name = "SECONDS", conflicts_with = "json")]
    watch: Option<u64>,
}

fn main() {
    let cli = Cli::parse();

    match cli.watch {
        Some(interval_secs) => watch_loop(&cli, interval_secs),
        None => inspect_ports(&cli),
    }
}

/// Clears the screen and re-runs `inspect_ports` every `interval_secs`
/// until interrupted (Ctrl+C).
fn watch_loop(cli: &Cli, interval_secs: u64) {
    unsafe {
        libc::signal(libc::SIGINT, handle_sigint as *const () as libc::sighandler_t);
    }

    let interval = Duration::from_secs(interval_secs);

    while !INTERRUPTED.load(Ordering::SeqCst) {
        print!("\x1B[2J\x1B[1;1H"); // clear screen, move cursor to top-left
        io::stdout().flush().ok();

        let ports = cli.ports.iter().map(u16::to_string).collect::<Vec<_>>().join(", ");
        println!(
            "{} watching port(s) {} — every {}s, Ctrl+C to stop\n",
            "●".cyan().bold(),
            ports.bold(),
            interval_secs
        );

        inspect_ports(cli);
        sleep_interruptible(interval);
    }

    println!("\n{} watch stopped", "✓".green().bold());
}

/// Sleeps for `total`, checking `INTERRUPTED` every 100ms so Ctrl+C
/// during the wait is picked up promptly instead of after the full interval.
fn sleep_interruptible(total: Duration) {
    let step = Duration::from_millis(100);
    let mut waited = Duration::ZERO;

    while waited < total && !INTERRUPTED.load(Ordering::SeqCst) {
        let remaining = total - waited;
        let this_step = step.min(remaining);
        std::thread::sleep(this_step);
        waited += this_step;
    }
}

/// Looks up and renders every port in `cli.ports` once.
fn inspect_ports(cli: &Cli) {
    // A process bound dual-stack (IPv4 + IPv6) on the same port shows up as
    // two separate SocketEntry results with the same pid — track which pids
    // we've already killed so we don't try (and fail) to kill them twice.
    let mut killed_pids = std::collections::HashSet::new();

    for port in &cli.ports {
        match platform::find_port(*port) {
            Ok(entries) if entries.is_empty() => {
                println!("{} Port {} is free", "✓".green().bold(), port.to_string().bold());
            }
            Ok(entries) => {
                for entry in &entries {
                    match process::enrich(entry, cli.all_ports) {
                        Ok(info) => {
                            if cli.json {
                                render::as_json(&info);
                            } else {
                                render::as_human(&info);

                                if cli.kill {
                                    if killed_pids.contains(&info.pid) {
                                        println!(
                                            "{} Process {} already terminated",
                                            "✓".green().bold(),
                                            info.pid.to_string().bold()
                                        );
                                    } else {
                                        match kill::kill_process(info.pid as i32) {
                                            Ok(()) => {
                                                killed_pids.insert(info.pid);
                                                println!(
                                                    "{} Process {} terminated",
                                                    "✓".green().bold(),
                                                    info.pid.to_string().bold()
                                                );
                                            }
                                            Err(e) => {
                                                eprintln!(
                                                    "{} Failed to terminate process {}: {}",
                                                    "✗".red().bold(),
                                                    info.pid.to_string().bold(),
                                                    e
                                                );
                                                std::process::exit(1);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("{} Could not enrich PID {}: {}", "!".yellow(), entry.pid, e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{} Failed to inspect port {}: {}", "✗".red().bold(), port, e);
                std::process::exit(1);
            }
        }
    }
}
