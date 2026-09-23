use aditup_db::repositories::audits::{Audit, AuditsRepository};
use aditup_db::repositories::observations::{Observation, ObservationsRepository};
use aditup_db::repositories::projects::ProjectsRepository;
use aditup_db::repositories::sites::SitesRepository;
use aditup_db::repositories::users::UsersRepository;
use tauri::State;

use crate::state::{AppState, OWNER_USERNAME};

/// Combined Stage 1 "Audit Details" submission (§4): creates the
/// project/site/audit chain in one step, rather than requiring a separate
/// project-browsing screen first (the Landing screen's project list/select/
/// archive UI is a future refinement, not in this module's scope — every
/// submission here starts a fresh project and site). `client` becomes the
/// project's name.
#[tauri::command]
pub fn start_audit(
    client: String,
    site_name: String,
    location: Option<String>,
    audit_date: String,
    scope_notes: Option<String>,
    state: State<AppState>,
) -> Result<Audit, String> {
    state.with_db(|db| {
        let project = ProjectsRepository::new(db)
            .create(&client, Some(&client))
            .map_err(|e| e.to_string())?;
        let site = SitesRepository::new(db)
            .create(&project.id, &site_name, location.as_deref())
            .map_err(|e| e.to_string())?;
        let owner = UsersRepository::new(db)
            .get_by_username(OWNER_USERNAME)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "no account found".to_string())?;

        AuditsRepository::new(db)
            .create(&site.id, &owner.id, &audit_date, scope_notes.as_deref())
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn list_observations(
    audit_id: String,
    state: State<AppState>,
) -> Result<Vec<Observation>, String> {
    state.with_db(|db| {
        ObservationsRepository::new(db)
            .list_for_audit(&audit_id)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn add_observation(
    audit_id: String,
    text: String,
    location: Option<String>,
    state: State<AppState>,
) -> Result<Observation, String> {
    state.with_db(|db| {
        ObservationsRepository::new(db)
            .create(&audit_id, &text, location.as_deref())
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn update_observation(
    id: String,
    text: String,
    state: State<AppState>,
) -> Result<Observation, String> {
    state.with_db(|db| {
        ObservationsRepository::new(db)
            .update_text(&id, &text)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn delete_observation(id: String, state: State<AppState>) -> Result<(), String> {
    state.with_db(|db| {
        ObservationsRepository::new(db)
            .delete(&id)
            .map_err(|e| e.to_string())
    })
}

/// Draft -> Stage2, per the Audit status state machine (Module 2). Called
/// when "Review & continue to Stage 2" is pressed.
#[tauri::command]
pub fn advance_to_stage2(audit_id: String, state: State<AppState>) -> Result<Audit, String> {
    state.with_db(|db| {
        AuditsRepository::new(db)
            .advance_status(&audit_id)
            .map_err(|e| e.to_string())
    })
}
