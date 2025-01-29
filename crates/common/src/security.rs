use anyhow::Result;
use crate::AppError;
use std::str::FromStr;
use base64::Engine as _;

pub fn encrypt_email(raw_email: &[u8], public_key: &str) -> Result<String, AppError> {
    // Parse the recipient's public key
    let recipient = age::x25519::Recipient::from_str(public_key)
        .map_err(|e| AppError::Mail(format!("Invalid public key: {}", e)))?;

    // Encrypt the email
    let encryptor = age::Encryptor::with_recipients(vec![Box::new(recipient)])
        .ok_or_else(|| AppError::Mail("Failed to create encryptor".to_string()))?;

    let mut encrypted = Vec::new();
    let mut writer = encryptor.wrap_output(&mut encrypted)
        .map_err(|e| AppError::Mail(format!("Encryption error: {}", e)))?;
    
    std::io::Write::write_all(&mut writer, raw_email)
        .map_err(|e| AppError::Mail(format!("Encryption error: {}", e)))?;
    
    writer.finish()
        .map_err(|e| AppError::Mail(format!("Encryption error: {}", e)))?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&encrypted))
}

pub fn decrypt_email(encrypted_content: &str, secret_key: &str) -> Result<Vec<u8>, AppError> {
    // Decode base64 content
    let encrypted = base64::engine::general_purpose::STANDARD.decode(encrypted_content)
        .map_err(|e| AppError::Mail(format!("Base64 decode error: {}", e)))?;

    // Parse the secret key
    let identity = age::x25519::Identity::from_str(secret_key)
        .map_err(|e| AppError::Mail(format!("Invalid secret key: {}", e)))?;

    // Create decryptor
    let decryptor = match age::Decryptor::new(&encrypted[..])
        .map_err(|e| AppError::Mail(format!("Decryption error: {}", e)))? {
        age::Decryptor::Recipients(d) => d,
        _ => return Err(AppError::Mail("Invalid decryptor type".to_string())),
    };

    // Decrypt the content
    let mut decrypted = Vec::new();
    let mut reader = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|e| AppError::Mail(format!("Decryption error: {}", e)))?;

    std::io::Read::read_to_end(&mut reader, &mut decrypted)
        .map_err(|e| AppError::Mail(format!("Decryption error: {}", e)))?;

    Ok(decrypted)
} 