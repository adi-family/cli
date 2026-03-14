# Stage 1: Build auth binaries
FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev perl make && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build both the server and migration binaries
RUN cargo build --release -p auth-http

# Stage 2: Minimal runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/auth-server /usr/local/bin/auth-server
COPY --from=builder /app/target/release/auth-migrate /usr/local/bin/auth-migrate

EXPOSE 8080

CMD ["auth-server"]
