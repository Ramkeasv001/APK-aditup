use aditup_db::repositories::audits::{Audit, AuditsRepository};
use aditup_db::repositories::observations::{Observation, ObservationsRepository};
use aditup_db::repositories::stage2_assessments::Stage2AssessmentsRepository;
use aditup_db::repositories::stage3_matches::{Stage3Match, Stage3MatchesRepository};
use aditup_kle::{KleEngine, PendingObservationPayload};
use aditup_recommendation_provider::{
    ObservationQuery, RankedCandidate, RecommendationProvider, RuleBasedProvider,
};
use aditup_scoring_engine::{CandidateScores, StructuredTags};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage3ListRow {
    pub observation: Observation,
    /// `None` until the card has been expanded at least once (see
    /// `suggest_candidates`) — Stage 3 loads suggestions lazily per card,
    /// not eagerly for the whole audit at once.
    pub match_state: Option<Stage3Match>,
}

#[tauri::command]
pub fn list_stage3_rows(
    audit_id: String,
    state: State<AppState>,
) -> Result<Vec<Stage3ListRow>, String> {
    state.with_db(|db| {
        let observations = ObservationsRepository::new(db)
            .list_for_audit(&audit_id)
            .map_err(|e| e.to_string())?;
        let matches = Stage3MatchesRepository::new(db);

        observations
            .into_iter()
            .map(|observation| {
                let match_state = matches
                    .get_for_observation(&observation.id)
                    .map_err(|e| e.to_string())?;
                Ok(Stage3ListRow {
                    observation,
                    match_state,
                })
            })
            .collect()
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage3Suggestions {
    pub candidates: Vec<RankedCandidate>,
    /// `None` means genuinely undecided — merely viewing suggestions is
    /// not a decision. A row only ever gets created by an explicit action
    /// (`select_candidate`/`write_own_recommendation`/`escalate_to_admin`),
    /// which is what lets the Stage 3 completion gate tell "hasn't been
    /// looked at" apart from "explicitly escalated" even though both would
    /// otherwise look like the schema's same default `escalated` status.
    pub match_state: Option<Stage3Match>,
}

/// Expanding a card (§4): ranks candidates against the current banks and
/// feedback-adjusted weights, logs the search event, and — via
/// `KleEngine::capture_entry` — always records a confidence snapshot and
/// stages the observation for Admin review if the top score is below the
/// novelty threshold. Does *not* create a `stage3_matches` row — viewing
/// candidates isn't a decision.
#[tauri::command]
pub fn suggest_candidates(
    observation_id: String,
    state: State<AppState>,
) -> Result<Stage3Suggestions, String> {
    state.with_db(|db| {
        let observation = ObservationsRepository::new(db)
            .get(&observation_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "observation not found".to_string())?;

        let assessment = Stage2AssessmentsRepository::new(db)
            .get_for_observation(&observation_id)
            .map_err(|e| e.to_string())?;
        let tags = StructuredTags {
            category: assessment.as_ref().and_then(|a| a.category.clone()),
            department: assessment.as_ref().and_then(|a| a.department.clone()),
            equipment: assessment.as_ref().and_then(|a| a.equipment.clone()),
        };

        let kle = KleEngine::new(db);
        // Per §8, feedback-adjusted weights apply "on the next reload" —
        // this is that reload point, once per suggestion request.
        let weights = kle.ranking_weights().map_err(|e| e.to_string())?;
        let provider = RuleBasedProvider::with_weights(db, weights);

        let query = ObservationQuery {
            text: observation.text.clone(),
            tags: tags.clone(),
            bank_keys: vec![],
        };
        let candidates = provider.suggest(&query, 5).map_err(|e| e.to_string())?;

        kle.log_search(&observation.entry_hash, &observation.text)
            .map_err(|e| e.to_string())?;

        let (confidence_percent, scores, bank_key_guess) = match candidates.first() {
            Some(top) => (
                top.confidence_percent,
                top.scores,
                Some(top.bank_key.clone()),
            ),
            None => (0.0, CandidateScores::default(), None),
        };
        let payload = PendingObservationPayload {
            text: observation.text.clone(),
            location: observation.location.clone(),
            tags,
        };
        kle.capture_entry(
            &observation.entry_hash,
            bank_key_guess.as_deref(),
            &payload,
            confidence_percent,
            &scores,
        )
        .map_err(|e| e.to_string())?;

        let match_state = Stage3MatchesRepository::new(db)
            .get_for_observation(&observation_id)
            .map_err(|e| e.to_string())?;

        Ok(Stage3Suggestions {
            candidates,
            match_state,
        })
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectCandidateInput {
    pub observation_id: String,
    pub bank_key: String,
    pub matched_row_id: String,
    pub match_score: f64,
    pub final_text: String,
    pub scores: CandidateScores,
}

/// "Use this" (§4): persists the choice, marks the card Matched, and logs
/// the selection — which nudges ranking weights toward whichever
/// sub-scores drove this match (§8).
#[tauri::command]
pub fn select_candidate(
    input: SelectCandidateInput,
    state: State<AppState>,
) -> Result<Stage3Match, String> {
    state.with_db(|db| {
        let observation = ObservationsRepository::new(db)
            .get(&input.observation_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "observation not found".to_string())?;

        let updated = Stage3MatchesRepository::new(db)
            .mark_matched(
                &input.observation_id,
                &input.bank_key,
                &input.matched_row_id,
                input.match_score,
                &input.final_text,
            )
            .map_err(|e| e.to_string())?;

        KleEngine::new(db)
            .log_selection(
                &observation.entry_hash,
                Some(&input.bank_key),
                &input.scores,
            )
            .map_err(|e| e.to_string())?;

        Ok(updated)
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectCandidateInput {
    pub observation_id: String,
    pub bank_key: String,
    pub scores: CandidateScores,
}

/// Rejecting a candidate doesn't change the card's status by itself (the
/// auditor might still pick a different candidate, write their own, or
/// escalate) — it only logs the rejection, which nudges ranking weights
/// away from whichever sub-scores drove this candidate's rank (§8).
#[tauri::command]
pub fn reject_candidate(input: RejectCandidateInput, state: State<AppState>) -> Result<(), String> {
    state.with_db(|db| {
        let observation = ObservationsRepository::new(db)
            .get(&input.observation_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "observation not found".to_string())?;

        KleEngine::new(db)
            .log_rejection(
                &observation.entry_hash,
                Some(&input.bank_key),
                &input.scores,
            )
            .map_err(|e| e.to_string())
    })
}

/// "Write your own" (§4): marks the card Written with no bank match.
#[tauri::command]
pub fn write_own_recommendation(
    observation_id: String,
    text: String,
    state: State<AppState>,
) -> Result<Stage3Match, String> {
    state.with_db(|db| {
        Stage3MatchesRepository::new(db)
            .mark_written(&observation_id, &text)
            .map_err(|e| e.to_string())
    })
}

/// Editing the filled text after a selection or write (§4: "logged as an
/// edit event, not silently overwritten").
#[tauri::command]
pub fn edit_final_text(
    observation_id: String,
    new_text: String,
    state: State<AppState>,
) -> Result<Stage3Match, String> {
    state.with_db(|db| {
        let observation = ObservationsRepository::new(db)
            .get(&observation_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "observation not found".to_string())?;
        let before = Stage3MatchesRepository::new(db)
            .get_for_observation(&observation_id)
            .map_err(|e| e.to_string())?
            .and_then(|m| m.final_text);

        let updated = Stage3MatchesRepository::new(db)
            .update_final_text(&observation_id, &new_text)
            .map_err(|e| e.to_string())?;

        KleEngine::new(db)
            .log_edit(&observation.entry_hash, before.as_deref(), Some(&new_text))
            .map_err(|e| e.to_string())?;

        Ok(updated)
    })
}

/// "Escalate to Admin as new" (§4): explicitly marks the card unresolved
/// and forces staging into the KLE pending queue regardless of what the
/// automatic confidence-based check in `suggest_candidates` decided — the
/// auditor has now explicitly said no candidate is acceptable, which
/// overrides a borderline-but-not-quite-below-threshold score.
#[tauri::command]
pub fn escalate_to_admin(observation_id: String, state: State<AppState>) -> Result<(), String> {
    state.with_db(|db| {
        let observation = ObservationsRepository::new(db)
            .get(&observation_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "observation not found".to_string())?;

        Stage3MatchesRepository::new(db)
            .mark_escalated(&observation_id)
            .map_err(|e| e.to_string())?;

        let payload = PendingObservationPayload {
            text: observation.text.clone(),
            location: observation.location.clone(),
            tags: StructuredTags::default(),
        };
        KleEngine::new(db)
            .capture_entry(
                &observation.entry_hash,
                None,
                &payload,
                0.0,
                &CandidateScores::default(),
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    })
}

/// Stage3 -> Finalized. Labeled "Finalize audit" in the UI rather than
/// "Export," on purpose: actual XLSX/PPTX/PDF/CSV generation is Module 11
/// (Export Engine), not built yet. This only advances the status state
/// machine (Module 2) so a finished audit is marked as such.
#[tauri::command]
pub fn finalize_audit(audit_id: String, state: State<AppState>) -> Result<Audit, String> {
    state.with_db(|db| {
        AuditsRepository::new(db)
            .advance_status(&audit_id)
            .map_err(|e| e.to_string())
    })
}
