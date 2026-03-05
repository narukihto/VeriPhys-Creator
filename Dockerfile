# --- Stage 1: Plan Stage ---
# Using cargo-chef to handle dependency caching more reliably
FROM rust:1.80-slim-bookworm as planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-json recipe.json

# --- Stage 2: Build Stage ---
FROM rust:1.80-slim-bookworm as builder
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json

# Install system dependencies for blockchain-grade cryptography
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Build dependencies (this layer is cached unless Cargo.toml/lock changes)
# The patch in your Cargo.toml for ring 0.17.13 is applied here
RUN cargo chef cook --release --recipe-json recipe.json

# Build the actual application
COPY . .
# Ensuring the ABI file is present for abigen! macro
RUN if [ ! -f "IntegrityLedger.json" ]; then echo "{}" > IntegrityLedger.json; fi 
RUN cargo build --release

# --- Stage 3: Runtime Stage ---
# Using the latest stable Debian for 2026 (Bookworm 12.13)
FROM debian:bookworm-slim

WORKDIR /app

# Security: Install only necessary certificates and openssl
RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary and necessary runtime assets
COPY --from=builder /app/target/release/veriphys-protocol-core ./veriphys-engine
COPY --from=builder /app/IntegrityLedger.json ./IntegrityLedger.json
COPY --from=builder /app/registry.txt ./registry.txt 2>/dev/null || touch registry.txt

# Networking and Environment Settings
EXPOSE 3000
ENV RUST_LOG=info
ENV SERVER_PORT=3000

# Security: Run as a non-privileged user
RUN useradd -ms /bin/bash veriphysuser
USER veriphysuser

# Start the Engine
CMD ["./veriphys-engine"]
