use aditup_db::repositories::admin_settings::AdminSettingsRepository;
use aditup_db::repositories::kle_history::KleHistoryRepository;
use aditup_db::repositories::kle_merge_requests::{KleMergeRequestsRepository, MergeRequestStatus};
use aditup_db::repositories::kle_pending_observations::{
    KlePendingObservationsRepository, ObservationPendingStatus,
};
use aditup_db::repositories::kle_pending_terms::KlePendingKeywordsRepository;
use aditup_db::repositories::observation_bank::{ObservationBankEntry, ObservationBankRepository};
use aditup_db::{Database, DbError};
use aditup_scoring_engine::{
    adjust_for_feedback, cosine_similarity, is_below_novelty_threshold, preprocess_default,
    shingles, CandidateScores, FeedbackDirection, RankingWeights,
};

use crate::error::KleError;
use crate::types::{
    CaptureOutcome, ConfidenceBreakdown, MergeDecision, PendingDecision, PendingObservationPayload,
};

const RANKING_WEIGHTS_SETTING: &str = "ranking_weights";

/// Orchestrates the continuous learning loop from §8: capture -> stage (if
/// novel) -> cluster -> Admin resolves -> publish, plus the
/// selection/rejection feedback that nudges ranking weights over time.
/// Wraps `aditup-db` repositories and `aditup-scoring-engine`'s pure
/// duplicate-detection/feedback math; owns no storage of its own.
pub struct KleEngine<'a> {
    db: &'a Database,
}

