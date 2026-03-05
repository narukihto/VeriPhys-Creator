# --- Stage 1: Build Stage ---
FROM rust:1.75-slim-bookworm as builder

WORKDIR /app

# Install system dependencies for building (SSL and C compilers)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# 1. Pre-build dependencies to cache them (Optimization)
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/veriphys*

# 2. Copy actual source and build the real binary
COPY . .
RUN cargo build --release

# --- Stage 2: Runtime Stage ---
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (OpenSSL)
RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/veriphys-protocol .
# Copy the registry file if it exists
COPY --from=builder /app/registry.txt ./registry.txt 2>/dev/null || touch registry.txt

# Expose the API port
EXPOSE 3000

# Set environment to production
ENV RUST_LOG=info

# Launch the protocol engine
CMD ["./veriphys-protocol"]
