# Stage 1: Build embed-proxy binaries
FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev perl make && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build both the server and migration binaries
RUN cargo build --release -p embed-proxy-http

# Stage 2: Minimal runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/embed-proxy /usr/local/bin/embed-proxy
COPY --from=builder /app/target/release/embed-proxy-migrate /usr/local/bin/embed-proxy-migrate

EXPOSE 8080

CMD ["embed-proxy"]
