# ── Build stage ──────────────────────────────────────────────────────────────
FROM rust:1.78-slim AS builder

WORKDIR /build

# Install system dependencies for SQLite and OpenSSL
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first (layer-cache dependencies)
COPY Cargo.toml Cargo.lock ./
COPY crates/axiom-core/Cargo.toml   crates/axiom-core/
COPY crates/axiom-server/Cargo.toml crates/axiom-server/
COPY crates/axiom-cli/Cargo.toml    crates/axiom-cli/

# Stub out source files to cache dependency compilation
RUN mkdir -p crates/axiom-core/src   && echo "pub fn stub(){}" > crates/axiom-core/src/lib.rs
RUN mkdir -p crates/axiom-server/src && echo "fn main(){}"     > crates/axiom-server/src/main.rs
RUN mkdir -p crates/axiom-cli/src    && echo "fn main(){}"     > crates/axiom-cli/src/main.rs
RUN cargo build --release --bin axiom-server --bin axiom 2>/dev/null || true

# Copy real source code
COPY crates/ crates/
COPY schema/  schema/

# Build for real
RUN cargo build --release --bin axiom-server --bin axiom

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Non-root user
RUN useradd -m -u 1000 axiom

COPY --from=builder /build/target/release/axiom-server /usr/local/bin/axiom-server
COPY --from=builder /build/target/release/axiom        /usr/local/bin/axiom

RUN mkdir -p /data /data/dead-letter && chown -R axiom:axiom /data

USER axiom
WORKDIR /data

ENV AXIOM_STORAGE_BACKEND=sqlite
ENV AXIOM_STORAGE_PATH=/data/axiom.db
ENV AXIOM_DEAD_LETTER_PATH=/data/dead-letter
ENV AXIOM_HOST=0.0.0.0
ENV AXIOM_PORT=8080

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/axiom-server", "--health-check"] || exit 1

ENTRYPOINT ["/usr/local/bin/axiom-server"]
