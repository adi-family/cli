FROM rust:1.83-slim as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/adi-auth-core crates/adi-auth-core
COPY crates/adi-auth-http crates/adi-auth-http
COPY crates/lib-migrations crates/lib-migrations

# Build release binary
RUN cargo build --release -p adi-auth-http

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/adi-auth-http /usr/local/bin/

# Create data directory for SQLite
RUN mkdir -p /data
ENV AUTH_DB_PATH=/data/auth.sqlite

ENV PORT=8090
EXPOSE 8090

CMD ["adi-auth-http"]
