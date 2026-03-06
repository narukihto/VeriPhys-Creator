# --- Stage 1: Plan Stage ---
FROM rustlang/rust:nightly-bookworm-slim AS planner
WORKDIR /app
# تثبيت نسخة مستقرة ومعروفة من cargo-chef
RUN cargo install cargo-chef --version 0.1.67
# نسخ ملفات التوصيف فقط أولاً لضمان عزل الاعتمادات
COPY Cargo.toml Cargo.lock ./
# نسخ الكود المصدري (مهم جداً لعملية prepare)
COPY src ./src
# تنفيذ التخطيط مع تجاهل الملفات غير الضرورية
RUN cargo chef prepare --recipe-json recipe.json

# --- Stage 2: Build Stage ---
FROM rustlang/rust:nightly-bookworm-slim AS builder
WORKDIR /app
RUN cargo install cargo-chef --version 0.1.67
COPY --from=planner /app/recipe.json recipe.json

# تثبيت مكتبات النظام اللازمة للربط (Linking)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# بناء المكتبات الخارجية فقط (Caching Layer)
RUN cargo chef cook --release --recipe-json recipe.json

# الآن نسخ باقي ملفات المشروع لبناء التطبيق النهائي
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
RUN useradd -ms /bin/bash veriphysuser && chown -R veriphysuser:veriphysuser /app
USER veriphysuser

EXPOSE 3000
CMD ["./veriphys-engine"]
