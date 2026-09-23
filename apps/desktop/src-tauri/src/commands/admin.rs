use aditup_db::repositories::keyword_bank::KeywordBankRepository;
use aditup_db::repositories::kle_history::{EntryTimeline, KleHistoryRepository};
use aditup_db::repositories::kle_merge_requests::{
    KleMergeRequestsRepository, MergeRequest, MergeRequestStatus,
};
use aditup_db::repositories::kle_pending_observations::{
    KlePendingObservationsRepository, ObservationPendingStatus, PendingObservation,
};
use aditup_db::repositories::kle_pending_terms::{
    KlePendingKeywordsRepository, KlePendingSynonymsRepository, PendingKeyword, PendingSynonym,
    TermPendingStatus,
};
use aditup_db::repositories::observation_bank::{ObservationBankEntry, ObservationBankRepository};
use aditup_db::repositories::synonym_bank::SynonymBankRepository;
use aditup_kle::{KleEngine, MergeDecision, PendingDecision};
use aditup_scoring_engine::{DEFAULT_DUPLICATE_THRESHOLD, DEFAULT_SHINGLE_SIZE};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::{AppState, OWNER_USERNAME};

// ---------------------------------------------------------------- Pending

#[tauri::command]
pub fn list_pending_observations(
    state: State<AppState>,
) -> Result<Vec<PendingObservation>, String> {
    state.with_admin(|db| {
        KlePendingObservationsRepository::new(db)
            .list_by_status(ObservationPendingStatus::Pending)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn resolve_pending_observation(
    id: i64,
    decision: PendingDecision,
    state: State<AppState>,
) -> Result<(), String> {
    state.with_admin(|db| {
        KleEngine::new(db)
            .resolve_pending(id, decision, Some(OWNER_USERNAME))
            .map_err(|e| e.to_string())
    })
}

// --------------------------------------------------------------- Keywords

#[tauri::command]
pub fn list_pending_keywords(state: State<AppState>) -> Result<Vec<PendingKeyword>, String> {
    state.with_admin(|db| {
        KlePendingKeywordsRepository::new(db)
            .list_by_status(TermPendingStatus::Pending)
            .map_err(|e| e.to_string())
    })
}

/// Approving admits the term into `keyword_bank` (Module 6) and marks the
/// pending row Approved in the same command — there is no separate
/// "publish" step for keywords/synonyms the way there is for observations
/// (§4 only describes a Publish tab for observation-bank rows).
#[tauri::command]
pub fn approve_pending_keyword(id: i64, state: State<AppState>) -> Result<(), String> {
    state.with_admin(|db| {
        let repo = KlePendingKeywordsRepository::new(db);
        let pending = repo
            .get(id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "pending keyword not found".to_string())?;
        KeywordBankRepository::new(db)
            .create(&pending.term, &pending.contexts)
            .map_err(|e| e.to_string())?;
        repo.set_status(id, TermPendingStatus::Approved)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn reject_pending_keyword(id: i64, state: State<AppState>) -> Result<(), String> {
    state.with_admin(|db| {
        KlePendingKeywordsRepository::new(db)
            .set_status(id, TermPendingStatus::Rejected)
            .map_err(|e| e.to_string())
    })
}

// -------------------------------------------------------------- Synonyms

#[tauri::command]
pub fn list_pending_synonyms(state: State<AppState>) -> Result<Vec<PendingSynonym>, String> {
    state.with_admin(|db| {
        KlePendingSynonymsRepository::new(db)
            .list_by_status(TermPendingStatus::Pending)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn approve_pending_synonym(id: i64, state: State<AppState>) -> Result<(), String> {
    state.with_admin(|db| {
        let repo = KlePendingSynonymsRepository::new(db);
        let pending = repo
            .get(id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "pending synonym not found".to_string())?;
        SynonymBankRepository::new(db)
            .create(&pending.term_a, &pending.term_b, pending.confidence)
            .map_err(|e| e.to_string())?;
        repo.set_status(id, TermPendingStatus::Approved)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn reject_pending_synonym(id: i64, state: State<AppState>) -> Result<(), String> {
    state.with_admin(|db| {
        KlePendingSynonymsRepository::new(db)
            .set_status(id, TermPendingStatus::Rejected)
            .map_err(|e| e.to_string())
    })
}

// ---------------------------------------------------------- Merge requests

/// Groups likely duplicates among currently-pending observations into
/// merge requests, at the §7 default shingle size/threshold. Returns the
/// ids of merge requests that exist afterward (idempotent — running this
/// again doesn't duplicate requests already awaiting a decision).
#[tauri::command]
pub fn run_cluster_scan(state: State<AppState>) -> Result<Vec<i64>, String> {
    state.with_admin(|db| {
        KleEngine::new(db)
            .cluster_pending(DEFAULT_SHINGLE_SIZE, DEFAULT_DUPLICATE_THRESHOLD)
            .map_err(|e| e.to_string())
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeRequestRow {
    pub request: MergeRequest,
    pub candidate_a: Option<PendingObservation>,
    pub candidate_b: Option<PendingObservation>,
}

/// Both candidates' pending-observation rows are resolved alongside the
/// request itself, so the UI can render the side-by-side comparison §4
/// calls for without a second round-trip per row.
#[tauri::command]
pub fn list_merge_requests(state: State<AppState>) -> Result<Vec<MergeRequestRow>, String> {
    state.with_admin(|db| {
        let requests = KleMergeRequestsRepository::new(db)
            .list_by_status(MergeRequestStatus::Pending)
            .map_err(|e| e.to_string())?;
        let pending_repo = KlePendingObservationsRepository::new(db);

        requests
            .into_iter()
            .map(|request| {
                let candidate_a = pending_repo
                    .get(request.candidate_a_id)
                    .map_err(|e| e.to_string())?;
                let candidate_b = pending_repo
                    .get(request.candidate_b_id)
                    .map_err(|e| e.to_string())?;
                Ok(MergeRequestRow {
                    request,
                    candidate_a,
                    candidate_b,
                })
            })
            .collect()
    })
}

#[tauri::command]
pub fn resolve_merge_request(
    id: i64,
    decision: MergeDecision,
    state: State<AppState>,
) -> Result<(), String> {
    state.with_admin(|db| {
        KleEngine::new(db)
            .resolve_merge_request(id, decision)
            .map_err(|e| e.to_string())
    })
}

// -------------------------------------------------------------- Publish

#[tauri::command]
pub fn list_approved_pending_observations(
    state: State<AppState>,
) -> Result<Vec<PendingObservation>, String> {
    state.with_admin(|db| {
        KlePendingObservationsRepository::new(db)
            .list_by_status(ObservationPendingStatus::Approved)
            .map_err(|e| e.to_string())
    })
}

/// For the bank_key picker on the Publish tab — existing keys to choose
/// from, though typing a brand new one is equally valid.
#[tauri::command]
pub fn list_bank_keys(state: State<AppState>) -> Result<Vec<String>, String> {
    state.with_admin(|db| {
        ObservationBankRepository::new(db)
            .list_bank_keys()
            .map_err(|e| e.to_string())
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitPublishInput {
    pub pending_id: i64,
    pub bank_key: String,
    pub topic: String,
    pub label: String,
}

/// Irreversible (§9: a publish becomes a new bank version; there is no
/// delete, only republishing a prior version). The frontend is responsible
/// for a confirmation step before calling this.
#[tauri::command]
pub fn commit_publish(
    input: CommitPublishInput,
    state: State<AppState>,
) -> Result<ObservationBankEntry, String> {
    state.with_admin(|db| {
        KleEngine::new(db)
            .commit_publish(
                input.pending_id,
                &input.bank_key,
                &input.topic,
                &input.label,
            )
            .map_err(|e| e.to_string())
    })
}

// ---------------------------------------------------------------- Stats

/// A subset of §4's Stats tab: queue backlog counts and the rejection
/// ratio, both backed by real aggregation queries already in Module 9.
/// Contribution-by-auditor, average-confidence-by-bank, category-coverage
/// gaps, and unused-official-rows all need dedicated queries that don't
/// exist yet — deliberately not faked here.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminStatsDto {
    pub pending_observations: usize,
    pub approved_observations: usize,
    pub rejected_observations: usize,
    pub pending_keywords: usize,
    pub pending_synonyms: usize,
    pub pending_merge_requests: usize,
    pub rejection_ratio: Option<f64>,
}

#[tauri::command]
pub fn get_admin_stats(state: State<AppState>) -> Result<AdminStatsDto, String> {
    state.with_admin(|db| {
        let obs_repo = KlePendingObservationsRepository::new(db);
        let pending_observations = obs_repo
            .list_by_status(ObservationPendingStatus::Pending)
            .map_err(|e| e.to_string())?
            .len();
        let approved_observations = obs_repo
            .list_by_status(ObservationPendingStatus::Approved)
            .map_err(|e| e.to_string())?
            .len();
        let rejected_observations = obs_repo
            .list_by_status(ObservationPendingStatus::Rejected)
            .map_err(|e| e.to_string())?
            .len();
        let pending_keywords = KlePendingKeywordsRepository::new(db)
            .list_by_status(TermPendingStatus::Pending)
            .map_err(|e| e.to_string())?
            .len();
        let pending_synonyms = KlePendingSynonymsRepository::new(db)
            .list_by_status(TermPendingStatus::Pending)
            .map_err(|e| e.to_string())?
            .len();
        let pending_merge_requests = KleMergeRequestsRepository::new(db)
            .list_by_status(MergeRequestStatus::Pending)
            .map_err(|e| e.to_string())?
            .len();
        let rejection_ratio = KleHistoryRepository::new(db)
            .rejection_ratio()
            .map_err(|e| e.to_string())?;

        Ok(AdminStatsDto {
            pending_observations,
            approved_observations,
            rejected_observations,
            pending_keywords,
            pending_synonyms,
            pending_merge_requests,
            rejection_ratio,
        })
    })
}

// --------------------------------------------------------- History search

/// §4's "paste or select an entry hash" — this only implements "paste."
/// Nothing in the UI yet surfaces a browsable list of entry hashes to
/// select from, so that half is a known gap rather than a built feature.
#[tauri::command]
pub fn get_entry_timeline(
    entry_hash: String,
    state: State<AppState>,
) -> Result<EntryTimeline, String> {
    state.with_admin(|db| {
        KleHistoryRepository::new(db)
            .timeline_for_entry(&entry_hash)
            .map_err(|e| e.to_string())
    })
}
