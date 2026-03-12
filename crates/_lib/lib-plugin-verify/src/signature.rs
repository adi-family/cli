//! Ed25519 signature verification.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::{Signature, Verifier as Ed25519Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

// VerifyError used only with signing feature
#[allow(unused_imports)]
use crate::error::VerifyError;

/// Result of signature verification.
#[derive(Debug, Clone)]
pub enum VerifyResult {
    /// Signature is valid and key is trusted
    Verified {
        /// Short identifier for the key (first 16 chars)
        key_id: String,
    },
    /// Signature is valid but key is not in trusted list
    UntrustedKey {
        /// The public key that signed the data
        key: String,
    },
    /// No signature was provided
    NoSignature,
    /// Signature is invalid
    Invalid {
        /// Reason for invalidity
        reason: String,
    },
}

impl VerifyResult {
    /// Check if verification succeeded (verified or untrusted but valid).
    pub fn is_valid(&self) -> bool {
        matches!(
            self,
            VerifyResult::Verified { .. } | VerifyResult::UntrustedKey { .. }
        )
    }

    /// Check if the signature is from a trusted key.
    pub fn is_trusted(&self) -> bool {
        matches!(self, VerifyResult::Verified { .. })
    }
}

/// Signature verifier with trusted key management.
#[derive(Debug, Clone, Default)]
pub struct Verifier {
    /// Trusted public keys (base64 encoded)
    trusted_keys: Vec<String>,
    /// Whether to require signatures
    require_signature: bool,
}

impl Verifier {
    /// Create a new verifier with no trusted keys.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a trusted public key.
    pub fn with_trusted_key(mut self, key: &str) -> Self {
        self.trusted_keys.push(key.to_string());
        self
    }

    /// Add multiple trusted public keys.
    pub fn with_trusted_keys(mut self, keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.trusted_keys.extend(keys.into_iter().map(Into::into));
        self
    }

    /// Set whether signatures are required.
    pub fn require_signatures(mut self, require: bool) -> Self {
        self.require_signature = require;
        self
    }

    /// Check if a key is in the trusted list.
    pub fn is_trusted(&self, key: &str) -> bool {
        self.trusted_keys.iter().any(|k| k == key)
    }

    /// Verify a signature against data.
    ///
    /// # Arguments
    /// * `data` - The data that was signed
    /// * `signature` - The signature bytes (raw Ed25519 signature)
    /// * `public_key` - The public key (base64 encoded)
    pub fn verify_signature(
        &self,
        data: &[u8],
        signature: Option<&[u8]>,
        public_key: Option<&str>,
    ) -> VerifyResult {
        let Some(sig_bytes) = signature else {
            return if self.require_signature {
                VerifyResult::Invalid {
                    reason: "Signature required but not provided".to_string(),
                }
            } else {
                VerifyResult::NoSignature
            };
        };

        let Some(key_str) = public_key else {
            return VerifyResult::Invalid {
                reason: "No public key provided".to_string(),
            };
        };

        // Decode public key
        let key_bytes = match BASE64.decode(key_str) {
            Ok(b) => b,
            Err(e) => {
                return VerifyResult::Invalid {
                    reason: format!("Invalid public key encoding: {}", e),
                }
            }
        };

        let key_array: [u8; 32] = match key_bytes.try_into() {
            Ok(a) => a,
            Err(_) => {
                return VerifyResult::Invalid {
                    reason: "Public key must be 32 bytes".to_string(),
                }
            }
        };

        let verifying_key = match VerifyingKey::from_bytes(&key_array) {
            Ok(k) => k,
            Err(e) => {
                return VerifyResult::Invalid {
                    reason: format!("Invalid public key: {}", e),
                }
            }
        };

        // Parse signature
        let signature = match Signature::from_slice(sig_bytes) {
            Ok(s) => s,
            Err(e) => {
                return VerifyResult::Invalid {
                    reason: format!("Invalid signature format: {}", e),
                }
            }
        };

        // Hash the data (sign the hash, not raw data)
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();

        // Verify
        if verifying_key.verify(&hash, &signature).is_ok() {
            if self.is_trusted(key_str) {
                VerifyResult::Verified {
                    key_id: key_str.chars().take(16).collect(),
                }
            } else {
                VerifyResult::UntrustedKey {
                    key: key_str.to_string(),
                }
            }
        } else {
            VerifyResult::Invalid {
                reason: "Signature verification failed".to_string(),
            }
        }
    }

