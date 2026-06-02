# ============================================================
# AetherOS — Multi-stage Production Dockerfile
#
# Stage 1 (builder): Rust + cargo build --release
# Stage 2 (runtime): Minimal debian-slim image
#
# Image boyutu: ~50-80MB (strip + slim base)
# ============================================================

# ── Stage 1: Builder ─────────────────────────────────────
FROM rust:1.87-slim-bookworm AS builder

WORKDIR /build

# Bağımlılıkları önce kopyala — layer cache optimizasyonu
# Cargo.toml değişmeden src/ değişirse bağımlılıklar
# yeniden derlenmez.
COPY Cargo.toml Cargo.lock ./

# Dummy main ile bağımlılıkları derle
RUN mkdir src && \
    echo 'fn main() {}' > src/main.rs && \
    echo 'pub fn dummy() {}' > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# Gerçek kaynak kodu
COPY src ./src

# Binary'yi derle (strip ile küçült)
RUN cargo build --release && \
    strip target/release/aetheros

# ── Stage 2: Runtime ─────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Güvenlik güncellemeleri + gerekli kütüphaneler
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Non-root kullanıcı
RUN useradd -r -s /bin/false -u 1001 aetheros

WORKDIR /app

# Binary kopyala
COPY --from=builder /build/target/release/aetheros .

# DB ve log dizinleri
RUN mkdir -p /data/db /data/logs && \
    chown -R aetheros:aetheros /app /data

USER aetheros

# ── Ortam değişkeni varsayılanları ───────────────────────
ENV AETHEROS_ADDR=0.0.0.0:8080 \
    AETHEROS_DB_PATH=/data/db/aetheros.db \
    AETHEROS_WORKERS=4 \
    AETHEROS_MAX_CONCURRENT=256 \
    AETHEROS_SHUTDOWN_TIMEOUT=30 \
    AETHEROS_JSON_LOGS=true \
    RUST_LOG=info

EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:8080/health || exit 1

ENTRYPOINT ["./aetheros"]
