use aditup_scoring_engine::{CandidateScores, StructuredTags};
use serde::{Deserialize, Serialize};

/// What gets JSON-serialized into `kle_pending_observations.payload`. The
/// `db` crate treats this column as an opaque blob — only this crate knows
/// its shape.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PendingObservationPayload {
    pub text: String,
    pub location: Option<String>,
    pub tags: StructuredTags,
}

/// What gets JSON-serialized into `kle_history_confidence.breakdown_json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceBreakdown {
    pub percent: f64,
    pub scores: CandidateScores,
}

/// The result of `KleEngine::capture_entry`.
#[derive(Debug, Clone, PartialEq)]
pub enum CaptureOutcome {
    /// Confidence was at/above the novelty threshold — nothing staged, the
    /// existing bank already covers this observation well enough.
    ConfidentMatch,
    /// Below the novelty threshold — staged (or re-staged) for Admin
    /// review at this pending-observation id.
    Staged(i64),
}

/// An Admin's decision on one pending observation (§4 Admin Console).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingDecision {
    Approve,
    Reject,
}

/// An Admin's decision on one merge request (§4 Admin Console). There is no
/// "merge" primitive that automatically decides which candidate survives —
/// see `KleEngine::resolve_merge_request` for how each variant is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeDecision {
    /// Candidate B is folded away; Candidate A proceeds through the normal
    /// approve/reject flow as the surviving representative.
    Merge,
    /// They're similar but legitimately distinct — leave both as they are.
    KeepBoth,
    /// Neither is worth pursuing — reject both.
    Discard,
}