impl<'a> KleEngine<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Always records a confidence snapshot. If `confidence_percent` is
    /// below the novelty threshold, stages (or re-stages) the observation
    /// for Admin review and mines its tokens as candidate keywords —
    /// mining only runs for genuinely novel content, not every observation,
    /// so the pending-keyword queue doesn't fill with already-known terms.
    pub fn capture_entry(
        &self,
        entry_hash: &str,
        bank_key_guess: Option<&str>,
        payload: &PendingObservationPayload,
        confidence_percent: f64,
        scores: &CandidateScores,
    ) -> Result<CaptureOutcome, KleError> {
        let breakdown = ConfidenceBreakdown {
            percent: confidence_percent,
            scores: *scores,
        };
        KleHistoryRepository::new(self.db)
            .log_confidence(entry_hash, &serde_json::to_string(&breakdown)?)?;

        if !is_below_novelty_threshold(confidence_percent) {
            return Ok(CaptureOutcome::ConfidentMatch);
        }

        let payload_json = serde_json::to_string(payload)?;
        let pending = KlePendingObservationsRepository::new(self.db).upsert(
            entry_hash,
            bank_key_guess,
            &payload_json,
        )?;

        self.mine_keywords(&payload.text, bank_key_guess)?;

        Ok(CaptureOutcome::Staged(pending.id))
    }

    fn mine_keywords(&self, text: &str, bank_key_guess: Option<&str>) -> Result<(), KleError> {
        let repo = KlePendingKeywordsRepository::new(self.db);
        for token in preprocess_default(text) {
            repo.observe(&token, bank_key_guess)?;
        }
        Ok(())
    }

    pub fn log_search(&self, entry_hash: &str, query: &str) -> Result<(), KleError> {
        KleHistoryRepository::new(self.db).log_search(entry_hash, query)?;
        Ok(())
    }

    /// Logs the selection and nudges ranking weights toward whichever
    /// sub-scores drove this match, returning the adjusted weights (already
    /// persisted — the caller doesn't need to save them separately).
    pub fn log_selection(
        &self,
        entry_hash: &str,
        chosen_bank_key: Option<&str>,
        scores: &CandidateScores,
    ) -> Result<RankingWeights, KleError> {
        KleHistoryRepository::new(self.db).log_selection(entry_hash, chosen_bank_key)?;
        self.apply_feedback(scores, FeedbackDirection::Positive)
    }

    pub fn log_rejection(
        &self,
        entry_hash: &str,
        rejected_bank_key: Option<&str>,
        scores: &CandidateScores,
    ) -> Result<RankingWeights, KleError> {
        KleHistoryRepository::new(self.db).log_rejection(entry_hash, rejected_bank_key)?;
        self.apply_feedback(scores, FeedbackDirection::Negative)
    }

    pub fn log_edit(
        &self,
        entry_hash: &str,
        before: Option<&str>,
        after: Option<&str>,
    ) -> Result<(), KleError> {
        KleHistoryRepository::new(self.db).log_edit(entry_hash, before, after)?;
        Ok(())
    }

    fn apply_feedback(
        &self,
        scores: &CandidateScores,
        direction: FeedbackDirection,
    ) -> Result<RankingWeights, KleError> {
        let current = self.ranking_weights()?;
        let adjusted = adjust_for_feedback(&current, scores, direction);
        AdminSettingsRepository::new(self.db)
            .set(RANKING_WEIGHTS_SETTING, &serde_json::to_string(&adjusted)?)?;
        Ok(adjusted)
    }

    /// The current feedback-adjusted ranking weights, or the §7 defaults if
    /// no feedback has been recorded yet. This is what `RuleBasedProvider`
    /// (Module 8) should load via `with_weights` on its next reload — per
    /// §8, adjustments apply on reload, not live mid-session.
    pub fn ranking_weights(&self) -> Result<RankingWeights, KleError> {
        match AdminSettingsRepository::new(self.db).get(RANKING_WEIGHTS_SETTING)? {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Ok(RankingWeights::default()),
        }
    }

    /// Compares every currently-pending observation against every other
    /// pairwise, creating (or reusing) a merge request wherever their
    /// shingle-cosine similarity meets `threshold`. Returns the ids of the
    /// merge requests that exist as a result (created this run or already
    /// pending from a previous run).
    pub fn cluster_pending(
        &self,
        shingle_size: usize,
        threshold: f64,
    ) -> Result<Vec<i64>, KleError> {
        let pending = KlePendingObservationsRepository::new(self.db)
            .list_by_status(ObservationPendingStatus::Pending)?;
        let merge_repo = KleMergeRequestsRepository::new(self.db);

        let tokenized: Vec<(i64, Vec<String>)> = pending
            .iter()
            .filter_map(|p| {
                serde_json::from_str::<PendingObservationPayload>(&p.payload)
                    .ok()
                    .map(|payload| (p.id, preprocess_default(&payload.text)))
            })
            .collect();

        let mut merge_request_ids = Vec::new();
        for i in 0..tokenized.len() {
            for j in (i + 1)..tokenized.len() {
                let (id_a, tokens_a) = &tokenized[i];
                let (id_b, tokens_b) = &tokenized[j];
                let similarity = cosine_similarity(
                    &shingles(tokens_a, shingle_size),
                    &shingles(tokens_b, shingle_size),
                );
                if similarity >= threshold {
                    let request = merge_repo.create_if_absent(*id_a, *id_b, similarity)?;
                    merge_request_ids.push(request.id);
                }
            }
        }
        Ok(merge_request_ids)
    }

    /// Admin's decision on one pending observation. Approving logs a
    /// `kle_history_approval` event; rejecting does not (there's nothing to
    /// attribute an approver to).
    pub fn resolve_pending(
        &self,
        id: i64,
        decision: PendingDecision,
        admin_user_id: Option<&str>,
    ) -> Result<(), KleError> {
        let repo = KlePendingObservationsRepository::new(self.db);
        let pending = match repo.get(id)? {
            Some(p) if p.status == ObservationPendingStatus::Pending => p,
            _ => return Err(KleError::NotPending(id)),
        };

        let new_status = match decision {
            PendingDecision::Approve => ObservationPendingStatus::Approved,
            PendingDecision::Reject => ObservationPendingStatus::Rejected,
        };
        repo.set_status(id, new_status)?;

        if decision == PendingDecision::Approve {
            KleHistoryRepository::new(self.db)
                .log_approval(&pending.source_entry_hash, admin_user_id)?;
        }
        Ok(())
    }

    /// Admin's decision on one merge request — see `MergeDecision` for what
    /// each variant does to the two underlying pending observations.
    pub fn resolve_merge_request(&self, id: i64, decision: MergeDecision) -> Result<(), KleError> {
        let merge_repo = KleMergeRequestsRepository::new(self.db);
        let request = merge_repo.get(id)?.ok_or(DbError::NotFound)?;
        let pending_repo = KlePendingObservationsRepository::new(self.db);

        match decision {
            MergeDecision::Merge => {
                pending_repo
                    .set_status(request.candidate_b_id, ObservationPendingStatus::Merged)?;
                merge_repo.set_status(id, MergeRequestStatus::Merged)?;
            }
            MergeDecision::KeepBoth => {
                merge_repo.set_status(id, MergeRequestStatus::KeptBoth)?;
            }
            MergeDecision::Discard => {
                pending_repo
                    .set_status(request.candidate_a_id, ObservationPendingStatus::Rejected)?;
                pending_repo
                    .set_status(request.candidate_b_id, ObservationPendingStatus::Rejected)?;
                merge_repo.set_status(id, MergeRequestStatus::Discarded)?;
            }
        }
        Ok(())
    }

    /// Publishes an approved pending observation into `observation_bank`
    /// (Module 4), bumping that bank's `official_meta` version. `bank_key`/
    /// `topic`/`label` are supplied by the Admin at publish time — they may
    /// differ from `bank_key_guess`, which was only ever a suggestion.
    /// Marks the pending row `Merged` on success: its lifecycle is
    /// complete, now represented by the new official row instead.
    pub fn commit_publish(
        &self,
        pending_id: i64,
        bank_key: &str,
        topic: &str,
        label: &str,
    ) -> Result<ObservationBankEntry, KleError> {
        let pending_repo = KlePendingObservationsRepository::new(self.db);
        let pending = pending_repo.get(pending_id)?.ok_or(DbError::NotFound)?;
        if pending.status != ObservationPendingStatus::Approved {
            return Err(KleError::NotApproved(pending_id));
        }

        let payload: PendingObservationPayload = serde_json::from_str(&pending.payload)?;
        let entry = ObservationBankRepository::new(self.db).create(
            bank_key,
            topic,
            label,
            &payload.text,
        )?;
        pending_repo.set_status(pending_id, ObservationPendingStatus::Merged)?;

        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:"), "test-passphrase").unwrap()
    }

    fn payload(text: &str) -> PendingObservationPayload {
        PendingObservationPayload {
            text: text.to_string(),
            location: None,
            tags: Default::default(),
        }
    }

    #[test]
    fn capture_entry_stages_a_below_threshold_observation_and_logs_confidence() {
        let db = test_db();
        let engine = KleEngine::new(&db);

        let outcome = engine
            .capture_entry(
                "hash-1",
                Some("bank_a"),
                &payload("brand new hazard type"),
                25.0,
                &CandidateScores::default(),
            )
            .unwrap();

        assert!(matches!(outcome, CaptureOutcome::Staged(_)));
        let timeline = KleHistoryRepository::new(&db)
            .timeline_for_entry("hash-1")
            .unwrap();
        assert_eq!(timeline.confidence_snapshots.len(), 1);
    }

    #[test]
    fn capture_entry_does_not_stage_a_confident_match() {
        let db = test_db();
        let engine = KleEngine::new(&db);

        let outcome = engine
            .capture_entry(
                "hash-1",
                Some("bank_a"),
                &payload("well known finding"),
                90.0,
                &CandidateScores::default(),
            )
            .unwrap();

        assert_eq!(outcome, CaptureOutcome::ConfidentMatch);
        assert!(KlePendingObservationsRepository::new(&db)
            .list_by_status(ObservationPendingStatus::Pending)
            .unwrap()
            .is_empty());
        // Confidence is still recorded even when nothing is staged.
        let timeline = KleHistoryRepository::new(&db)
            .timeline_for_entry("hash-1")
            .unwrap();
        assert_eq!(timeline.confidence_snapshots.len(), 1);
    }

    #[test]
    fn capture_entry_mines_keywords_only_for_staged_observations() {
        let db = test_db();
        let engine = KleEngine::new(&db);

        engine
            .capture_entry(
                "hash-1",
                None,
                &payload("novel scaffold hazard"),
                10.0,
                &CandidateScores::default(),
            )
            .unwrap();

        let pending_keywords = KlePendingKeywordsRepository::new(&db)
            .list_by_status(aditup_db::repositories::kle_pending_terms::TermPendingStatus::Pending)
            .unwrap();
        assert!(pending_keywords.iter().any(|k| k.term == "scaffold"));
    }

    #[test]
    fn ranking_weights_defaults_until_feedback_is_recorded() {
        let db = test_db();
        let engine = KleEngine::new(&db);
        assert_eq!(engine.ranking_weights().unwrap(), RankingWeights::default());
    }

    #[test]
    fn log_selection_persists_adjusted_weights_for_the_next_reload() {
        let db = test_db();
        let engine = KleEngine::new(&db);
        let scores = CandidateScores {
            fuzzy_similarity: 0.9,
            ..CandidateScores::default()
        };

        let returned = engine
            .log_selection("hash-1", Some("bank_a"), &scores)
            .unwrap();
        let reloaded = engine.ranking_weights().unwrap();

        // JSON round-trips an f64 losslessly in value but not always in its
        // very last bit of string representation, so compare numerically.
        assert!((returned.fuzzy_similarity - reloaded.fuzzy_similarity).abs() < 1e-12);
        assert!(reloaded.fuzzy_similarity > RankingWeights::default().fuzzy_similarity);
    }

    #[test]
    fn log_rejection_persists_weights_moved_in_the_opposite_direction() {
        let db = test_db();
        let engine = KleEngine::new(&db);
        let scores = CandidateScores {
            keyword_overlap: 0.9,
            ..CandidateScores::default()
        };

        let adjusted = engine
            .log_rejection("hash-1", Some("bank_a"), &scores)
            .unwrap();
        assert!(adjusted.keyword_overlap < RankingWeights::default().keyword_overlap);
    }

    #[test]
    fn resolve_pending_approve_logs_an_approval_event() {
        let db = test_db();
        let engine = KleEngine::new(&db);
        let staged = engine
            .capture_entry(
                "hash-1",
                None,
                &payload("novel finding"),
                10.0,
                &CandidateScores::default(),
            )
            .unwrap();
        let CaptureOutcome::Staged(id) = staged else {
            panic!("expected staged")
        };

        engine
            .resolve_pending(id, PendingDecision::Approve, Some("admin-1"))
            .unwrap();

        let pending = KlePendingObservationsRepository::new(&db)
            .get(id)
            .unwrap()
            .unwrap();
        assert_eq!(pending.status, ObservationPendingStatus::Approved);
        let timeline = KleHistoryRepository::new(&db)
            .timeline_for_entry("hash-1")
            .unwrap();
        assert_eq!(timeline.approvals.len(), 1);
        assert_eq!(
            timeline.approvals[0].admin_user_id.as_deref(),
            Some("admin-1")
        );
    }

    #[test]
    fn resolve_pending_reject_does_not_log_an_approval_event() {
        let db = test_db();
        let engine = KleEngine::new(&db);
        let staged = engine
            .capture_entry(
                "hash-1",
                None,
                &payload("novel finding"),
                10.0,
                &CandidateScores::default(),
            )
            .unwrap();
        let CaptureOutcome::Staged(id) = staged else {
            panic!("expected staged")
        };

        engine
            .resolve_pending(id, PendingDecision::Reject, None)
            .unwrap();

        let pending = KlePendingObservationsRepository::new(&db)
            .get(id)
            .unwrap()
            .unwrap();
        assert_eq!(pending.status, ObservationPendingStatus::Rejected);
        let timeline = KleHistoryRepository::new(&db)
            .timeline_for_entry("hash-1")
            .unwrap();
        assert!(timeline.approvals.is_empty());
    }

    #[test]
    fn resolve_pending_on_an_already_resolved_id_errors() {
        let db = test_db();
        let engine = KleEngine::new(&db);
        let staged = engine
            .capture_entry(
                "hash-1",
                None,
                &payload("novel finding"),
                10.0,
                &CandidateScores::default(),
            )
            .unwrap();
        let CaptureOutcome::Staged(id) = staged else {
            panic!("expected staged")
        };
        engine
            .resolve_pending(id, PendingDecision::Approve, None)
            .unwrap();

        let second_attempt = engine.resolve_pending(id, PendingDecision::Approve, None);
        assert!(matches!(second_attempt, Err(KleError::NotPending(_))));
    }

    #[test]
    fn cluster_pending_creates_a_merge_request_for_near_duplicate_observations() {
        let db = test_db();
        let engine = KleEngine::new(&db);
        engine
            .capture_entry(
                "hash-1",
                None,
                &payload("fire extinguisher missing pressure gauge cover"),
                10.0,
                &CandidateScores::default(),
            )
            .unwrap();
        engine
            .capture_entry(
                "hash-2",
                None,
                &payload("fire extinguisher missing pressure gauge cap"),
                10.0,
                &CandidateScores::default(),
            )
            .unwrap();
        engine
            .capture_entry(
                "hash-3",
                None,
                &payload("scaffold missing toe board entirely"),
                10.0,
                &CandidateScores::default(),
            )
            .unwrap();

        let requests = engine.cluster_pending(3, 0.6).unwrap();

        assert_eq!(requests.len(), 1);
        assert_eq!(
            KleMergeRequestsRepository::new(&db)
                .list_by_status(MergeRequestStatus::Pending)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn resolve_merge_request_merge_folds_candidate_b_away() {
        let db = test_db();
        let engine = KleEngine::new(&db);
        let a = KlePendingObservationsRepository::new(&db)
            .upsert("hash-a", None, "{}")
            .unwrap();
        let b = KlePendingObservationsRepository::new(&db)
            .upsert("hash-b", None, "{}")
            .unwrap();
        let request = KleMergeRequestsRepository::new(&db)
            .create_if_absent(a.id, b.id, 0.9)
            .unwrap();

        engine
            .resolve_merge_request(request.id, MergeDecision::Merge)
            .unwrap();

        let repo = KlePendingObservationsRepository::new(&db);
        assert_eq!(
            repo.get(b.id).unwrap().unwrap().status,
            ObservationPendingStatus::Merged
        );
        assert_eq!(
            repo.get(a.id).unwrap().unwrap().status,
            ObservationPendingStatus::Pending
        );
    }

    #[test]
    fn resolve_merge_request_discard_rejects_both_candidates() {
        let db = test_db();
        let engine = KleEngine::new(&db);
        let a = KlePendingObservationsRepository::new(&db)
            .upsert("hash-a", None, "{}")
            .unwrap();
        let b = KlePendingObservationsRepository::new(&db)
            .upsert("hash-b", None, "{}")
            .unwrap();
        let request = KleMergeRequestsRepository::new(&db)
            .create_if_absent(a.id, b.id, 0.9)
            .unwrap();

        engine
            .resolve_merge_request(request.id, MergeDecision::Discard)
            .unwrap();

        let repo = KlePendingObservationsRepository::new(&db);
        assert_eq!(
            repo.get(a.id).unwrap().unwrap().status,
            ObservationPendingStatus::Rejected
        );
        assert_eq!(
            repo.get(b.id).unwrap().unwrap().status,
            ObservationPendingStatus::Rejected
        );
    }

    #[test]
    fn commit_publish_requires_approved_status() {
        let db = test_db();
        let engine = KleEngine::new(&db);
        let staged = engine
            .capture_entry(
                "hash-1",
                None,
                &payload("novel finding"),
                10.0,
                &CandidateScores::default(),
            )
            .unwrap();
        let CaptureOutcome::Staged(id) = staged else {
            panic!("expected staged")
        };

        let result = engine.commit_publish(id, "bank_a", "Topic", "Label");
        assert!(matches!(result, Err(KleError::NotApproved(_))));
    }

    #[test]
    fn commit_publish_creates_the_official_row_and_marks_pending_merged() {
        let db = test_db();
        let engine = KleEngine::new(&db);
        let staged = engine
            .capture_entry(
                "hash-1",
                None,
                &payload("brand new hazard description"),
                10.0,
                &CandidateScores::default(),
            )
            .unwrap();
        let CaptureOutcome::Staged(id) = staged else {
            panic!("expected staged")
        };
        engine
            .resolve_pending(id, PendingDecision::Approve, Some("admin-1"))
            .unwrap();

        let entry = engine
            .commit_publish(id, "bank_a", "Topic", "New hazard")
            .unwrap();

        assert_eq!(entry.bank_key, "bank_a");
        assert_eq!(entry.text, "brand new hazard description");
        assert_eq!(
            KlePendingObservationsRepository::new(&db)
                .get(id)
                .unwrap()
                .unwrap()
                .status,
            ObservationPendingStatus::Merged
        );
        assert_eq!(
            ObservationBankRepository::new(&db)
                .list_for_bank("bank_a")
                .unwrap()
                .len(),
            1
        );
    }
}
