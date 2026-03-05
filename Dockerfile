# --- Stage 1: Plan Stage ---
# Using cargo-chef to manage dependency layers for lightning-fast builds
FROM rust:1.80-slim-bookworm as planner
WORKDIR /app
RUN cargo install cargo-chef
# Copying lockfile and toml first to ensure the 'recipe' is deterministic
COPY Cargo.toml Cargo.lock ./
COPY . .
RUN cargo chef prepare --recipe-json recipe.json

# --- Stage 2: Build Stage ---
FROM rust:1.80-slim-bookworm as builder
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json

# Install system dependencies required for SHA3 and blockchain cryptography
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Build dependencies only (This layer is cached unless Cargo.lock changes)
# This respects the [patch.crates-io] ring = "=0.17.14" setting
RUN cargo chef cook --release --recipe-json recipe.json

# Build the actual application
COPY . .
# Ensure Solidity ABI file is present for the ethers abigen! macro
RUN if [ ! -f "IntegrityLedger.json" ]; then echo "{}" > IntegrityLedger.json; fi 
RUN cargo build --release

# --- Stage 3: Runtime Stage ---
# Minimal Debian Bookworm image for the smallest possible attack surface
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime SSL certificates for secure RPC communication
RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/veriphys-protocol-core ./veriphys-engine
COPY --from=builder /app/IntegrityLedger.json ./IntegrityLedger.json
# Handle the registry file existence
COPY --from=builder /app/registry.txt ./registry.txt 2>/dev/null || touch registry.txt

# Networking and Environment Settings
EXPOSE 3000
ENV RUST_LOG=info
ENV SERVER_PORT=3000

# Security Hardening: Run as a non-privileged user to prevent container escape
RUN useradd -ms /bin/bash veriphysuser && \
    chown -R veriphysuser:veriphysuser /app
USER veriphysuser

# Launch the VeriPhys Core Engine
CMD ["./veriphys-engine"]
