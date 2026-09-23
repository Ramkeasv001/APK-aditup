//! The Self-Learning Engine — Module 9 of the Phase 2 build (see
//! docs/architecture/phase-1-blueprint.md §2, §6, §8).
//!
//! `KleEngine` closes the continuous-learning loop: an observation is
//! captured with its confidence score (always logged); if it's below the
//! novelty threshold, it's staged for Admin review and its terms are mined
//! as candidate keywords. Every Stage 3 search/selection/rejection/edit is
//! logged against the observation's stable `entry_hash`, and
//! selections/rejections nudge the ranking weights `aditup-scoring-engine`
//! computes with (persisted for the next reload, per §8 — never live
//! mid-session). Admin resolves pending observations and merge-request
//! duplicates; approved ones publish into `observation_bank` (Module 4)
//! through `commit_publish`.

mod engine;
mod error;
mod types;

pub use engine::KleEngine;
pub use error::KleError;
pub use types::{
    CaptureOutcome, ConfidenceBreakdown, MergeDecision, PendingDecision, PendingObservationPayload,
};
