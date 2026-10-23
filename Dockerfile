# syntax=docker/dockerfile:1.7
# Multi-stage build: frontend (Vite) → backend (Rust, release) → small runtime image.

FROM node:22-alpine AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY frontend/ ./
RUN npm run build

FROM rust:1.90-slim-bookworm AS backend
WORKDIR /app/backend
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config && rm -rf /var/lib/apt/lists/*
# Cache dependencies separately from application code.
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
    && cargo build --release --locked \
    && rm -rf src target/release/deps/opensyllabus* target/release/deps/libopensyllabus* target/release/.fingerprint/opensyllabus-* target/release/opensyllabus*
COPY backend/ ./
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tzdata && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 opensyllabus \
    && useradd --system --uid 10001 --gid opensyllabus --home /data --shell /usr/sbin/nologin opensyllabus \
    && mkdir -p /data/db /data/storage /data/backups && chown -R opensyllabus:opensyllabus /data
COPY --from=backend /app/backend/target/release/opensyllabus /usr/local/bin/opensyllabus
COPY --from=frontend /app/frontend/dist /app/public
ENV BIND_ADDR=0.0.0.0:8080 \
    DATABASE_PATH=/data/db/opensyllabus.db \
    STORAGE_DIR=/data/storage \
    STATIC_DIR=/app/public \
    RUST_LOG=info,tower_http=info,sqlx=warn
USER opensyllabus
WORKDIR /data
VOLUME ["/data"]
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 CMD ["opensyllabus", "healthcheck"]
STOPSIGNAL SIGTERM
ENTRYPOINT ["opensyllabus"]
CMD ["serve"]
