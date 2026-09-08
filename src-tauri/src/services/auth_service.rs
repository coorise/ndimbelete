//! Password hashing and verification (Argon2).

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;

/// Normalize recovery answers: trim + lowercase before hashing / verifying.
pub fn normalize_recovery_answer(answer: &str) -> String {
    answer.trim().to_lowercase()
}

/// Hash a plaintext password for storage.
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("Password hashing failed: {e}"))
}

/// Verify a plaintext password against a stored Argon2 hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    let parsed = PasswordHash::new(hash).map_err(|e| format!("Invalid password hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Hash a recovery hint answer (normalized first).
pub fn hash_recovery_answer(answer: &str) -> Result<String, String> {
    hash_password(&normalize_recovery_answer(answer))
}

/// Verify a recovery hint answer against a stored hash.
pub fn verify_recovery_answer(answer: &str, hash: &str) -> Result<bool, String> {
    verify_password(&normalize_recovery_answer(answer), hash)
}

/// Optional recovery answer → hashed value when non-empty after trim.
pub fn optional_recovery_hash(answer: Option<&str>) -> Result<Option<String>, String> {
    match answer {
        Some(a) if !a.trim().is_empty() => Ok(Some(hash_recovery_answer(a)?)),
        _ => Ok(None),
    }
}
