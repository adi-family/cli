# lib-plugin-verify

Plugin signature and checksum verification for the universal Rust plugin system.

## Overview

Provides optional security verification for plugin binaries:
- SHA256 checksums for integrity
- Ed25519 signatures for authenticity

## Usage

```rust
use lib_plugin_verify::{Verifier, VerifyResult, verify_checksum, calculate_checksum};

// Checksum verification
let data = std::fs::read("plugin.dylib")?;
let checksum = calculate_checksum(&data);
assert!(verify_checksum(&data, &checksum));

// Signature verification
let verifier = Verifier::new()
    .with_trusted_key("base64-public-key")
    .require_signatures(false);

let result = verifier.verify_signature(&data, Some(&signature), Some(&public_key));
match result {
    VerifyResult::Verified { key_id } => println!("Verified with {}", key_id),
    VerifyResult::UntrustedKey { key } => println!("Valid but untrusted key"),
    VerifyResult::NoSignature => println!("No signature provided"),
    VerifyResult::Invalid { reason } => println!("Invalid: {}", reason),
}
```

## Features

- `keygen` - Enable keypair generation (requires `rand`)
- `signing` - Enable signing functions

## License

MIT
