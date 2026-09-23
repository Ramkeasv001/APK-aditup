//! The `RecommendationProvider` seam and Mode 1's rule-based implementation
//! — Module 8 of the Phase 2 build (see
//! docs/architecture/phase-1-blueprint.md §7, §8, §14).
//!
//! This is where Modules 4-7 actually meet real data: `RuleBasedProvider`
//! loads `observation_bank`/`recommendation_bank`/`synonym_bank` rows,
//! builds an index and TF-IDF corpus over them, and combines the five §7
//! sub-scores into ranked, confidence-banded candidates. Modes 2/3 (cloud/
//! local LLM agentic RAG) implement the same `RecommendationProvider` trait
//! later — Stage 3, the schema, KLE, and export/analytics don't change when
//! that happens.

mod error;
mod provider;
mod rule_based;
mod types;

pub use error::ProviderError;
pub use provider::RecommendationProvider;
pub use rule_based::RuleBasedProvider;
pub use types::{ObservationQuery, RankedCandidate, RankedLegalClause, RecommendationText};
