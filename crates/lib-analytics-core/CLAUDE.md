lib-analytics-core, analytics, timescaledb, event-tracking, migrations

## Overview
- Core analytics library for ADI platform
- Event tracking with batching and TimescaleDB persistence
- Non-blocking event collection via async channels
- Owns analytics database schema and migrations

## Components

### Library (lib-analytics-core)
Event tracking library used by all services:
- **AnalyticsEvent**: Enum of all trackable events
- **AnalyticsClient**: Non-blocking event tracking
- **AnalyticsWorker**: Background worker for bulk database inserts
- **EnrichedEvent**: Events with timestamp and metadata

### Binary (analytics-migrate)
Migration runner for analytics database schema:
- Creates TimescaleDB hypertable for events
- Creates continuous aggregates for fast queries
- Manages compression and retention policies

## Usage

### Event Tracking (in services)
```rust
use lib_analytics_core::{AnalyticsClient, AnalyticsWorker, AnalyticsEvent};

// Initialize (in main)
let (analytics_client, worker_config) = AnalyticsClient::new(
    100,  // batch_size
    10,   // flush_interval_secs
);
let analytics_worker = AnalyticsWorker::new(worker_config, pool.clone());

// Spawn worker
tokio::spawn(async move {
    analytics_worker.run().await;
});

// Track events
analytics_client.track(AnalyticsEvent::TaskCreated {
    task_id: task.id,
    user_id: user.id,
    project_id: Some(project_id),
    cocoon_id: task.cocoon_id,
    command: task.command.clone(),
});
```

### Running Migrations
```bash
# Run all migrations
cargo run --bin analytics-migrate --features migrate all

# Check migration status
cargo run --bin analytics-migrate --features migrate status

# Dry run (preview pending migrations)
cargo run --bin analytics-migrate --features migrate dry-run

# Pre-deploy only (creates tables)
cargo run --bin analytics-migrate --features migrate pre

# Post-deploy only (creates aggregates)
cargo run --bin analytics-migrate --features migrate post
```

## Event Types

### Authentication
- `AuthLoginAttempt` - User login attempt (success/failure)
- `AuthCodeVerified` - Login code verification
- `AuthTokenRefresh` - Token refresh attempt
- `AuthSessionValidated` - Session validation check

### Tasks
- `TaskCreated` - Task created
- `TaskStarted` - Task execution started
- `TaskCompleted` - Task finished successfully
- `TaskFailed` - Task execution failed
- `TaskCancelled` - Task cancelled by user

### Integrations
- `IntegrationConnected` - Integration connected
- `IntegrationDisconnected` - Integration disconnected
- `IntegrationUsed` - Integration action performed
- `IntegrationError` - Integration error occurred
- `OAuthFlowStarted` - OAuth flow initiated
- `OAuthFlowCompleted` - OAuth flow finished

### Webhooks
- `WebhookReceived` - Webhook received from external service
- `WebhookProcessed` - Webhook processing completed

### Cocoons
- `CocoonRegistered` - Cocoon registered
- `CocoonConnected` - Cocoon connected to signaling server
- `CocoonDisconnected` - Cocoon disconnected
- `CocoonClaimed` - Cocoon claimed by user
- `CocoonSetupTokenCreated` - Setup token generated
- `CocoonSetupTokenUsed` - Setup token redeemed

### Projects
- `ProjectCreated` - Project created
- `ProjectUpdated` - Project updated
- `ProjectDeleted` - Project deleted

### System
- `ApiRequest` - HTTP API request (with latency, status code)
- `DatabaseQuery` - Database query executed
- `ApplicationError` - Application error occurred

## Database Schema

### analytics_events (TimescaleDB Hypertable)
```sql
CREATE TABLE analytics_events (
    id BIGSERIAL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type VARCHAR(100) NOT NULL,
    service VARCHAR(100) NOT NULL,
    user_id UUID,
    data JSONB NOT NULL,  -- Full event data
    PRIMARY KEY (timestamp, id)
);

-- Convert to hypertable (time-series partitioning)
SELECT create_hypertable('analytics_events', 'timestamp');
```

