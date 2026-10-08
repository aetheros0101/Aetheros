# syntax=docker/dockerfile:1.7
# ============================================================
# AetherOS — Multi-stage Production Dockerfile
#
# Stage 1 (builder): Rust + cargo build --release
# Stage 2 (runtime): Minimal debian-slim image
#
# Not: workspace path bağımlılıkları (aetheros-terminal, workspace_core)
# ve benches/ build context'ine dahil edilmelidir; aksi halde Cargo
# manifest'i çözemez. .dockerignore bunları dışarıda BIRAKMAZ.
# ============================================================

# Rust sürümü build arg: `docker build --build-arg RUST_VERSION=1.90 .`
# Varsayılan "1" = en güncel kararlı sürüm (wasmtime MSRV'sini karşılar).
ARG RUST_VERSION=1

# ── Stage 1: Builder ─────────────────────────────────────
FROM rust:${RUST_VERSION}-slim-bookworm AS builder

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY aetheros-terminal ./aetheros-terminal
COPY workspace_core ./workspace_core
COPY benches ./benches
COPY src ./src

# BuildKit cache mount'ları: registry ve target katmanları build'ler
# arası korunur (dummy-main hilesinden daha sağlam; path crate'lerle de çalışır).
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked --bin aetheros && \
    mkdir -p /out && \
    cp target/release/aetheros /out/aetheros && \
    strip /out/aetheros

# ── Stage 2: Runtime ─────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# reqwest rustls-tls kullanıyor → libssl gerekmez.
# curl: HEALTHCHECK için (slim imajda wget/curl yok).
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        curl && \
    rm -rf /var/lib/apt/lists/*

# Non-root kullanıcı
RUN useradd -r -s /bin/false -u 1001 aetheros

WORKDIR /app

COPY --from=builder /out/aetheros .

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

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fsS http://localhost:8080/health || exit 1

ENTRYPOINT ["./aetheros"]
