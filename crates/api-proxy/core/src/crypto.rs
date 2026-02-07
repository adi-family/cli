//! Cryptographic utilities for encrypting/decrypting API keys.
//!
//! Uses ChaCha20-Poly1305 AEAD for symmetric encryption, matching
//! the pattern used in lib-plugin-host.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;

use crate::error::ApiError;

/// Prefix for encrypted values to identify them.
const ENCRYPTED_PREFIX: &str = "ENC:";

/// Nonce size for ChaCha20-Poly1305 (96 bits).
const NONCE_SIZE: usize = 12;

/// Secret manager for encrypting/decrypting sensitive values.
#[derive(Clone)]
pub struct SecretManager {
    key: [u8; 32],
}

impl SecretManager {
    /// Create a new secret manager with the given 32-byte key.
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Create from hex-encoded key string.
    pub fn from_hex(hex_key: &str) -> Result<Self, ApiError> {
        let bytes = hex::decode(hex_key)
            .map_err(|e| ApiError::EncryptionError(format!("Invalid hex key: {}", e)))?;

        let key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| ApiError::EncryptionError("Key must be exactly 32 bytes".to_string()))?;

        Ok(Self::new(key))
    }

    /// Encrypt a plaintext value.
    ///
    /// Returns a string in format: "ENC:{base64(nonce + ciphertext)}"
    pub fn encrypt(&self, plaintext: &str) -> Result<String, ApiError> {
        let cipher = ChaCha20Poly1305::new_from_slice(&self.key)
            .map_err(|e| ApiError::EncryptionError(format!("Failed to create cipher: {}", e)))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| ApiError::EncryptionError(format!("Encryption failed: {}", e)))?;

        // Combine nonce + ciphertext and encode
        let mut combined = nonce_bytes.to_vec();
        combined.extend(ciphertext);

        Ok(format!("{}{}", ENCRYPTED_PREFIX, BASE64.encode(combined)))
    }

    /// Decrypt an encrypted value.
    ///
    /// Expects a string in format: "ENC:{base64(nonce + ciphertext)}"
    pub fn decrypt(&self, encrypted: &str) -> Result<String, ApiError> {
        let encoded = encrypted
            .strip_prefix(ENCRYPTED_PREFIX)
            .ok_or_else(|| ApiError::EncryptionError("Not an encrypted value".to_string()))?;

        let combined = BASE64
            .decode(encoded)
            .map_err(|e| ApiError::EncryptionError(format!("Invalid base64: {}", e)))?;

        if combined.len() < NONCE_SIZE {
            return Err(ApiError::EncryptionError(
                "Encrypted data too short".to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = ChaCha20Poly1305::new_from_slice(&self.key)
            .map_err(|e| ApiError::EncryptionError(format!("Failed to create cipher: {}", e)))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| ApiError::EncryptionError("Decryption failed".to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|e| ApiError::EncryptionError(format!("Invalid UTF-8: {}", e)))
    }

    /// Check if a value is encrypted.
    pub fn is_encrypted(value: &str) -> bool {
        value.starts_with(ENCRYPTED_PREFIX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0u8; 32]; // Test key
        let manager = SecretManager::new(key);

        let plaintext = "sk-test-api-key-12345";
        let encrypted = manager.encrypt(plaintext).unwrap();

        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));
        assert!(SecretManager::is_encrypted(&encrypted));

        let decrypted = manager.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_from_hex() {
        let hex_key = "0000000000000000000000000000000000000000000000000000000000000000";
        let manager = SecretManager::from_hex(hex_key).unwrap();

        let plaintext = "test";
        let encrypted = manager.encrypt(plaintext).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_invalid_encrypted_value() {
        let key = [0u8; 32];
        let manager = SecretManager::new(key);

        assert!(manager.decrypt("not-encrypted").is_err());
        assert!(manager.decrypt("ENC:invalid-base64!!!").is_err());
    }
}
