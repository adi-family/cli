FROM rust:1.83-slim-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    perl \
    libfindbin-libs-perl \
    make \
    && rm -rf /var/lib/apt/lists/*

# Copy only what we need for the plugin registry
COPY Cargo.toml Cargo.lock ./

# Copy required crates
COPY crates/lib-plugin-manifest crates/lib-plugin-manifest
COPY crates/lib-plugin-registry crates/lib-plugin-registry
COPY crates/adi-plugin-registry-http crates/adi-plugin-registry-http

# Create minimal Cargo.toml for building just the registry
RUN echo '[workspace]\n\
resolver = "2"\n\
members = [\n\
    "crates/lib-plugin-manifest",\n\
    "crates/lib-plugin-registry",\n\
    "crates/adi-plugin-registry-http",\n\
]\n\
\n\
[workspace.package]\n\
version = "0.8.4"\n\
edition = "2021"\n\
license = "BSL-1.0"\n\
authors = ["ADI Team"]\n\
\n\
[workspace.dependencies]\n\
lib-plugin-manifest = { path = "crates/lib-plugin-manifest" }\n\
lib-plugin-registry = { path = "crates/lib-plugin-registry" }\n\
thiserror = "2"\n\
anyhow = "1.0"\n\
tokio = { version = "1.0", features = ["full"] }\n\
serde = { version = "1.0", features = ["derive"] }\n\
serde_json = "1.0"\n\
toml = "0.8"\n\
reqwest = { version = "0.12", features = ["json", "native-tls-vendored", "stream"] }\n\
futures = "0.3"\n\
semver = "1"\n\
axum = "0.7"\n\
tower = "0.4"\n\
tower-http = { version = "0.5", features = ["cors", "trace"] }\n\
tracing = "0.1"\n\
tracing-subscriber = { version = "0.3", features = ["env-filter"] }\n\
sha2 = "0.10"\n\
hex = "0.4"\n\
chrono = { version = "0.4", features = ["serde"] }\n\
flate2 = "1"\n\
tar = "0.4"\n\
' > Cargo.toml

# Build release binary
RUN cargo build --release -p adi-plugin-registry-http

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false registry

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/adi-plugin-registry /app/adi-plugin-registry

# Create data directory
RUN mkdir -p /data && chown registry:registry /data

USER registry

ENV REGISTRY_DATA_DIR=/data
ENV PORT=8080
ENV RUST_LOG=info

EXPOSE 8080

VOLUME ["/data"]

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/app/adi-plugin-registry"]
