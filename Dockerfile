# --- Stage 1: Plan Stage ---
FROM rustlang/rust:nightly-bookworm-slim AS planner
WORKDIR /app
RUN cargo install cargo-chef --version 0.1.67
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# الصياغة الصحيحة للنسخة 0.1.67 هي --recipe-path
RUN cargo chef prepare --recipe-path recipe.json

# --- Stage 2: Build Stage ---
FROM rustlang/rust:nightly-bookworm-slim AS builder
WORKDIR /app
RUN cargo install cargo-chef --version 0.1.67
COPY --from=planner /app/recipe.json recipe.json

# تثبيت مكتبات النظام اللازمة
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# بناء المكتبات الخارجية (Caching)
RUN cargo chef cook --release --recipe-path recipe.json

# بناء التطبيق النهائي
COPY IntegrityLedger.json . 
COPY . .
RUN cargo build --release

# --- Stage 3: Runtime Stage ---
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/veriphys-protocol-core ./veriphys-engine
COPY --from=builder /app/IntegrityLedger.json ./IntegrityLedger.json

RUN touch registry.txt
RUN useradd -ms /bin/bash veriphysuser && chown -R veriphysuser:veriphysuser /app
USER veriphysuser

EXPOSE 3000
CMD ["./veriphys-engine"]
