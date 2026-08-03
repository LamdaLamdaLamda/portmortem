#!/bin/bash
# Smoke-tests portmortem's Linux code path (/proc/net/*, /proc/<pid>/fd,
# /proc/<pid>/status) inside the container. Mirrors the checks in
# .github/workflows/macos.yml so both platforms get the same coverage.
set -euo pipefail

echo "--- version / help ---"
portmortem --version
portmortem --help

echo "--- free port ---"
portmortem 65432

echo "--- occupied port ---"
nc -lk 8123 &
LISTENER_PID=$!
sleep 1

portmortem 8123
portmortem 8123 --json | tee /tmp/out.json
grep -q '"port":8123' /tmp/out.json

echo "--- --kill flag ---"
portmortem 8123 --kill
sleep 1

if kill -0 "$LISTENER_PID" 2>/dev/null; then
    echo "FAIL: listener still alive after --kill" >&2
    exit 1
fi

echo "All smoke tests passed."
