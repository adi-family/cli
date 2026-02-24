analytics, timescaledb, event-tracking, http-api, ingestion

## Structure
- `core/` - Core types: `AnalyticsEvent` enum, `EnrichedEvent`, `AnalyticsError`, migrations
- `client/` - `AnalyticsClient` (HTTP sender), `EventWriter` (DB writer), query models, read queries
- `plugin/` - Two HTTP binaries: `analytics-http` (read API) and `analytics-ingestion` (write API)

## Event Flow
- Services create `AnalyticsClient::new(ingestion_url)` and call `.track(event)`
- Client batches events (100 or 10s) and POSTs to ingestion service
- `analytics-ingestion` receives batches via `/events/batch` and bulk-inserts to TimescaleDB
- `analytics-http` serves read-only queries against continuous aggregates

## Database
- `analytics_events` TimescaleDB hypertable (partitioned by day, compressed after 7d, retained 90d)
- 7 continuous aggregates (DAU, task stats, API latency, integration health, auth, cocoon, errors)

## Migrations
```bash
cargo run -p analytics-core --features migrate --bin analytics-migrate all
```

## Building
```bash
cargo build -p analytics-plugin --bin analytics-http
cargo build -p analytics-plugin --bin analytics-ingestion
```

## Environment Variables
- `DATABASE_URL` / `PLATFORM_DATABASE_URL` - PostgreSQL connection
- `PORT` - Listen port (8093 for read API, 8094 for ingestion)
- `ANALYTICS_URL` - Ingestion service URL (used by other services' AnalyticsClient)

## Usage in Other Services
```rust
use analytics_client::{AnalyticsClient, AnalyticsEvent};

let client = AnalyticsClient::new("http://localhost:8094");
client.track(AnalyticsEvent::TaskCreated { ... });
```
