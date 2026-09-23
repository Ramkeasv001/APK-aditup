//! ADITUP desktop host (Tauri).
//!
//! This crate owns the process boundary only: window bootstrap, logging,
//! resolving the encrypted database's file path, and wiring Tauri IPC
//! commands to the domain crates (db, security, scoring-engine, kle,
//! recommendation-provider, export). It must never contain business logic
//! itself — see docs/architecture.

mod commands;
mod state;

use serde::Serialize;
use tauri::Manager;

use state::AppState;

/// Static build/version metadata surfaced to the frontend so the UI can show
/// "About" information without hardcoding the version in two places.
#[derive(Serialize)]
struct AppInfo {
    name: &'static str,
    version: &'static str,
    mode: &'static str,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "ADITUP",
        version: env!("CARGO_PKG_VERSION"),
        // Mode 1 = rule-based + self-learning engine (no LLM). Modes 2/3 are
        // selected at runtime once the recommendation-provider seam (§14 of
        // the architecture doc) lands; hardcoded here until Module 14 (Settings).
        mode: "mode-1-rule-based",
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("resolve app data directory");
            std::fs::create_dir_all(&app_data_dir).expect("create app data directory");
            let db_path = app_data_dir.join("aditup.sqlite");

            log::info!(
                "ADITUP desktop host starting up (db at {})",
                db_path.display()
            );
            app.manage(AppState::new(db_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            commands::auth::check_setup_status,
            commands::auth::run_setup,
            commands::auth::unlock,
            commands::auth::lock_session,
            commands::auth::session_status,
            commands::stage1::start_audit,
            commands::stage1::list_observations,
            commands::stage1::add_observation,
            commands::stage1::update_observation,
            commands::stage1::delete_observation,
            commands::stage1::advance_to_stage2,
            commands::stage2::list_stage2_rows,
            commands::stage2::save_stage2_assessment,
            commands::stage2::advance_to_stage3,
            commands::stage3::list_stage3_rows,
            commands::stage3::suggest_candidates,
            commands::stage3::select_candidate,
            commands::stage3::reject_candidate,
            commands::stage3::write_own_recommendation,
            commands::stage3::edit_final_text,
            commands::stage3::escalate_to_admin,
            commands::stage3::finalize_audit,
            commands::admin_auth::is_admin_pin_set,
            commands::admin_auth::set_admin_pin,
            commands::admin_auth::verify_admin_pin,
            commands::admin_auth::exit_admin,
            commands::admin::list_pending_observations,
            commands::admin::resolve_pending_observation,
            commands::admin::list_pending_keywords,
            commands::admin::approve_pending_keyword,
            commands::admin::reject_pending_keyword,
            commands::admin::list_pending_synonyms,
            commands::admin::approve_pending_synonym,
            commands::admin::reject_pending_synonym,
            commands::admin::run_cluster_scan,
            commands::admin::list_merge_requests,
            commands::admin::resolve_merge_request,
            commands::admin::list_approved_pending_observations,
            commands::admin::list_bank_keys,
            commands::admin::commit_publish,
            commands::admin::get_admin_stats,
            commands::admin::get_entry_timeline,
            commands::analytics::get_audit_analytics,
            commands::backup::create_database_backup,
            commands::backup::verify_backup,
            commands::backup::restore_from_backup,
            commands::backup::list_backups,
            commands::security::log_audit_event,
            commands::security::verify_audit_integrity,
            commands::security::get_encryption_key_status,
            commands::security::rotate_encryption_key,
            commands::settings::get_app_settings,
            commands::settings::update_app_settings,
            commands::settings::get_recommendation_modes,
            commands::settings::get_backup_status,
            commands::settings::get_security_settings,
            commands::export::export_audit_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_reports_mode_1() {
        let info = app_info();
        assert_eq!(info.name, "ADITUP");
        assert_eq!(info.mode, "mode-1-rule-based");
        assert!(!info.version.is_empty());
    }
}
