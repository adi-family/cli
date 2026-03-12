use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn hmac_sign(data: &str, secret: &str) -> anyhow::Result<String> {
    anyhow::ensure!(!secret.is_empty(), "HMAC secret must not be empty");

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC-SHA256 accepts any key size");
    mac.update(data.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sign() {
        let sig1 = hmac_sign("hive-001", "secret").unwrap();
        let sig2 = hmac_sign("hive-001", "secret").unwrap();
        assert_eq!(sig1, sig2);

        let sig3 = hmac_sign("hive-002", "secret").unwrap();
        assert_ne!(sig1, sig3);

        let sig4 = hmac_sign("hive-001", "different-secret").unwrap();
        assert_ne!(sig1, sig4);
    }

    #[test]
    fn test_hmac_sign_empty_data() {
        let sig = hmac_sign("", "secret").unwrap();
        assert!(!sig.is_empty());
    }

    #[test]
    fn test_hmac_sign_rejects_empty_secret() {
        assert!(hmac_sign("data", "").is_err());
        assert!(hmac_sign("", "").is_err());
    }
}
