adi-analytics-api, rust, axum, timescaledb, analytics, metrics

## Overview
- Analytics API for ADI platform
- Provides metrics, dashboards, and aggregated statistics
- Built with Axum + SQLx + TimescaleDB
- Reads from analytics events written by all services

## Architecture
```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  All Services   │────▶│  PostgreSQL      │◀────│  Analytics API  │
│  (track events) │     │  + TimescaleDB   │     │  (query only)   │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │ analytics_events  │ (hypertable)
                    │ - auto compression│
                    │ - 90d retention   │
                    └─────────┬─────────┘
                              │
                    ┌─────────┴─────────┐
                    │ 7 Aggregates      │ (continuous)
                    │ - DAU/WAU/MAU     │
                    │ - Task stats      │
                    │ - API latency     │
                    └───────────────────┘
```

## API Endpoints

### Health
- `GET /health` - Health check endpoint (returns `{"status": "ok"}`)

### Overview
- `GET /api/analytics/overview` - Dashboard summary (DAU/WAU/MAU, tasks, cocoons, integrations)

### Users
- `GET /api/analytics/users/daily` - Daily active users over time
- `GET /api/analytics/users/weekly` - Weekly active users rollup

### Tasks
- `GET /api/analytics/tasks/daily` - Task statistics by day (created, completed, failed, success rate)
- `GET /api/analytics/tasks/overview` - Summary (total tasks, success rate, avg duration)

### API Performance
- `GET /api/analytics/api/latency` - Endpoint latency stats (p50, p95, p99, error rates)
- `GET /api/analytics/api/slowest` - Top 10 slowest endpoints in last 24h

All endpoints support query parameters:
- `start_date` - ISO 8601 timestamp (default: 30 days ago)
- `end_date` - ISO 8601 timestamp (default: now)

## Database Migrations

Migrations are managed by `lib-analytics-core` (the library that owns the event schema):

```bash
# Run all migrations
cd ../lib-analytics-core
cargo run --bin analytics-migrate --features migrate all

# Check migration status
cargo run --bin analytics-migrate --features migrate status

# Dry run (show pending)
cargo run --bin analytics-migrate --features migrate dry-run
```

See `lib-analytics-core/migrations/` for migration files.

## Environment Variables
- `PORT` - Listen port (default: 8093)
- `DATABASE_URL` - PostgreSQL connection string (same as platform API)
- `PLATFORM_DATABASE_URL` - Alternative to DATABASE_URL

## Building
```bash
cargo build --release
```

## Running
```bash
DATABASE_URL=postgres://... cargo run

# Or with specific port
PORT=8093 DATABASE_URL=postgres://... cargo run
```

## Event Tracking

Analytics events are written by other services using `lib-analytics-core`:
- **adi-platform-api**: API requests, tasks, integrations
- **adi-auth-http**: Authentication events
- **tarminal-signaling-server**: Cocoon connections
- **cocoon-manager**: Cocoon orchestration

All events flow into `analytics_events` table, this API queries them.

## TimescaleDB Features

### Hypertable (Time-Series Partitioning)
- Automatic partitioning by day
- Chunks compressed after 7 days (~90% space savings)
- 90-day retention policy (old data auto-deleted)
- Optimized for time-range queries

### Continuous Aggregates
Auto-updating materialized views:
1. **analytics_daily_active_users** - DAU with total events
2. **analytics_task_stats_daily** - Task metrics (created, completed, failed, avg duration, p95)
3. **analytics_api_latency_hourly** - API performance (p50, p95, p99, error counts)
4. **analytics_integration_health_daily** - Integration stats (connections, errors, unique users)
5. **analytics_auth_events_daily** - Auth metrics (login attempts, success rate)
6. **analytics_cocoon_activity_daily** - Cocoon stats (connections, session duration)
7. **analytics_errors_hourly** - Error tracking (error counts, affected users)

Refresh policy: Every 1 hour for last 3 days

## Performance

### Query Performance
- Raw events: Indexed on user_id, event_type, service, timestamp
- GIN index on JSONB data column for flexible queries
- Continuous aggregates: Pre-computed rollups for instant dashboards
- Compression: Older data compressed but still queryable

### Scalability
- Handles billions of events
- Sub-second queries on aggregates
- Minimal storage growth (compression + retention)
- No impact on write performance (data ingestion)

## Example Response

```json
// GET /api/analytics/overview
{
  "total_users": 1234,
  "active_users_today": 56,
  "active_users_week": 234,
  "active_users_month": 567,
  "total_tasks": 5678,
  "tasks_today": 123,
  "task_success_rate": 0.95,
  "total_cocoons": 45,
  "active_cocoons": 12,
  "total_integrations": 89
}
```

## Read-Only Design

This service is **read-only**:
- ✅ Queries analytics_events and aggregates
- ✅ No writes to database
- ✅ No event ingestion (handled by lib-analytics-core in other services)
- ✅ Safe to scale horizontally

For event tracking, see `lib-analytics-core` documentation.
