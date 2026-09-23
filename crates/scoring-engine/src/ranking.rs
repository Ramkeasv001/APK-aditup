use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// Weights for each ranking sub-score, per §7 of the architecture doc.
/// Configurable per deployment (Module 14 Settings) — the feedback loop
/// (Module 9) is the primary thing expected to nudge these over time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RankingWeights {
    pub keyword_overlap: f64,
    pub tfidf: f64,
    pub fuzzy_similarity: f64,
    pub synonym_contribution: f64,
    pub structured_match: f64,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            keyword_overlap: 0.30,
            tfidf: 0.25,
            fuzzy_similarity: 0.15,
            synonym_contribution: 0.10,
            structured_match: 0.20,
        }
    }
}

/// The five independently-computed `[0, 1]` sub-scores for one candidate,
/// ready to combine via `weighted_score`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CandidateScores {
    pub keyword_overlap: f64,
    pub tfidf: f64,
    pub fuzzy_similarity: f64,
    pub synonym_contribution: f64,
    pub structured_match: f64,
}

/// Weighted sum of the five sub-scores, clamped to `[0, 1]` — the number
/// `confidence_percent` rescales into a badge.
pub fn weighted_score(scores: &CandidateScores, weights: &RankingWeights) -> f64 {
    (scores.keyword_overlap * weights.keyword_overlap
        + scores.tfidf * weights.tfidf
        + scores.fuzzy_similarity * weights.fuzzy_similarity
        + scores.synonym_contribution * weights.synonym_contribution
        + scores.structured_match * weights.structured_match)
        .clamp(0.0, 1.0)
}

/// Fraction of distinct query tokens present in the candidate's tokens —
/// the "keyword overlap" component of the ranking formula.
pub fn keyword_overlap_score(query_tokens: &[String], doc_tokens: &[String]) -> f64 {
    let mut distinct_query: Vec<&str> = query_tokens.iter().map(String::as_str).collect();
    distinct_query.sort_unstable();
    distinct_query.dedup();
    if distinct_query.is_empty() {
        return 0.0;
    }

    let doc_set: HashSet<&str> = doc_tokens.iter().map(String::as_str).collect();
    let overlap = distinct_query
        .iter()
        .filter(|t| doc_set.contains(*t))
        .count();
    overlap as f64 / distinct_query.len() as f64
}

/// A Stage 2 candidate's structured classification. Fields the auditor left
/// unset (`None`) don't count for or against a match — only tags actually
/// asserted on the query side are compared.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredTags {
    pub category: Option<String>,
    pub department: Option<String>,
    pub equipment: Option<String>,
}

/// Fraction of the query's *asserted* structured tags that match the
/// candidate's corresponding tag. `0.0` if the query asserted nothing to
/// compare (rather than treating "no assertion" as a perfect or zero
/// match).
pub fn structured_match_score(query: &StructuredTags, candidate: &StructuredTags) -> f64 {
    let pairs = [
        (&query.category, &candidate.category),
        (&query.department, &candidate.department),
        (&query.equipment, &candidate.equipment),
    ];
    let asserted: Vec<_> = pairs.iter().filter(|(q, _)| q.is_some()).collect();
    if asserted.is_empty() {
        return 0.0;
    }
    let matches = asserted
        .iter()
        .filter(|(q, c)| q.as_deref() == c.as_deref())
        .count();
    matches as f64 / asserted.len() as f64
}

/// The three confidence bands a candidate's percent score falls into,
/// matching the badges shown in Stage 3 (§4/§7): `<50%` low, `50-74%`
/// medium, `>=75%` high.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceBand {
    Low,
    Medium,
    High,
}

/// Below this percent, Stage 3 stops suggesting entirely and routes the
/// entry to "write your own / escalate to Admin as new" — i.e. into KLE
/// staging (Module 9) — rather than showing a low-confidence guess.
pub const NOVELTY_THRESHOLD_PERCENT: f64 = 40.0;

pub fn confidence_percent(weighted: f64) -> f64 {
    (weighted * 100.0).clamp(0.0, 100.0)
}

pub fn confidence_band(percent: f64) -> ConfidenceBand {
    if percent >= 75.0 {
        ConfidenceBand::High
    } else if percent >= 50.0 {
        ConfidenceBand::Medium
    } else {
        ConfidenceBand::Low
    }
}

