# --- Stage 1: Plan Stage ---
FROM rust:1.80-slim-bookworm as planner
WORKDIR /app
RUN cargo install cargo-chef
# We need both the manifest and the lockfile for a deterministic recipe
COPY Cargo.toml Cargo.lock ./
COPY . .
RUN cargo chef prepare --recipe-json recipe.json

# --- Stage 2: Build Stage ---
FROM rust:1.80-slim-bookworm as builder
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json

# Install system dependencies for SHA3 and SSL
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Layer Caching: Build dependencies only
RUN cargo chef cook --release --recipe-json recipe.json

# --- CRITICAL FIX START ---
# We must copy the ABI file BEFORE building, because 'abigen!' 
# looks for this file at compile time.
COPY IntegrityLedger.json . 

# Copy the actual source code
COPY src ./src
COPY Cargo.toml Cargo.lock ./

# Final Compilation
RUN cargo build --release
# --- CRITICAL FIX END ---

# --- Stage 3: Runtime Stage ---
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime certificates for blockchain RPC (SSL/TLS)
RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary and required assets from builder
COPY --from=builder /app/target/release/veriphys-protocol-core ./veriphys-engine
COPY --from=builder /app/IntegrityLedger.json ./IntegrityLedger.json

# Ensure the registry file exists for the audit trail
RUN touch registry.txt

# Networking
EXPOSE 3000
ENV RUST_LOG=info
ENV SERVER_PORT=3000

# Security: Run as non-root user
RUN useradd -ms /bin/bash veriphysuser && \
    chown -R veriphysuser:veriphysuser /app
USER veriphysuser

CMD ["./veriphys-engine"]
