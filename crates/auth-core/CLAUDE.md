adi-auth-core, rust, authentication, email, passwordless, jwt

## Overview
- Email-based passwordless authentication library
- SQLite storage with migrations
- JWT token generation and verification
- SMTP email sending for verification codes
- Optional Axum middleware for protected routes

## Auth Flow
1. User requests code via `request_code(email)`
2. 6-digit code sent to email (expires in 10 minutes)
3. User verifies with `verify_code(email, code)`
4. Returns JWT token valid for 7 days
5. User auto-created on first login

## Environment Variables
- `JWT_SECRET` - Secret for JWT signing (required for production)
- `JWT_EXPIRY_HOURS` - Token expiry (default: 168 = 7 days)
- `SMTP_HOST` - SMTP server hostname
- `SMTP_PORT` - SMTP server port (default: 587)
- `SMTP_USERNAME` - SMTP auth username
- `SMTP_PASSWORD` - SMTP auth password
- `SMTP_FROM_EMAIL` - Sender email address
- `SMTP_FROM_NAME` - Sender display name (default: "ADI")

## Usage
```rust
let auth = AuthManager::open_global()?;
auth.request_code("user@example.com")?;
let token = auth.verify_code("user@example.com", "123456")?;
let claims = auth.verify_token(&token.access_token)?;
```

## Service Integration (Axum)
Enable the `axum` feature to use extractors in other services:
```toml
adi-auth-core = { path = "../adi-auth-core", features = ["axum"] }
```

```rust
use adi_auth_core::{AuthUser, OptionalAuthUser};

// Protected route - returns 401 if no valid token
async fn protected(user: AuthUser) -> String {
    format!("Hello, {}", user.email)
}

// Optional auth - None if no token
async fn public(user: OptionalAuthUser) -> String {
    match user.0 {
        Some(u) => format!("Hello, {}", u.email),
        None => "Hello, guest".to_string(),
    }
}
```

All services must use the same `JWT_SECRET` env var.
