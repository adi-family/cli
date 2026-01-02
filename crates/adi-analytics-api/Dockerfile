FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy analytics core library
COPY crates/lib-analytics-core ./crates/lib-analytics-core

# Copy analytics API
COPY crates/adi-analytics-api ./crates/adi-analytics-api

# Build
WORKDIR /build/crates/adi-analytics-api
RUN cargo build --release

# Runtime image
FROM alpine:latest

RUN apk add --no-cache ca-certificates

COPY --from=builder /build/crates/adi-analytics-api/target/release/adi-analytics-api /usr/local/bin/

EXPOSE 8093

CMD ["adi-analytics-api"]
