adi-auth, rust, authentication, email, passwordless, jwt, axum, totp, postgresql

## Overview
- Email-based passwordless authentication service
- TOTP authenticator app support (Google Authenticator, Authy, etc.)
- PostgreSQL storage with sqlx migrations
- JWT token generation and verification
- SMTP email sending for verification codes
- HTTP API via Axum

## Structure
- `core/` - Auth logic, storage, JWT, email, TOTP
- `http/` - Axum HTTP server
- `cli/` - Migrations CLI (adi-auth-migrate)

## Migrations CLI
```bash
DATABASE_URL=postgres://... adi-auth-migrate run    # Run all pending migrations
DATABASE_URL=postgres://... adi-auth-migrate info   # Show migration status (not implemented)
DATABASE_URL=postgres://... adi-auth-migrate revert # Revert last migration (not implemented)
```

## Auth Flow (Email)
1. POST /auth/request-code - sends 6-digit code to email
2. POST /auth/verify - validates code, returns JWT (7 day expiry)
3. GET /auth/me - returns current user (requires Bearer token)
4. User auto-created on first login

## Auth Flow (TOTP)
1. POST /auth/totp/setup - returns secret + QR code (requires Bearer token)
2. POST /auth/totp/enable - enables TOTP with secret + code (requires Bearer token)
3. POST /auth/verify-totp - validates TOTP code, returns JWT
4. POST /auth/totp/disable - removes TOTP (requires Bearer token)

## Environment Variables
- `PORT` - HTTP port (default: 8090)
- `DATABASE_URL` - PostgreSQL connection string (required, e.g., postgres://user:password@localhost/adi_auth)
- `JWT_SECRET` - Required for production (min 32 chars)
- `JWT_EXPIRY_HOURS` - Token expiry (default: 168 = 7 days)
- `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD`
- `SMTP_FROM_EMAIL`, `SMTP_FROM_NAME`
- `SMTP_TLS` - TLS mode: true (default), false for plain SMTP

## Build & Run
```bash
# Setup database
createdb adi_auth
DATABASE_URL=postgres://postgres:postgres@localhost/adi_auth adi-auth-migrate run

# Development
DATABASE_URL=postgres://... cargo run -p adi-auth-http

# Docker (runs migrations automatically)
docker build -t adi-auth .
docker run -p 8090:8090 \
  -e DATABASE_URL=postgres://... \
  -e JWT_SECRET=... \
  adi-auth
```

## Database Setup
PostgreSQL 12+ required. Create database:
```bash
createdb adi_auth
```

Then run migrations (see Migrations CLI above).
