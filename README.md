# portmortem

![macOS CI](https://github.com/LamdaLamdaLamda/portmortem/actions/workflows/macos.yml/badge.svg)
![Linux CI](https://github.com/LamdaLamdaLamda/portmortem/actions/workflows/linux.yml/badge.svg)
![License](https://img.shields.io/github/license/LamdaLamdaLamda/portmortem)
![Release](https://img.shields.io/github/v/release/LamdaLamdaLamda/portmortem)

**Who's blocking your port, and why?**

`portmortem` gives you everything about a port-holding process in one command — no more chaining `lsof`, `ps`, `ls /proc`, and `cat /proc/*/cmdline`.

## Example

```
$ portmortem 8080

● Port 8080 is held by PID 21443

  Binary   /home/alice/.nvm/versions/node/v20.1.0/bin/node
     Cmd   node server.js --port 8080 --env development
    User   alice
 Started   3h 12min ago
     Cwd   /home/alice/projects/myapp
  Socket   TCP / 0.0.0.0:8080  (LISTEN)
```

With `--all-ports`:

```
$ portmortem 8080 --all-ports

● Port 8080 is held by PID 21443

  Binary   /home/alice/.nvm/versions/node/v20.1.0/bin/node
     Cmd   node server.js
    User   alice
 Started   3h 12min ago
     Cwd   /home/alice/projects/myapp
  Socket   TCP / 0.0.0.0:8080  (LISTEN)
 Also on   8081 (LISTEN)  9229 (LISTEN)
```

Multiple ports at once:

```
$ portmortem 80 443 8080
```

JSON output for scripting:

```
$ portmortem 8080 --json | jq .cmdline
"node server.js --port 8080"
```

## Install

```bash
cargo install --path .
```

Or build a release binary:

```bash
cargo build --release
# Binary: ./target/release/portmortem
sudo cp target/release/portmortem /usr/local/bin/
```

or just

```bash
just deploy
```

## Requirements

- **Linux**: No external dependencies. Reads directly from `/proc/net` and `/proc/<pid>/`.  
  Requires read access to `/proc` — works without root for processes owned by your user.  
  For other users' processes you may need `sudo`.
- **macOS**: Requires `lsof` (installed by default on all macOS versions) and `ps`.
- **Windows**: No external dependencies. Reads the socket table directly via
  the IP Helper API (`GetExtendedTcpTable`/`GetExtendedUdpTable`).

## Usage

```
portmortem [OPTIONS] <PORT>...

Arguments:
  <PORT>...  Port number(s) to investigate

Options:
  -a, --all-ports        Show all ports held by the same process(es)
  -j, --json             Output as JSON (newline-delimited, one object per match)
  -k, --kill             Kills binded process
  -w, --watch <SECONDS>  Re-run every SECONDS (human-readable output only)
  -h, --help             Print help
  -V, --version          Print version
```

Watch a port, clearing the screen every 2 seconds:

```bash
$ portmortem 8080 --watch 2
```

## Why not just use `lsof -i :8080`?

`lsof` gives you a PID and maybe a name. Getting the full picture requires:

```bash
lsof -i :8080          # PID
ps aux | grep 21443    # cmdline
ls -la /proc/21443/exe # binary path
readlink /proc/21443/cwd # working dir
cat /proc/21443/status | grep Uid  # user
```

`portmortem` does all of that in one shot.

## Architecture

```
src/
├── main.rs       CLI parsing (clap), dispatch loop
├── platform.rs   OS-level socket table reading
│                 Linux: /proc/net/tcp*, /proc/*/fd (inode map)
│                 macOS: lsof -F (machine-readable mode)
│                 Windows: GetExtendedTcpTable/GetExtendedUdpTable (IP Helper API)
├── process.rs    Process enrichment (binary, cmdline, cwd, user, uptime)
│                 Windows enrichment uses the `sysinfo` crate — there's no
│                 public API for reading another process's command line
└── render.rs     Human-readable + JSON output
```

No runtime. No daemon. Zero configuration. Single binary, ~2MB release.

## Testing the Linux code path (via Docker)

 The `Dockerfile` builds `portmortem` for Linux and smoke-tests the
resulting binary in a minimal container — useful for verifying that path
before pushing, e.g. from macOS:

```bash
docker build -t portmortem-linux-test .
docker run --rm portmortem-linux-test
```

This builds the binary in `rust:1-bookworm`, then runs it against a real
listener (`nc`) inside a `debian:bookworm-slim` container, checking plain
output, `--json`, and `--kill`. Exit code `0` means all checks passed.

## Roadmap

- [x] `--kill` flag
- [x] `--watch` mode: re-runs every N seconds
- [x] Windows support (via the IP Helper API)
- [x] Shell completions

## License

MIT