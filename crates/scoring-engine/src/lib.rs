//! Text preprocessing, TF-IDF weighting, and search/ranking primitives —
//! Modules 6-7 of the Phase 2 build (see
//! docs/architecture/phase-1-blueprint.md §7-8).
//!
//! This crate has no knowledge of the database or of any specific bank —
//! it operates purely on token lists and plain data. Module 8 (Rule-Based
//! Engine) wires these primitives up against real
//! `observation_bank`/`keyword_bank`/`synonym_bank` content — building the
//! actual index, loading real synonym pairs, and combining sub-scores per
//! observation — into the ranked-candidate pipeline Stage 3 calls. Module 9
//! (KLE) reuses the shingle/cosine duplicate-detection primitives here for
//! clustering pending observations.

pub mod abbreviations;
pub mod feedback;
pub mod fuzzy;
pub mod inverted_index;
pub mod preprocessing;
pub mod ranking;
pub mod shingles;
pub mod stopwords;
pub mod synonyms;
pub mod tf_idf;

pub use abbreviations::normalize_abbreviations;
pub use feedback::{adjust_for_feedback, FeedbackDirection};
pub use fuzzy::{fuzzy_similarity, levenshtein_distance, levenshtein_similarity};
pub use inverted_index::InvertedIndex;
pub use preprocessing::{preprocess, preprocess_default, strip_punctuation, PreprocessConfig};
pub use ranking::{
    confidence_band, confidence_percent, is_below_novelty_threshold, keyword_overlap_score,
    structured_match_score, weighted_score, CandidateScores, ConfidenceBand, RankingWeights,
    StructuredTags, NOVELTY_THRESHOLD_PERCENT,
};
pub use shingles::{
    cosine_similarity, is_probable_duplicate, shingles, DEFAULT_DUPLICATE_THRESHOLD,
    DEFAULT_SHINGLE_SIZE,
};
pub use stopwords::is_stopword;
pub use synonyms::{synonym_contribution_score, SynonymLookup};
pub use tf_idf::{normalized_tfidf_score, term_frequency, tfidf_score, TfIdfCorpus};
