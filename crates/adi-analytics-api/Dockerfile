FROM rust:alpine AS builder

WORKDIR /build

# Copy dependency first (at same level as service for relative path to work)
COPY lib-analytics-core ./lib-analytics-core

# Copy analytics API
COPY adi-analytics-api ./adi-analytics-api

WORKDIR /build/adi-analytics-api
RUN apk add --no-cache musl-dev && cargo build --release

FROM alpine:latest

RUN apk add --no-cache ca-certificates curl

COPY --from=builder /build/adi-analytics-api/target/release/adi-analytics-api /usr/local/bin/

EXPOSE 8093

CMD ["adi-analytics-api"]
