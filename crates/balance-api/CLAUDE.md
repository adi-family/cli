adi-balance-api, rust, axum, postgres, balance, transactions, credits

## Overview
- Balance and transaction tracking service for ADI platform
- Manages AdiToken credits (internal USD analogue, 1 token = 1,000,000 microtokens)
- Standalone service that other services call as needed
- Stripe handles actual payments; this service tracks internal credit balances

## API Endpoints

### Balances
- `GET /balances/me` - Get current user's balance (JWT auth)
- `GET /balances/{user_id}` - Get balance by user ID (service auth)
- `POST /balances/init` - Create balance for user (JWT auth)

### Transactions
- `POST /transactions/deposit` - Add credits to balance (service auth)
- `POST /transactions/debit` - Deduct credits from balance (service auth)
- `POST /transactions/check` - Check if user has sufficient balance (service auth)
- `GET /transactions` - List user's transactions (JWT auth)
- `GET /transactions/{id}` - Get transaction details (JWT auth)

## Transaction Types
- `deposit` - Credits added (from Stripe purchase, promo, refund)
- `debit` - Credits consumed (API usage, task execution)
- `adjustment` - Manual admin adjustment
- `transfer_in` - Transfer received from another user
- `transfer_out` - Transfer sent to another user

## Key Features
- **Microtokens**: Amounts stored as BIGINT (1 AdiToken = 1,000,000 microtokens) for precision
- **Atomic Updates**: Database transactions with `FOR UPDATE` row locks
- **Optimistic Locking**: Version field prevents concurrent modification conflicts
- **Idempotency**: Unique index on (user_id, idempotency_key) prevents duplicate transactions
- **Audit Trail**: Every balance change creates a transaction record with before/after values

## Environment Variables
- `HOST` - Bind address (default: 0.0.0.0)
- `PORT` - Listen port (default: 8030)
- `DATABASE_URL` - PostgreSQL connection string
- `JWT_SECRET` - JWT secret for token verification (shared with adi-auth)
- `ANALYTICS_URL` - Analytics ingestion service URL (default: http://localhost:8094)
- `CORS_ORIGIN` - Allowed CORS origin (default: http://localhost:8013)

## Database Migrations
```bash
# Run all migrations
DATABASE_URL=postgres://... cargo run --bin adi-balance-migrate all

# Check migration status
DATABASE_URL=postgres://... cargo run --bin adi-balance-migrate status

# Dry run (show pending)
DATABASE_URL=postgres://... cargo run --bin adi-balance-migrate dry-run
```

## Building
```bash
cargo build --release
```

## Running
```bash
DATABASE_URL=postgres://... JWT_SECRET=... cargo run --bin adi-balance-api
```

## Integration Examples

### Stripe Webhook (deposit after payment)
```rust
balance_client.post("/transactions/deposit")
    .json(&DepositRequest {
        user_id,
        amount: stripe_amount * 1_000_000, // dollars to microtokens
        reference_type: Some("stripe_payment".into()),
        reference_id: Some(payment_intent_id),
        idempotency_key: Some(payment_intent_id.clone()),
        ..Default::default()
    })
    .send().await?;
```

### LLM Proxy (debit on usage)
```rust
balance_client.post("/transactions/debit")
    .json(&DebitRequest {
        user_id,
        amount: cost_in_microtokens,
        reference_type: Some("api_usage".into()),
        reference_id: Some(request_id),
        description: Some(format!("{} tokens", total_tokens)),
        ..Default::default()
    })
    .send().await?;
```

### Pre-check before expensive operation
```rust
let check: CheckBalanceResponse = balance_client.post("/transactions/check")
    .json(&CheckBalanceRequest { user_id, amount: estimated_cost })
    .send().await?.json().await?;

if !check.sufficient {
    return Err("Insufficient balance");
}
```
