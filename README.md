# portmortem

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

## Usage

```
portmortem [OPTIONS] <PORT>...

Arguments:
  <PORT>...  Port number(s) to investigate

Options:
  -a, --all-ports  Show all ports held by the same process(es)
  -j, --json       Output as JSON (newline-delimited, one object per match)
  -k, --kill       Kills binded process
  -h, --help       Print help
  -V, --version    Print version
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
├── process.rs    Process enrichment (binary, cmdline, cwd, user, uptime)
└── render.rs     Human-readable + JSON output
```

No runtime. No daemon. Zero configuration. Single binary, ~2MB release.

## Roadmap

- [x] `--kill` flag
- [ ] `--watch` mode: re-runs every N seconds
- [ ] Windows support (via `netstat` + WMI)
- [x] Shell completions

## License

MIT