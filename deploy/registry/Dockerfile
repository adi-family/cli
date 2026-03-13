# Stage 1: Build registry server binary
FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev perl make && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build standalone registry server binary
RUN cargo build --release -p adi-registry --bin adi-registry-server

# Stage 2: Minimal runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/adi-registry-server /usr/local/bin/adi-registry-server

ENV REGISTRY_DATA_DIR=/data
EXPOSE 8080

CMD ["adi-registry-server", "--port", "8080"]
