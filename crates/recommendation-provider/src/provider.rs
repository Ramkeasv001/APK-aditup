use crate::error::ProviderError;
use crate::types::{ObservationQuery, RankedCandidate};

/// The one interface Stage 3 depends on for "what should I suggest" (§5/§14
/// of the architecture doc). `RuleBasedProvider` is Mode 1's implementation
/// — deterministic scoring over the local encrypted banks, no network, no
/// LLM. Modes 2 (cloud agentic RAG) and 3 (local LLM agentic RAG) implement
/// this same trait against a cloud or on-device model later; Stage 1/2, the
/// schema, KLE, and export/analytics never need to change when that happens
/// — only which implementation of this trait Settings (Module 14) selects
/// at startup.
pub trait RecommendationProvider {
    /// Returns up to `limit` candidates, ranked by descending confidence.
    /// Never errors on "no good match" — a query with nothing relevant in
    /// the bank returns an empty (or all-low-confidence) list; the caller
    /// checks the top result's `confidence_percent` against
    /// `aditup_scoring_engine::NOVELTY_THRESHOLD_PERCENT` to decide whether
    /// to show suggestions at all or route the entry to KLE staging.
    fn suggest(
        &self,
        query: &ObservationQuery,
        limit: usize,
    ) -> Result<Vec<RankedCandidate>, ProviderError>;
}
