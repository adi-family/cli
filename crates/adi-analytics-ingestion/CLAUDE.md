adi-analytics-ingestion, rust, axum, timescaledb, analytics, events

## Overview
- Analytics event ingestion service for ADI platform
- Receives events via HTTP and writes to TimescaleDB
- Built with Axum + SQLx + TimescaleDB
- Batches events for efficient database writes

## Architecture
```
┌─────────────────┐     ┌──────────────────────┐     ┌─────────────────┐
│  All Services   │────▶│  Analytics           │────▶│  PostgreSQL     │
│ (lib-analytics- │ HTTP│  Ingestion Service   │     │  + TimescaleDB  │
│     -core)      │     │  (this service)      │     │                 │
└─────────────────┘     └──────────────────────┘     └─────────────────┘
```

## API Endpoints

### Health
- `GET /health` - Health check endpoint (returns `{"status": "ok"}`)

### Event Ingestion
- `POST /events/batch` - Receive and persist a batch of analytics events
  - Request body: `Vec<EnrichedEvent>` (JSON array of events)
  - Response: `{"received": <count>}` (HTTP 200)
  - Returns 500 if database write fails

## Event Processing

Events are processed using a batching worker:
1. Receive events via HTTP POST
2. Validate event structure
3. Write to `analytics_events` hypertable in TimescaleDB
4. Return success/failure status

### Event Schema
Events contain:
- `event_id` (UUID)
- `event_type` (enum: TaskCreated, ApiRequest, etc.)
- `user_id` (UUID)
- `service` (string: which service emitted the event)
- `timestamp` (DateTime<Utc>)
- `data` (JSONB: event-specific data)

## Environment Variables
- `PORT` - Listen port (default: 8094)
- `DATABASE_URL` - PostgreSQL connection string (same as analytics API)
- `PLATFORM_DATABASE_URL` - Alternative to DATABASE_URL

## Building
```bash
cargo build --release
```

## Running
```bash
DATABASE_URL=postgres://... cargo run

# Or with specific port
PORT=8094 DATABASE_URL=postgres://... cargo run
```

## Event Sources

Services that send events to this ingestion service:
- **adi-platform-api**: API requests, tasks, integrations
- **adi-auth-http**: Authentication events
- **tarminal-signaling-server**: Cocoon connections
- **cocoon-manager**: Cocoon orchestration

All services use `lib-analytics-core::AnalyticsClient` to send events.

## Database Writes

### Batch Insertion
- Uses prepared statements with batch execution
- Efficient bulk inserts to TimescaleDB hypertable
- Automatic transaction handling

### Error Handling
- Returns 500 if database write fails
- Logs detailed error messages
- Clients can retry failed batches

## Performance

### Write Performance
- Handles high-throughput event streams
- Efficient batching reduces database round-trips
- TimescaleDB optimized for time-series inserts

### Scalability
- Stateless service (can scale horizontally)
- No in-memory state between requests
- Database handles all persistence

## Read-Only vs Write-Only

This service is **write-only**:
- ✅ Receives and writes analytics events
- ❌ No read/query endpoints (handled by adi-analytics-api)
- ✅ Safe to scale horizontally (multiple instances)
- ✅ No event processing logic (just persistence)

For querying analytics data, see `adi-analytics-api` documentation.
