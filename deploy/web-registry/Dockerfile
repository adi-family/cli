# Stage 1: Build Web registry server binary
FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev perl make && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release -p adi-web-registry-server --bin adi-web-registry-server

# Stage 2: Minimal runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/adi-web-registry-server /usr/local/bin/adi-web-registry-server

ENV REGISTRY_DATA_DIR=/data
EXPOSE 8080

CMD ["adi-web-registry-server", "--port", "8080"]
