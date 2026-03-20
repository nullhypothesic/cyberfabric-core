use aes_gcm::{KeyInit, aead::Aead};
use rand::RngCore;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("{0}")]
    Error(String),
}

/// Encrypts a JSON value using AES-256-GCM.
/// Returns `nonce (12 bytes) || ciphertext`.
pub fn encrypt_value(value: &Value, key: &str) -> Result<Vec<u8>, CryptoError> {
    let key_bytes = key.as_bytes();
    if key_bytes.len() != 32 {
        return Err(CryptoError::Error(
            "Encryption key must be 32 bytes long".to_string(),
        ));
    }

    let value_str = serde_json::to_string(value)
        .map_err(|e| CryptoError::Error(format!("Failed to serialize value: {e}")))?;

    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);

    let cipher = aes_gcm::Aes256Gcm::new_from_slice(key_bytes)
        .map_err(|e| CryptoError::Error(format!("Failed to create cipher: {e}")))?;

    let ciphertext = cipher
        .encrypt(&nonce_bytes.into(), value_str.as_bytes())
        .map_err(|e| CryptoError::Error(format!("Failed to encrypt data: {e}")))?;

    let mut result = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypts data produced by [`encrypt_value`].
pub fn decrypt_value(encrypted_data: &[u8], key: &str) -> Result<Value, CryptoError> {
    let key_bytes = key.as_bytes();
    if key_bytes.len() != 32 {
        return Err(CryptoError::Error(
            "Encryption key must be 32 bytes long".to_string(),
        ));
    }

    if encrypted_data.len() < 12 {
        return Err(CryptoError::Error(
            "Encrypted data is too short".to_string(),
        ));
    }

    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);

    let cipher = aes_gcm::Aes256Gcm::new_from_slice(key_bytes)
        .map_err(|e| CryptoError::Error(format!("Failed to create cipher: {e}")))?;

    let decrypted_bytes = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::Error(format!("Failed to decrypt data: {e}")))?;

    let value_str = String::from_utf8(decrypted_bytes)
        .map_err(|e| CryptoError::Error(format!("Failed to decode decrypted data: {e}")))?;

    serde_json::from_str(&value_str)
        .map_err(|e| CryptoError::Error(format!("Failed to parse decrypted JSON: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const KEY_32: &str = "12345678901234567890123456789012";

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let value = json!({ "token": "abc123", "expires": 9999 });
        let encrypted = encrypt_value(&value, KEY_32).expect("encrypt failed");
        let decrypted = decrypt_value(&encrypted, KEY_32).expect("decrypt failed");
        assert_eq!(value, decrypted);
    }

    #[test]
    fn encrypt_decrypt_complex_json() {
        let value = json!({
            "string": "hello",
            "number": 42,
            "boolean": true,
            "null": null,
            "array": [1, 2, 3],
            "nested": { "a": "b" }
        });
        let encrypted = encrypt_value(&value, KEY_32).expect("encrypt failed");
        let decrypted = decrypt_value(&encrypted, KEY_32).expect("decrypt failed");
        assert_eq!(value, decrypted);
    }

    #[test]
    fn encrypt_short_key_returns_error() {
        let value = json!("test");
        let result = encrypt_value(&value, "too_short");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Encryption key must be 32 bytes long"
        );
    }

    #[test]
    fn decrypt_short_key_returns_error() {
        let result = decrypt_value(&[0u8; 20], "too_short");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Encryption key must be 32 bytes long"
        );
    }

    #[test]
    fn decrypt_data_too_short_returns_error() {
        let result = decrypt_value(&[0u8; 8], KEY_32);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Encrypted data is too short"
        );
    }

    #[test]
    fn decrypt_corrupted_data_returns_error() {
        let value = json!("test");
        let mut encrypted = encrypt_value(&value, KEY_32).expect("encrypt failed");
        if let Some(last) = encrypted.last_mut() {
            *last ^= 1;
        }
        assert!(decrypt_value(&encrypted, KEY_32).is_err());
    }

    #[test]
    fn each_encryption_produces_different_ciphertext() {
        let value = json!("same_value");
        let enc1 = encrypt_value(&value, KEY_32).expect("enc1 failed");
        let enc2 = encrypt_value(&value, KEY_32).expect("enc2 failed");
        // Different nonces → different ciphertexts
        assert_ne!(enc1, enc2);
        // But both decrypt to the same value
        assert_eq!(decrypt_value(&enc1, KEY_32).unwrap(), value);
        assert_eq!(decrypt_value(&enc2, KEY_32).unwrap(), value);
    }
}
