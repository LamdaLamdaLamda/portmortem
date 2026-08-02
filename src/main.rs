mod kill;
mod platform;
mod process;
mod render;

use clap::Parser;
use colored::Colorize;

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
}

fn main() {
    let cli = Cli::parse();
    let mut any_found = false;

    for port in &cli.ports {
        match platform::find_port(*port) {
            Ok(entries) if entries.is_empty() => {
                println!("{} Port {} is free", "✓".green().bold(), port.to_string().bold());
            }
            Ok(entries) => {
                any_found = true;

                for entry in &entries {
                    match process::enrich(entry, cli.all_ports) {
                        Ok(info) => {
                            if cli.json {
                                render::as_json(&info);
                            } else {
                                render::as_human(&info);

                                if cli.kill {
                                    kill::kill_process(info.pid as i32)
                                        .expect("Failed to terminate process");
                                    println!(
                                        "{} Process {} terminated",
                                        "✓".green().bold(),
                                        info.pid.to_string().bold()
                                    );
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

    if !any_found && cli.ports.len() == 1 {
        std::process::exit(0);
    }
}
