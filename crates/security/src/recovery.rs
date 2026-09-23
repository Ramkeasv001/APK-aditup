use rand_core::{OsRng, RngCore};

use crate::error::SecurityError;
use crate::password::{hash_password, verify_password};

/// Generates a new one-time recovery key and its Argon2id hash.
///
/// The plaintext is returned exactly once so the caller (Module 10's setup
/// wizard) can show it to the user and tell them to store it somewhere
/// outside the app. Only the hash is persisted (in `admin_settings` under
/// `recovery_key_hash`) — there is deliberately no way to recover the
/// plaintext from what's stored, matching §9's "no backdoor password reset."
/// Losing this key means losing the ability to reset a forgotten master
/// password.
pub fn generate_recovery_key() -> Result<(String, String), SecurityError> {
    let mut bytes = [0u8; 20];
    OsRng.fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let grouped = hex
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).expect("hex digits are always valid utf8"))
        .collect::<Vec<_>>()
        .join("-");
    let hash = hash_password(&grouped)?;
    Ok((grouped, hash))
}

pub fn verify_recovery_key(candidate: &str, stored_hash: &str) -> Result<bool, SecurityError> {
    verify_password(candidate, stored_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_are_unique_and_readably_formatted() {
        let (key_a, _) = generate_recovery_key().unwrap();
        let (key_b, _) = generate_recovery_key().unwrap();

        assert_ne!(key_a, key_b);
        assert!(
            key_a.contains('-'),
            "expected dash-grouped key, got {key_a}"
        );
        assert_eq!(key_a.len(), 49); // 40 hex chars + 9 dashes
    }

    #[test]
    fn correct_recovery_key_verifies_against_its_hash() {
        let (key, hash) = generate_recovery_key().unwrap();
        assert!(verify_recovery_key(&key, &hash).unwrap());
    }

    #[test]
    fn wrong_recovery_key_does_not_verify() {
        let (_, hash) = generate_recovery_key().unwrap();
        assert!(
            !verify_recovery_key("0000-0000-0000-0000-0000-0000-0000-0000-0000-0000", &hash)
                .unwrap()
        );
    }
}
