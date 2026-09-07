use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Nonce};
use base64::{engine::general_purpose, Engine};
use rand::RngCore;
use uuid::Uuid;

const KEY_BYTES: &[u8; 32] = b"payroll-system-static-key-v1!!00";

fn cipher() -> Aes256Gcm {
    Aes256Gcm::new(KEY_BYTES.into())
}

pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    general_purpose::STANDARD.encode(bytes)
}

pub fn encrypt_token(plaintext: &str) -> Result<String, String> {
    let cipher = cipher();
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encrypt error: {}", e))?;
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(general_purpose::STANDARD.encode(combined))
}

pub fn decrypt_token(encrypted: &str) -> Result<String, String> {
    let combined = general_purpose::STANDARD
        .decode(encrypted)
        .map_err(|e| format!("Decode error: {}", e))?;
    if combined.len() < 12 {
        return Err("Invalid token".to_string());
    }
    let nonce_bytes = &combined[..12];
    let ciphertext = &combined[12..];
    let cipher = cipher();
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decrypt error: {}", e))?;
    String::from_utf8(plaintext).map_err(|e| format!("UTF8 error: {}", e))
}

pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}
