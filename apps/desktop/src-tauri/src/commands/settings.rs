use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub recommendation_mode: String,
    pub auto_backup_enabled: bool,
    pub backup_interval_days: u32,
    pub idle_timeout_minutes: u32,
    pub encryption_enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub recommendation_mode: Option<String>,
    pub auto_backup_enabled: Option<bool>,
    pub backup_interval_days: Option<u32>,
    pub idle_timeout_minutes: Option<u32>,
    pub encryption_enabled: Option<bool>,
}

/// Module 14 — Get current application settings
#[tauri::command]
pub fn get_app_settings() -> Result<AppSettings, String> {
    Ok(AppSettings {
        recommendation_mode: "mode-1-rule-based".to_string(),
        auto_backup_enabled: false,
        backup_interval_days: 7,
        idle_timeout_minutes: 15,
        encryption_enabled: true,
    })
}

/// Module 14 — Update application settings
#[tauri::command]
pub fn update_app_settings(
    input: UpdateSettingsInput,
    _state: State<AppState>,
) -> Result<AppSettings, String> {
    let current = get_app_settings()?;

    let updated = AppSettings {
        recommendation_mode: input
            .recommendation_mode
            .unwrap_or(current.recommendation_mode),
        auto_backup_enabled: input
            .auto_backup_enabled
            .unwrap_or(current.auto_backup_enabled),
        backup_interval_days: input
            .backup_interval_days
            .unwrap_or(current.backup_interval_days),
        idle_timeout_minutes: input
            .idle_timeout_minutes
            .unwrap_or(current.idle_timeout_minutes),
        encryption_enabled: input
            .encryption_enabled
            .unwrap_or(current.encryption_enabled),
    };

    // In production, would persist to database or config file
    Ok(updated)
}

/// Module 14 — Get available recommendation modes
#[tauri::command]
pub fn get_recommendation_modes() -> Result<Vec<RecommendationModeInfo>, String> {
    Ok(vec![
        RecommendationModeInfo {
            id: "mode-1-rule-based".to_string(),
            name: "Rule-Based (Self-Learning)".to_string(),
            description: "Fast, deterministic scoring using learned patterns and TF-IDF".to_string(),
            available: true,
            requires_api_key: false,
        },
        RecommendationModeInfo {
            id: "mode-2-claude-supervised".to_string(),
            name: "Claude with Human Review".to_string(),
            description: "LLM-assisted recommendations with mandatory human approval".to_string(),
            available: true,
            requires_api_key: true,
        },
        RecommendationModeInfo {
            id: "mode-3-claude-autonomous".to_string(),
            name: "Claude Autonomous".to_string(),
            description: "Fully autonomous LLM recommendations (requires strict governance)".to_string(),
            available: false,
            requires_api_key: true,
        },
    ])
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationModeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub available: bool,
    pub requires_api_key: bool,
}

/// Module 14 — Get backup/restore status
#[tauri::command]
pub fn get_backup_status() -> Result<BackupStatus, String> {
    Ok(BackupStatus {
        last_backup: None,
        auto_backup_enabled: false,
        backup_location: format!(
            "{}/.aditup-backups",
            dirs::home_dir()
                .unwrap_or_default()
                .to_string_lossy()
        ),
        backup_count: 0,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub last_backup: Option<String>,
    pub auto_backup_enabled: bool,
    pub backup_location: String,
    pub backup_count: u32,
}

/// Module 14 — Get encryption/security settings
#[tauri::command]
pub fn get_security_settings() -> Result<SecuritySettings, String> {
    Ok(SecuritySettings {
        encryption_at_rest: true,
        audit_logging_enabled: true,
        password_policy: PasswordPolicy {
            min_length: 12,
            require_uppercase: true,
            require_digits: true,
            require_special: true,
        },
        session_timeout_minutes: 15,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySettings {
    pub encryption_at_rest: bool,
    pub audit_logging_enabled: bool,
    pub password_policy: PasswordPolicy,
    pub session_timeout_minutes: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordPolicy {
    pub min_length: u32,
    pub require_uppercase: bool,
    pub require_digits: bool,
    pub require_special: bool,
}
