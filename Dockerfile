# --- Stage 1: Plan Stage ---
# Changed version to 1.84 to support Edition 2024 and newer crates
FROM rust:1.84-slim-bookworm AS planner
WORKDIR /app
RUN cargo install cargo-chef
COPY Cargo.toml Cargo.lock ./
COPY . .
RUN cargo chef prepare --recipe-json recipe.json

# --- Stage 2: Build Stage ---
FROM rust:1.84-slim-bookworm AS builder
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies
RUN cargo chef cook --release --recipe-json recipe.json

# Final build with ABI and source code
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

COPY --from=builder /app/target/release/veriphys-protocol-core ./veriphys-engine
COPY --from=builder /app/IntegrityLedger.json ./IntegrityLedger.json

RUN touch registry.txt

EXPOSE 3000
ENV RUST_LOG=info
ENV SERVER_PORT=3000

RUN useradd -ms /bin/bash veriphysuser && \
    chown -R veriphysuser:veriphysuser /app
USER veriphysuser

CMD ["./veriphys-engine"]
