# syntax=docker/dockerfile:1.7
# ────────────────────────────────────────────────────────────────────────────
# Dockerfile — Ứng Dụng Từ Bi (v0.9)
# Multi-stage build với Rust 1.97.1
#
# Image final: ~30 MB (glibc + stripped binary + static assets)
# ────────────────────────────────────────────────────────────────────────────

# ── Stage 1: Builder ─────────────────────────────────────────────────────────
FROM rust:1.97.1-slim-bookworm AS builder

# Cài các dependency hệ thống cần thiết cho build:
#   - pkg-config + libssl-dev: cho rustls/openssl
#   - libpq-dev: cho sqlx-postgres
#   - ca-certificates: để cargo tải crates
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        libpq-dev \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependencies: copy Cargo.toml/Cargo.lock trước, tạo dummy main.rs để build deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && \
    echo 'fn main() { println!("dummy"); }' > src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy source thật + templates + migrations + static assets
COPY src/ ./src/
COPY templates/ ./templates/
COPY migrations/ ./migrations/

# ⚠️ Force cargo to recompile src/ (docker COPY có thể preserve old mtime,
# khiến cargo nghĩ src/main.rs không đổi từ bản dummy → binary cuối là "dummy").
# Touch tất cả .rs để cập nhật mtime, buộc cargo recompile main crate.
RUN find src/ -name "*.rs" -exec touch {} +

# Build binary release (LTO + strip đã config trong Cargo.toml)
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin ungdungtubi && \
    cp target/release/ungdungtubi /build/ungdungtubi && \
    test -f /build/ungdungtubi && \
    # Verify binary không phải dummy: tìm string "axum" (web framework thật).
    # `strings` mặc định chỉ show ASCII, không thấy Vietnamese (UTF-8) nên
    # không thể grep "Ứng Dụng Từ Bi" — dùng "axum" thay thế.
    strings /build/ungdungtubi | grep -q "axum" || \
    (echo "ERROR: binary không chứa 'axum' — có vẻ là dummy binary!" && exit 1)

# ── Stage 2: Runtime ────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Cài runtime libraries:
#   - libssl3, libpq5: runtime cho rustls + sqlx-postgres
#   - ca-certificates: để reqwest gọi Google OAuth qua HTTPS
#   - curl + wget: cho healthcheck (Coolify có thể dùng cái nào có sẵn)
#   - tini: PID 1 signal handler
RUN apt-get update && apt-get install -y --no-install-recommends \
        libssl3 \
        libpq5 \
        ca-certificates \
        curl \
        wget \
        tini \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 1001 tubi \
    && useradd  --system --uid 1001 --gid tubi --home-dir /app --shell /sbin/nologin tubi

WORKDIR /app

# Copy binary từ builder stage
COPY --from=builder /build/ungdungtubi /app/ungdungtubi

# Copy static assets (CSS/JS) + templates (đã compile vào binary, copy cho chắc) + migrations
COPY src/static/ /app/static/
COPY migrations/ /app/migrations/

# Tạo thư mục uploads và chown cho user tubi
RUN mkdir -p /app/static/uploads && \
    chown -R tubi:tubi /app

# Cấu hình env mặc định (có thể override bằng Coolify env vars)
ENV APP_ENV=production \
    HOST=0.0.0.0 \
    PORT=8080 \
    STATIC_DIR=/app/static \
    UPLOAD_DIR=/app/static/uploads \
    UPLOAD_URL_PREFIX=/static/uploads \
    MAX_UPLOAD_BYTES=5242880 \
    RUST_LOG=ungdungtubi=info,axum=info,tower_http=info,sqlx=warn \
    RUST_BACKTRACE=0

USER tubi

EXPOSE 8080

# Healthcheck: gọi /api/health, kỳ vọng HTTP 200
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fsS http://localhost:8080/api/health || exit 1

# Dùng tini làm PID 1 để xử lý signals đúng (graceful shutdown)
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/app/ungdungtubi"]
