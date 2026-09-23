use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand_core::OsRng;

use crate::error::SecurityError;

/// Hashes a plaintext secret (master password, Admin PIN, or recovery key —
/// anything a human might type in) with Argon2id, using a fresh random salt
/// each time. The returned PHC string embeds the salt and parameters, so
/// `verify_password` needs nothing else to check a later attempt against it.
pub fn hash_password(password: &str) -> Result<String, SecurityError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| SecurityError::Hash(e.to_string()))?;
    Ok(hash.to_string())
}

/// Verifies a plaintext secret against a previously stored PHC hash.
/// Returns `Ok(false)` for a wrong password and `Err` only if `stored_hash`
/// itself is malformed (e.g. database corruption) — callers should treat
/// both as "not authenticated" but may want to log the latter differently.
pub fn verify_password(password: &str, stored_hash: &str) -> Result<bool, SecurityError> {
    let parsed = PasswordHash::new(stored_hash).map_err(|e| SecurityError::Hash(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_password_hashes_differently_each_time() {
        let a = hash_password("correct-horse-battery-staple").unwrap();
        let b = hash_password("correct-horse-battery-staple").unwrap();
        assert_ne!(a, b, "salts must differ between calls");
    }

    #[test]
    fn verify_succeeds_for_the_correct_password() {
        let hash = hash_password("correct-horse-battery-staple").unwrap();
        assert!(verify_password("correct-horse-battery-staple", &hash).unwrap());
    }

    #[test]
    fn verify_fails_for_the_wrong_password() {
        let hash = hash_password("correct-horse-battery-staple").unwrap();
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn verify_returns_error_for_malformed_stored_hash() {
        let result = verify_password("anything", "not-a-real-phc-string");
        assert!(result.is_err());
    }
}
