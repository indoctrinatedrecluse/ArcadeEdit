# ==============================================================================
# ArcadeEdit Containerized Runtime & Headless Automation Host (v1.2.0)
# Multi-stage build providing headless file inspection, search/replace,
# and integrated companion CLI tooling (ir v3.8) inside Docker / Linux containers.
# ==============================================================================

# Stage 1: Build arcade-headless binary
FROM rust:1.80-slim-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace sources
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Compile arcade-headless in release mode
RUN cargo build --release -p arcade-headless

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim AS runtime

WORKDIR /workspace

# Install runtime utilities & certificates
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    tar \
    procps \
    && rm -rf /var/lib/apt/lists/*

# Install bundled `ir` CLI utility companion (v3.8)
RUN curl -fsSL "https://github.com/indoctrinatedrecluse/ir-cli-utility/releases/download/v3.8/ir-linux.tar.gz" -o /tmp/ir-linux.tar.gz \
    && mkdir -p /tmp/ir_dist \
    && tar -xzf /tmp/ir-linux.tar.gz -C /tmp/ir_dist \
    && cp /tmp/ir_dist/ir /usr/local/bin/ir \
    && cp /tmp/ir_dist/term-sys-monitor-linux /usr/local/bin/term-sys-monitor-linux \
    && chmod +x /usr/local/bin/ir /usr/local/bin/term-sys-monitor-linux \
    && rm -rf /tmp/ir-linux.tar.gz /tmp/ir_dist

# Copy compiled arcade-headless from builder
COPY --from=builder /build/target/release/arcade-headless /usr/local/bin/arcade-headless
RUN chmod +x /usr/local/bin/arcade-headless

# Set default workspace directory
VOLUME ["/workspace"]

# Default entrypoint to arcade-headless CLI
ENTRYPOINT ["arcade-headless"]
CMD ["--help"]

