# Syntax: Dockerfile.build using buildkit for better caching
#
# Build:
#   docker build -t openclaw/cli .
# Run:
#   docker run --rm openclaw/cli status
#   docker run --rm -e OPENCLAW_API_KEY=... openclaw/cli demo --provider openai --model gpt-4o-mini --message 'hello'

# ---- Build stage ----
FROM rust:1.75-slim AS builder

WORKDIR /usr/src/openclaw-rs

# Install minimal build deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY crates ./crates
COPY Cargo.toml Cargo.lock ./

# Build dependencies (cache-friendly)
RUN cargo build --release --bin openclaw-cli --locked --target x86_64-unknown-linux-musl || true

# Copy source and build
RUN cp -r crates . && cargo build --release --bin openclaw-cli --locked --target x86_64-unknown-linux-musl

# ---- Runtime stage ----
FROM debian:bookworm-slim

# Create nonroot user
RUN useradd -m -u 10000 nonroot
USER nonroot

WORKDIR /app

COPY --from=builder /usr/src/openclaw-rs/target/x86_64-unknown-linux-musl/release/openclaw-cli /usr/local/bin/openclaw-cli

# Entrypoint runs the CLI
ENTRYPOINT ["/usr/local/bin/openclaw-cli"]
CMD ["status"]
