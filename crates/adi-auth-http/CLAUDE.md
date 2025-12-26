adi-auth-http, rust, axum, authentication, http-api

## Overview
- HTTP API for email-based passwordless authentication
- Built on Axum 0.7
- Wraps adi-auth-core functionality

## Endpoints
- `GET /health` - Health check
- `POST /auth/request-code` - Request verification code
- `POST /auth/verify` - Verify code and get JWT token
- `GET /auth/me` - Get current user (requires Bearer token)

## Environment Variables
- `PORT` - Server port (default: 8090)
- `AUTH_DB_PATH` - Database file path (default: global ~/.local/share/adi/auth/)
- Plus all adi-auth-core env vars (JWT_SECRET, SMTP_*)

## API Examples
```bash
# Request code
curl -X POST http://localhost:8090/auth/request-code \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com"}'

# Verify code
curl -X POST http://localhost:8090/auth/verify \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "code": "123456"}'

# Get current user
curl http://localhost:8090/auth/me \
  -H "Authorization: Bearer <token>"
```
