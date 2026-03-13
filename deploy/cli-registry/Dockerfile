# Stage 1: Build CLI registry server binary
FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev perl make && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release -p adi-cli-registry-server --bin adi-cli-registry-server

# Stage 2: Minimal runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/adi-cli-registry-server /usr/local/bin/adi-cli-registry-server

ENV REGISTRY_DATA_DIR=/data
EXPOSE 8080

CMD ["adi-cli-registry-server", "--port", "8080"]
