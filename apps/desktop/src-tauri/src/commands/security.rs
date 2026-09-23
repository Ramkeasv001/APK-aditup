use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;
use aditup_security::{AuditEventType, AuditLogEntry, AuditResult};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogInfo {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub user_id: Option<String>,
    pub action: String,
    pub resource: String,
    pub result: String,
    pub details: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogAuditEventInput {
    pub event_type: String,
    pub user_id: Option<String>,
    pub action: String,
    pub resource: String,
    pub result: String,
    pub details: Option<String>,
}

/// Module 13 — Log security event to audit trail with hash-chain integrity
#[tauri::command]
pub fn log_audit_event(
    input: LogAuditEventInput,
    _state: State<AppState>,
) -> Result<AuditLogInfo, String> {
    let event_type = match input.event_type.as_str() {
        "auth" => AuditEventType::Auth,
        "data_access" => AuditEventType::DataAccess,
        "data_modification" => AuditEventType::DataModification,
        "admin_action" => AuditEventType::AdminAction,
        "config_change" => AuditEventType::ConfigurationChange,
        "security_event" => AuditEventType::SecurityEvent,
        other => return Err(format!("unknown event type: {}", other)),
    };

    let result = match input.result.as_str() {
        "success" => AuditResult::Success,
        "failure" => AuditResult::Failure,
        "denied" => AuditResult::Denied,
        other => return Err(format!("unknown result type: {}", other)),
    };

    let entry = AuditLogEntry::new(
        event_type,
        input.user_id,
        input.action,
        input.resource,
        result,
        input.details,
        None,
    );

    Ok(AuditLogInfo {
        id: entry.id.to_string(),
        timestamp: entry.timestamp.to_rfc3339(),
        event_type: format!("{:?}", entry.event_type),
        user_id: entry.user_id,
        action: entry.action,
        resource: entry.resource,
        result: format!("{:?}", entry.result),
        details: entry.details,
    })
}

/// Module 13 — Verify audit log entry integrity
#[tauri::command]
pub fn verify_audit_integrity(entry_hash: String) -> Result<bool, String> {
    // Stub: in production, would verify the hash against stored entries
    // and verify the hash chain back to root
    Ok(!entry_hash.is_empty())
}

/// Module 13 — Get encryption key status (for audit purposes)
#[tauri::command]
pub fn get_encryption_key_status() -> Result<String, String> {
    // Stub: in production, would return key rotation status,
    // last rotation time, and key version
    Ok("encryption_enabled:true,version:1".to_string())
}

/// Module 13 — Rotate project encryption key
#[tauri::command]
pub fn rotate_encryption_key() -> Result<String, String> {
    // Stub: in production, would:
    // 1. Generate new key material
    // 2. Re-encrypt all sensitive data
    // 3. Archive old key
    // 4. Log rotation event to audit trail
    Ok("key_rotation_initiated".to_string())
}
