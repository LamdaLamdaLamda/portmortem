# syntax=docker/dockerfile:1
#
# Builds portmortem for Linux and smoke-tests the resulting binary in a
# minimal Debian container — useful for verifying the target_os = "linux"
# code path (platform.rs / process.rs) on machines that aren't Linux.
#
# Usage:
#   docker build -t portmortem-linux-test .
#   docker run --rm portmortem-linux-test

########################################
# Build stage
########################################
FROM rust:1-bookworm AS builder
WORKDIR /build

# Cache dependency builds separately from source changes
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release

########################################
# Test stage
########################################
FROM debian:bookworm-slim AS test

RUN apt-get update \
    && apt-get install -y --no-install-recommends netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/portmortem /usr/local/bin/portmortem
COPY docker/smoke-test.sh /usr/local/bin/smoke-test.sh
RUN chmod +x /usr/local/bin/smoke-test.sh

ENTRYPOINT ["/usr/local/bin/smoke-test.sh"]