    /// Verify signature from base64-encoded signature string.
    pub fn verify_signature_base64(
        &self,
        data: &[u8],
        signature_base64: Option<&str>,
        public_key: Option<&str>,
    ) -> VerifyResult {
        let sig_bytes = signature_base64.and_then(|s| BASE64.decode(s).ok());
        self.verify_signature(data, sig_bytes.as_deref(), public_key)
    }
}

/// Generate a new Ed25519 keypair for signing plugins.
///
/// Returns (private_key_base64, public_key_base64).
#[cfg(feature = "keygen")]
pub fn generate_keypair() -> (String, String) {
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let private_key = BASE64.encode(signing_key.to_bytes());
    let public_key = BASE64.encode(verifying_key.to_bytes());

    (private_key, public_key)
}

/// Sign data with a private key.
///
/// Returns the signature as base64.
#[cfg(feature = "signing")]
pub fn sign_data(data: &[u8], private_key_base64: &str) -> Result<String, VerifyError> {
    use ed25519_dalek::{Signer, SigningKey};

    let key_bytes = BASE64
        .decode(private_key_base64)
        .map_err(VerifyError::Base64Error)?;

    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| VerifyError::InvalidPublicKey("Private key must be 32 bytes".to_string()))?;

    let signing_key = SigningKey::from_bytes(&key_array);

    // Hash the data first
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    let signature = signing_key.sign(&hash);
    Ok(BASE64.encode(signature.to_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test key pair (DO NOT USE IN PRODUCTION)
    const TEST_PRIVATE_KEY: &str = "nWGxne/9WmC6hEr0kuwsxERJxWl7MmkZcDusAxyuf2A=";
    const TEST_PUBLIC_KEY: &str = "11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=";

    fn sign_test_data(data: &[u8]) -> Vec<u8> {
        use ed25519_dalek::{Signer, SigningKey};

        let key_bytes = BASE64.decode(TEST_PRIVATE_KEY).unwrap();
        let key_array: [u8; 32] = key_bytes.try_into().unwrap();
        let signing_key = SigningKey::from_bytes(&key_array);

        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();

        signing_key.sign(&hash).to_bytes().to_vec()
    }

    #[test]
    fn test_verify_valid_signature() {
        let data = b"test data to sign";
        let signature = sign_test_data(data);

        let verifier = Verifier::new().with_trusted_key(TEST_PUBLIC_KEY);

        let result = verifier.verify_signature(data, Some(&signature), Some(TEST_PUBLIC_KEY));
        assert!(matches!(result, VerifyResult::Verified { .. }));
    }

    #[test]
    fn test_verify_untrusted_key() {
        let data = b"test data to sign";
        let signature = sign_test_data(data);

        let verifier = Verifier::new(); // No trusted keys

        let result = verifier.verify_signature(data, Some(&signature), Some(TEST_PUBLIC_KEY));
        assert!(matches!(result, VerifyResult::UntrustedKey { .. }));
    }

    #[test]
    fn test_verify_no_signature() {
        let verifier = Verifier::new();

        let result = verifier.verify_signature(b"data", None, Some(TEST_PUBLIC_KEY));
        assert!(matches!(result, VerifyResult::NoSignature));
    }

    #[test]
    fn test_verify_required_signature() {
        let verifier = Verifier::new().require_signatures(true);

        let result = verifier.verify_signature(b"data", None, Some(TEST_PUBLIC_KEY));
        assert!(matches!(result, VerifyResult::Invalid { .. }));
    }

    #[test]
    fn test_verify_invalid_signature() {
        let data = b"test data";
        let wrong_signature = vec![0u8; 64]; // Invalid signature

        let verifier = Verifier::new();

        let result = verifier.verify_signature(data, Some(&wrong_signature), Some(TEST_PUBLIC_KEY));
        assert!(matches!(result, VerifyResult::Invalid { .. }));
    }

    #[test]
    fn test_verify_result_helpers() {
        let verified = VerifyResult::Verified {
            key_id: "test".to_string(),
        };
        assert!(verified.is_valid());
        assert!(verified.is_trusted());

        let untrusted = VerifyResult::UntrustedKey {
            key: "key".to_string(),
        };
        assert!(untrusted.is_valid());
        assert!(!untrusted.is_trusted());

        let invalid = VerifyResult::Invalid {
            reason: "test".to_string(),
        };
        assert!(!invalid.is_valid());
        assert!(!invalid.is_trusted());
    }
}
