# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- CI, License, and Release badges to README.md

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

[Unreleased]: https://github.com/LamdaLamdaLamda/portmortem/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/LamdaLamdaLamda/portmortem/releases/tag/v1.0.0
