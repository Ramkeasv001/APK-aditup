use aditup_db::repositories::audits::{Audit, AuditsRepository};
use aditup_db::repositories::observations::ObservationsRepository;
use aditup_db::repositories::stage2_assessments::{
    RiskLevel, Stage2Assessment, Stage2AssessmentsRepository,
};
use aditup_recommendation_provider::{RankedLegalClause, RuleBasedProvider};
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

/// One row of the Stage 2 table (§4): an observation, its assessment if
/// already saved, and an auto-suggested legal clause. The suggestion is
/// computed fresh on every load rather than cached — `legal_bank` starts
/// empty on a new install (no seed-data import tool exists yet), so
/// `suggested_clause` will simply be `None` until an Admin publishes clause
/// data; the wiring is real and works the moment that data exists.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage2Row {
    pub observation_id: String,
    pub text: String,
    pub location: Option<String>,
    pub assessment: Option<Stage2Assessment>,
    pub suggested_clause: Option<RankedLegalClause>,
}

#[tauri::command]
pub fn list_stage2_rows(
    audit_id: String,
    state: State<AppState>,
) -> Result<Vec<Stage2Row>, String> {
    state.with_db(|db| {
        let observations = ObservationsRepository::new(db)
            .list_for_audit(&audit_id)
            .map_err(|e| e.to_string())?;
        let assessments = Stage2AssessmentsRepository::new(db);
        let provider = RuleBasedProvider::new(db);

        observations
            .into_iter()
            .map(|observation| {
                let assessment = assessments
                    .get_for_observation(&observation.id)
                    .map_err(|e| e.to_string())?;
                let suggested_clause = provider
                    .suggest_legal_clauses(&observation.text, None, 1)
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .next();

                Ok(Stage2Row {
                    observation_id: observation.id,
                    text: observation.text,
                    location: observation.location,
                    assessment,
                    suggested_clause,
                })
            })
            .collect()
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn save_stage2_assessment(
    observation_id: String,
    risk_level: RiskLevel,
    category: Option<String>,
    department: Option<String>,
    equipment: Option<String>,
    legal_clause_id: Option<String>,
    state: State<AppState>,
) -> Result<Stage2Assessment, String> {
    state.with_db(|db| {
        Stage2AssessmentsRepository::new(db)
            .upsert(
                &observation_id,
                risk_level,
                category.as_deref(),
                department.as_deref(),
                equipment.as_deref(),
                legal_clause_id.as_deref(),
            )
            .map_err(|e| e.to_string())
    })
}

/// Stage2 -> Stage3, per the Audit status state machine (Module 2). Called
/// when "Save & continue to Stage 3" is pressed.
#[tauri::command]
pub fn advance_to_stage3(audit_id: String, state: State<AppState>) -> Result<Audit, String> {
    state.with_db(|db| {
        AuditsRepository::new(db)
            .advance_status(&audit_id)
            .map_err(|e| e.to_string())
    })
}
