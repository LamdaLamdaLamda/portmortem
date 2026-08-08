# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.2.0] - 2026-08-06

### Added

- Windows support: socket table via the IP Helper API
  (`GetExtendedTcpTable`/`GetExtendedUdpTable`), process enrichment via the
  `sysinfo` crate, `--kill` via `taskkill`
- Prebuilt release binaries for Linux (x86_64), macOS (arm64/x86_64), and
  Windows (x86_64), with a `SHA256SUMS` file per release

### Fixed

- `--kill` no longer panics on failure; it prints a clean error and exits
  with a non-zero status instead
- A process bound dual-stack (IPv4 + IPv6 on the same port) was killed
  twice by `--kill`, causing a spurious "process not found"/"access
  denied" error on the second attempt. Already-terminated pids are now
  tracked and skipped.

## [1.1.0] - 2026-08-03

### Added

- `-w`, `--watch <SECONDS>` to re-run at a fixed interval, clearing the
  screen between updates. Human-readable output only, mutually exclusive
  with `--json`
- CI, License, and Release badges to README.md

### Security

- Bump `crossbeam-epoch` to `0.9.20`, fixing
  [RUSTSEC-2026-0204](https://rustsec.org/advisories/RUSTSEC-2026-0204)

## [1.0.0] - 2026-08-03

### Added

- Port lookup by number, resolving the owning process's binary path, cmdline,
  user, working directory, and uptime
- Support for querying multiple ports in a single invocation
  (`portmortem 80 443 8080`)
- `-a`, `--all-ports` to show all other ports held by the same process
- `-j`, `--json` for newline-delimited JSON output, one object per match
- `-k`, `--kill` to terminate the process holding a port (sends `SIGTERM`)
- Linux support via `/proc/net/tcp*`, `/proc/net/udp*`, and `/proc/*/fd`
  inode-to-PID mapping
- macOS support via `lsof -F` (machine-readable output mode)
- Shell completion scripts for bash, zsh, fish, and nu

[Unreleased]: https://github.com/LamdaLamdaLamda/portmortem/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/LamdaLamdaLamda/portmortem/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/LamdaLamdaLamda/portmortem/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/LamdaLamdaLamda/portmortem/releases/tag/v1.0.0