**Features:**
- Automatic partitioning by day
- Compression after 7 days (~90% space savings)
- 90-day retention policy (auto-delete old data)
- Indexed for fast queries

### Continuous Aggregates
Auto-updating materialized views:
1. `analytics_daily_active_users` - DAU/WAU/MAU
2. `analytics_task_stats_daily` - Task metrics by day
3. `analytics_api_latency_hourly` - API performance (p50/p95/p99)
4. `analytics_integration_health_daily` - Integration stats
5. `analytics_auth_events_daily` - Auth metrics
6. `analytics_cocoon_activity_daily` - Cocoon usage
7. `analytics_errors_hourly` - Error tracking

Refresh policy: Every hour for last 3 days

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Services (Platform, Auth, Signaling, etc.)    │
│  - Use AnalyticsClient                          │
│  - Track events (non-blocking)                  │
└───────────────┬─────────────────────────────────┘
                │ async channel
                ▼
┌─────────────────────────────────────────────────┐
│  AnalyticsWorker (background task)              │
│  - Batches events (100 or 10s)                  │
│  - Bulk INSERT to database                      │
└───────────────┬─────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────────────┐
│  PostgreSQL + TimescaleDB                       │
│  - analytics_events (hypertable)                │
│  - Continuous aggregates (auto-update)          │
└─────────────────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────────────┐
│  adi-analytics-api (read-only)                  │
│  - Queries aggregates                           │
│  - Provides HTTP endpoints                      │
└─────────────────────────────────────────────────┘
```

## Performance

### Non-Blocking Design
- Events sent via unbounded async channel
- Worker batches 100 events or 10 seconds
- No impact on API response times
- Graceful degradation if worker fails

### Database Optimization
- TimescaleDB hypertable (optimized for time-series)
- Automatic compression (~90% space savings)
- Continuous aggregates (pre-computed rollups)
- Smart retention (90d raw, unlimited aggregates)

### Scalability
- Handles billions of events
- Sub-millisecond event tracking
- Sub-second aggregate queries
- Automatic data lifecycle management

## Migration Files

Located in `migrations/`:

**001_create_analytics_events.sql:**
- Creates analytics_events table
- Converts to TimescaleDB hypertable
- Adds indexes (user_id, event_type, service)
- Configures compression and retention

**002_create_analytics_aggregates.sql:**
- Creates 7 continuous aggregates
- Adds refresh policies (hourly)
- Adds indexes for fast dashboard queries

## Environment Variables

**For analytics-migrate binary:**
- `DATABASE_URL` - PostgreSQL connection string
- `PLATFORM_DATABASE_URL` - Alternative to DATABASE_URL

**For services using the library:**
- No special env vars needed
- Just initialize client and worker with database pool

## Building

```bash
# Library only
cargo build --release

# With migration binary
cargo build --release --features migrate --bin analytics-migrate
```

## Integration

### Platform API
```rust
// In adi-platform-api/src/main.rs
use lib_analytics_core::{AnalyticsClient, AnalyticsWorker};

let (analytics_client, worker_config) = AnalyticsClient::new(100, 10);
let analytics_worker = AnalyticsWorker::new(worker_config, pool.clone());
tokio::spawn(async move { analytics_worker.run().await });

// Add to AppState
AppState { analytics: analytics_client, ... }

// In handlers
state.analytics.track(AnalyticsEvent::TaskCreated { ... });
```

### Other Services
Same pattern - initialize client, spawn worker, track events.

## Design Principles

✅ **Separation of Concerns**: Core owns data model, API queries it
✅ **Non-Blocking**: Event tracking never blocks business logic
✅ **Batching**: Efficient bulk inserts reduce DB load
✅ **Type-Safe**: Enum ensures valid event structure
✅ **Scalable**: TimescaleDB handles billions of events
✅ **Cost-Effective**: Compression + retention manage storage

## Why Migrations Live Here

The migrations are in `lib-analytics-core` because:
1. Core defines `AnalyticsEvent` enum → schema must match
2. Core contains `AnalyticsWorker` → writes to database
3. Core owns the data model → owns the schema
4. API is read-only → doesn't own the structure
5. Multiple services use core → any can run migrations

The binary ensures migrations can be run independently without requiring a full service deployment.
