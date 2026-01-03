FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy analytics core library
COPY crates/lib-analytics-core ./crates/lib-analytics-core

# Copy analytics ingestion
COPY crates/adi-analytics-ingestion ./crates/adi-analytics-ingestion

# Build
WORKDIR /build/crates/adi-analytics-ingestion
RUN cargo build --release

# Runtime image
FROM alpine:latest

RUN apk add --no-cache ca-certificates curl

COPY --from=builder /build/crates/adi-analytics-ingestion/target/release/adi-analytics-ingestion /usr/local/bin/

EXPOSE 8094

CMD ["adi-analytics-ingestion"]
