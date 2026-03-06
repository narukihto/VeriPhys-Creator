# --- Stage 1: Plan Stage ---
# We use Nightly to support Edition 2024 required by modern dependencies
FROM rustlang/rust:nightly-bookworm-slim AS planner
WORKDIR /app
# Install a specific version of cargo-chef to avoid build breakages
RUN cargo install cargo-chef --version 0.1.67
COPY Cargo.toml Cargo.lock ./
COPY . .
RUN cargo chef prepare --recipe-json recipe.json

# --- Stage 2: Build Stage ---
FROM rustlang/rust:nightly-bookworm-slim AS builder
WORKDIR /app
RUN cargo install cargo-chef --version 0.1.67
COPY --from=planner /app/recipe.json recipe.json

# Install essential system dependencies for encryption
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies independently
RUN cargo chef cook --release --recipe-json recipe.json

# Build the actual project
COPY IntegrityLedger.json . 
COPY . .
RUN cargo build --release

# --- Stage 3: Runtime Stage ---
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/veriphys-protocol-core ./veriphys-engine
COPY --from=builder /app/IntegrityLedger.json ./IntegrityLedger.json

# Initialize registry file
RUN touch registry.txt

EXPOSE 3000
ENV RUST_LOG=info

# Create non-root user for security
RUN useradd -ms /bin/bash veriphysuser && \
    chown -R veriphysuser:veriphysuser /app
USER veriphysuser

CMD ["./veriphys-engine"]
