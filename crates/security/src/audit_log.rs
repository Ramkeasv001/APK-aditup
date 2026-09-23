use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub user_id: Option<String>,
    pub action: String,
    pub resource: String,
    pub result: AuditResult,
    pub details: Option<String>,
    pub hash: String,
    pub previous_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    Auth,
    DataAccess,
    DataModification,
    AdminAction,
    ConfigurationChange,
    SecurityEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuditResult {
    Success,
    Failure,
    Denied,
}

impl AuditLogEntry {
    pub fn new(
        event_type: AuditEventType,
        user_id: Option<String>,
        action: String,
        resource: String,
        result: AuditResult,
        details: Option<String>,
        previous_hash: Option<String>,
    ) -> Self {
        let entry = Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            user_id,
            action,
            resource,
            result,
            details,
            hash: String::new(),
            previous_hash,
        };

        // Calculate hash with previous_hash to create a chain
        let hash = entry.calculate_hash();

        Self {
            hash,
            ..entry
        }
    }

    /// Calculate SHA256 hash for this entry and optional chain to previous
    fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(self.id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(format!("{:?}", self.event_type).as_bytes());
        hasher.update(self.user_id.as_deref().unwrap_or("").as_bytes());
        hasher.update(self.action.as_bytes());
        hasher.update(self.resource.as_bytes());
        hasher.update(format!("{:?}", self.result).as_bytes());
        hasher.update(self.details.as_deref().unwrap_or("").as_bytes());

        if let Some(prev) = &self.previous_hash {
            hasher.update(prev.as_bytes());
        }

        hex::encode(hasher.finalize())
    }

    /// Verify entry hash integrity
    pub fn verify_hash(&self) -> bool {
        self.hash == self.calculate_hash()
    }

    /// Verify hash chain (current entry's hash should match next entry's previous_hash)
    pub fn verify_chain_link(&self, next_entry: &AuditLogEntry) -> bool {
        next_entry.previous_hash.as_ref() == Some(&self.hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_entry_verifies_own_hash() {
        let entry = AuditLogEntry::new(
            AuditEventType::Auth,
            Some("user1".to_string()),
            "login".to_string(),
            "user:user1".to_string(),
            AuditResult::Success,
            None,
            None,
        );

        assert!(entry.verify_hash());
    }

    #[test]
    fn audit_chain_verification_works() {
        let entry1 = AuditLogEntry::new(
            AuditEventType::Auth,
            Some("user1".to_string()),
            "login".to_string(),
            "user:user1".to_string(),
            AuditResult::Success,
            None,
            None,
        );

        let entry2 = AuditLogEntry::new(
            AuditEventType::DataAccess,
            Some("user1".to_string()),
            "read".to_string(),
            "audit:audit1".to_string(),
            AuditResult::Success,
            None,
            Some(entry1.hash.clone()),
        );

        assert!(entry1.verify_chain_link(&entry2));
    }
}
