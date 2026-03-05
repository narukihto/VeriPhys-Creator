# --- Stage 1: Build Stage ---
# Using the stable Slim Bookworm image for a smaller footprint
FROM rust:1.75-slim-bookworm as builder

WORKDIR /app

# Install system dependencies for blockchain-grade cryptography (OpenSSL & C Toolchains)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# 1. Dependency Caching: Builds dependencies separately to speed up future CI/CD runs
COPY Cargo.toml Cargo.lock ./
# Note: The patch for the 'ring' vulnerability is applied here via Cargo.toml
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/veriphys*

# 2. Final Build: Copy source code and the Solidity ABI file
COPY . .
# Ensuring the ABI file is present for abigen! macro
RUN if [ ! -f "IntegrityLedger.json" ]; then echo "{}" > IntegrityLedger.json; fi 
RUN cargo build --release

# --- Stage 2: Runtime Stage ---
# Minimal Debian image for production security
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime SSL certificates for secure Blockchain RPC communication
RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary and configuration files
COPY --from=builder /app/target/release/veriphys-protocol-core ./veriphys-engine
COPY --from=builder /app/IntegrityLedger.json ./IntegrityLedger.json
COPY --from=builder /app/registry.txt ./registry.txt 2>/dev/null || touch registry.txt

# Networking and Environment Settings
EXPOSE 3000
ENV RUST_LOG=info
ENV SERVER_PORT=3000

# Start the VeriPhys Infrastructure Engine
CMD ["./veriphys-engine"]