pub fn is_below_novelty_threshold(percent: f64) -> bool {
    percent < NOVELTY_THRESHOLD_PERCENT
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn keyword_overlap_is_full_when_every_query_term_is_present() {
        let query = tokens(&["fire", "extinguisher"]);
        let doc = tokens(&["fire", "extinguisher", "missing", "seal"]);
        assert_eq!(keyword_overlap_score(&query, &doc), 1.0);
    }

    #[test]
    fn keyword_overlap_is_partial() {
        let query = tokens(&["fire", "extinguisher", "gauge"]);
        let doc = tokens(&["fire", "extinguisher", "missing"]);
        assert!((keyword_overlap_score(&query, &doc) - (2.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn keyword_overlap_of_empty_query_is_zero() {
        assert_eq!(keyword_overlap_score(&[], &tokens(&["fire"])), 0.0);
    }

    #[test]
    fn structured_match_ignores_unasserted_query_fields() {
        let query = StructuredTags {
            category: Some("Fire Protection".to_string()),
            department: None,
            equipment: None,
        };
        let candidate = StructuredTags {
            category: Some("Fire Protection".to_string()),
            department: Some("Facilities".to_string()),
            equipment: None,
        };
        assert_eq!(structured_match_score(&query, &candidate), 1.0);
    }

    #[test]
    fn structured_match_is_partial_across_asserted_fields() {
        let query = StructuredTags {
            category: Some("Fire Protection".to_string()),
            department: Some("Facilities".to_string()),
            equipment: None,
        };
        let candidate = StructuredTags {
            category: Some("Fire Protection".to_string()),
            department: Some("Operations".to_string()),
            equipment: None,
        };
        assert_eq!(structured_match_score(&query, &candidate), 0.5);
    }

    #[test]
    fn structured_match_with_nothing_asserted_is_zero() {
        assert_eq!(
            structured_match_score(&StructuredTags::default(), &StructuredTags::default()),
            0.0
        );
    }

    #[test]
    fn weighted_score_of_a_perfect_candidate_is_one() {
        let scores = CandidateScores {
            keyword_overlap: 1.0,
            tfidf: 1.0,
            fuzzy_similarity: 1.0,
            synonym_contribution: 1.0,
            structured_match: 1.0,
        };
        assert_eq!(weighted_score(&scores, &RankingWeights::default()), 1.0);
    }

    #[test]
    fn weighted_score_of_a_hopeless_candidate_is_zero() {
        let scores = CandidateScores::default();
        assert_eq!(weighted_score(&scores, &RankingWeights::default()), 0.0);
    }

    #[test]
    fn weighted_score_respects_relative_weights() {
        let heavy_on_keyword_overlap = CandidateScores {
            keyword_overlap: 1.0,
            ..CandidateScores::default()
        };
        let heavy_on_synonym = CandidateScores {
            synonym_contribution: 1.0,
            ..CandidateScores::default()
        };
        let weights = RankingWeights::default();
        // keyword_overlap (0.30) outweighs synonym_contribution (0.10).
        assert!(
            weighted_score(&heavy_on_keyword_overlap, &weights)
                > weighted_score(&heavy_on_synonym, &weights)
        );
    }

    #[test]
    fn confidence_bands_match_the_documented_thresholds() {
        assert_eq!(confidence_band(90.0), ConfidenceBand::High);
        assert_eq!(confidence_band(75.0), ConfidenceBand::High);
        assert_eq!(confidence_band(74.9), ConfidenceBand::Medium);
        assert_eq!(confidence_band(50.0), ConfidenceBand::Medium);
        assert_eq!(confidence_band(49.9), ConfidenceBand::Low);
        assert_eq!(confidence_band(0.0), ConfidenceBand::Low);
    }

    #[test]
    fn novelty_threshold_gates_low_scores() {
        assert!(is_below_novelty_threshold(39.9));
        assert!(!is_below_novelty_threshold(40.0));
    }

    #[test]
    fn confidence_percent_clamps_to_the_valid_range() {
        assert_eq!(confidence_percent(1.5), 100.0);
        assert_eq!(confidence_percent(-0.5), 0.0);
        assert_eq!(confidence_percent(0.75), 75.0);
    }
}
