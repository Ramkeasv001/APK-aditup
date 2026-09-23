use aditup_scoring_engine::{CandidateScores, ConfidenceBand, StructuredTags};
use serde::{Deserialize, Serialize};

/// What Stage 3 asks a `RecommendationProvider` to match against. `bank_keys`
/// scopes the search to specific datasets (e.g. the standards relevant to
/// this audit's category); left empty, every published `observation_bank`
/// dataset is searched.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationQuery {
    pub text: String,
    pub tags: StructuredTags,
    pub bank_keys: Vec<String>,
}

/// One recommendation text linked to a matched observation, ready to show
/// under a Stage 3 candidate card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationText {
    pub id: String,
    pub text: String,
}

/// A ranked candidate observation from the bank, with its confidence and
/// the full sub-score breakdown — every suggestion this engine makes must
/// be explainable in terms of a visible score, per the product vision; this
/// is what makes that possible instead of a black-box percentage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedCandidate {
    pub observation_bank_id: String,
    pub bank_key: String,
    pub topic: String,
    pub label: String,
    pub text: String,
    pub recommendations: Vec<RecommendationText>,
    pub confidence_percent: f64,
    pub confidence_band: ConfidenceBand,
    pub scores: CandidateScores,
}

/// A ranked legal clause suggestion — a lighter-weight match (keyword
/// overlap + fuzzy phrase similarity only; there's no per-clause synonym or
/// structured-tag data to draw on the way there is for observations).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedLegalClause {
    pub legal_bank_id: String,
    pub standard: String,
    pub clause: String,
    pub text: String,
    pub confidence_percent: f64,
}
