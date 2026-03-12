//! SHA256 checksum verification.

use sha2::{Digest, Sha256};

use crate::error::VerifyError;

/// Calculate the SHA256 checksum of data.
///
/// Returns a hex-encoded string prefixed with "sha256:".
pub fn calculate_checksum(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    format!("sha256:{:x}", hash)
}

/// Verify that data matches an expected checksum.
///
/// The expected checksum should be in format "sha256:hex...".
/// Returns true if the checksum matches.
pub fn verify_checksum(data: &[u8], expected: &str) -> bool {
    let actual = calculate_checksum(data);
    actual == expected
}

/// Verify checksum and return detailed error on mismatch.
pub fn verify_checksum_strict(data: &[u8], expected: &str) -> Result<(), VerifyError> {
    // Validate format
    if !expected.starts_with("sha256:") {
        return Err(VerifyError::InvalidChecksumFormat(
            "Checksum must start with 'sha256:'".to_string(),
        ));
    }

    let actual = calculate_checksum(data);
    if actual == expected {
        Ok(())
    } else {
        Err(VerifyError::ChecksumMismatch {
            expected: expected.to_string(),
            actual,
        })
    }
}

/// Parse a checksum string and return just the hash portion.
///
/// Returns None if the format is invalid.
pub fn parse_checksum(checksum: &str) -> Option<&str> {
    checksum.strip_prefix("sha256:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_checksum() {
        let data = b"hello world";
        let checksum = calculate_checksum(data);
        assert!(checksum.starts_with("sha256:"));
        // Known SHA256 of "hello world"
        assert_eq!(
            checksum,
            "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_verify_checksum() {
        let data = b"hello world";
        let checksum = "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert!(verify_checksum(data, checksum));
        assert!(!verify_checksum(data, "sha256:wrong"));
    }

    #[test]
    fn test_verify_checksum_strict() {
        let data = b"test data";
        let checksum = calculate_checksum(data);

        // Valid checksum
        assert!(verify_checksum_strict(data, &checksum).is_ok());

        // Invalid format
        assert!(matches!(
            verify_checksum_strict(data, "md5:abc"),
            Err(VerifyError::InvalidChecksumFormat(_))
        ));

        // Mismatch
        assert!(matches!(
            verify_checksum_strict(data, "sha256:wrong"),
            Err(VerifyError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn test_parse_checksum() {
        assert_eq!(parse_checksum("sha256:abc123"), Some("abc123"));
        assert_eq!(parse_checksum("md5:abc123"), None);
        assert_eq!(parse_checksum("invalid"), None);
    }
}
