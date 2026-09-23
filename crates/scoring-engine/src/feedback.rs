use crate::ranking::{CandidateScores, RankingWeights};

/// Whether a Stage 3 decision confirmed or contradicted the ranking that
/// produced it (§8: "selections nudge the responsible sub-score's weight
/// up slightly... rejections nudge it down").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackDirection {
    /// The auditor selected this candidate — whichever sub-scores drove
    /// its ranking should be trusted a little more next time.
    Positive,
    /// The auditor rejected this candidate despite it ranking highly —
    /// whichever sub-scores drove its ranking should be trusted a little
    /// less next time.
    Negative,
}

/// Fixed per-event adjustment step. Deliberately small and constant (not a
/// gradient-descent rate) so no single session can distort ranking — §8 is
/// explicit that this must be a bounded, slowly-decaying adjustment.
const WEIGHT_STEP: f64 = 0.01;

/// A sub-score below this value wasn't a meaningful contributor to the
/// outcome either way, so its weight isn't adjusted for this event.
const RESPONSIBILITY_THRESHOLD: f64 = 0.5;

/// Bounds each event's raw pre-normalization step, so one event can't move
/// a sub-score by more than a step past these values. Under realistic,
/// mixed feedback (different observations pulling different sub-scores in
/// different directions) the renormalized share this produces stays close
/// to these bounds. This is *not* a hard ceiling on the final normalized
/// share under a long, perfectly one-sided run of identical events —
/// renormalization can let a dominant sub-score's share drift past
/// `MAX_WEIGHT` in that degenerate case, because pulling every other
/// weight down (never below their own last value, since only the
/// "responsible" weight moves each event) shrinks the denominator faster
/// than the clamp shrinks the numerator. A full bounded-simplex projection
/// would close that gap; given feedback in practice arrives mixed across
/// many topics rather than as hundreds of identical events in a row, that
/// complexity isn't justified here.
const MIN_WEIGHT: f64 = 0.05;
const MAX_WEIGHT: f64 = 0.60;

/// Nudges whichever sub-score weights were "responsible" for `scores`
/// (i.e. scored above `RESPONSIBILITY_THRESHOLD`) one step in the
/// direction implied by `direction`, then renormalizes so the five weights
/// still sum to `1.0`. Applied once per Stage 3 selection/rejection event;
/// per §8, the *result* only takes effect on the next bank/index reload,
/// not live mid-session — this function only computes the new weights, it
/// doesn't decide when they're loaded.
pub fn adjust_for_feedback(
    weights: &RankingWeights,
    scores: &CandidateScores,
    direction: FeedbackDirection,
) -> RankingWeights {
    let step = match direction {
        FeedbackDirection::Positive => WEIGHT_STEP,
        FeedbackDirection::Negative => -WEIGHT_STEP,
    };

    let mut adjusted = *weights;
    if scores.keyword_overlap > RESPONSIBILITY_THRESHOLD {
        adjusted.keyword_overlap = (adjusted.keyword_overlap + step).clamp(MIN_WEIGHT, MAX_WEIGHT);
    }
    if scores.tfidf > RESPONSIBILITY_THRESHOLD {
        adjusted.tfidf = (adjusted.tfidf + step).clamp(MIN_WEIGHT, MAX_WEIGHT);
    }
    if scores.fuzzy_similarity > RESPONSIBILITY_THRESHOLD {
        adjusted.fuzzy_similarity =
            (adjusted.fuzzy_similarity + step).clamp(MIN_WEIGHT, MAX_WEIGHT);
    }
    if scores.synonym_contribution > RESPONSIBILITY_THRESHOLD {
        adjusted.synonym_contribution =
            (adjusted.synonym_contribution + step).clamp(MIN_WEIGHT, MAX_WEIGHT);
    }
    if scores.structured_match > RESPONSIBILITY_THRESHOLD {
        adjusted.structured_match =
            (adjusted.structured_match + step).clamp(MIN_WEIGHT, MAX_WEIGHT);
    }

    normalize(&adjusted)
}

/// Rescales the five weights to sum to `1.0`, preserving their relative
/// proportions. Falls back to the documented §7 defaults in the
/// pathological case where every weight has been clamped to `0.0` (not
/// reachable via `adjust_for_feedback` given `MIN_WEIGHT > 0.0`, but this
/// keeps the function total rather than assuming that invariant forever).
fn normalize(weights: &RankingWeights) -> RankingWeights {
    let sum = weights.keyword_overlap
        + weights.tfidf
        + weights.fuzzy_similarity
        + weights.synonym_contribution
        + weights.structured_match;

    if sum <= 0.0 {
        return RankingWeights::default();
    }

    RankingWeights {
        keyword_overlap: weights.keyword_overlap / sum,
        tfidf: weights.tfidf / sum,
        fuzzy_similarity: weights.fuzzy_similarity / sum,
        synonym_contribution: weights.synonym_contribution / sum,
        structured_match: weights.structured_match / sum,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sum_of(weights: &RankingWeights) -> f64 {
        weights.keyword_overlap
            + weights.tfidf
            + weights.fuzzy_similarity
            + weights.synonym_contribution
            + weights.structured_match
    }

    #[test]
    fn positive_feedback_increases_the_responsible_weight() {
        let weights = RankingWeights::default();
        let scores = CandidateScores {
            fuzzy_similarity: 0.9,
            ..CandidateScores::default()
        };

        let adjusted = adjust_for_feedback(&weights, &scores, FeedbackDirection::Positive);

        // Relative to the default, fuzzy_similarity's share should have grown.
        assert!(adjusted.fuzzy_similarity / sum_of(&adjusted) > weights.fuzzy_similarity);
    }

    #[test]
    fn negative_feedback_decreases_the_responsible_weight() {
        let weights = RankingWeights::default();
        let scores = CandidateScores {
            keyword_overlap: 0.9,
            ..CandidateScores::default()
        };

        let adjusted = adjust_for_feedback(&weights, &scores, FeedbackDirection::Negative);

        assert!(adjusted.keyword_overlap / sum_of(&adjusted) < weights.keyword_overlap);
    }

    #[test]
    fn weights_always_sum_to_one_after_adjustment() {
        let weights = RankingWeights::default();
        let scores = CandidateScores {
            keyword_overlap: 0.9,
            tfidf: 0.8,
            ..CandidateScores::default()
        };

        let adjusted = adjust_for_feedback(&weights, &scores, FeedbackDirection::Positive);
        assert!((sum_of(&adjusted) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_sub_score_below_the_responsibility_threshold_is_left_alone() {
        let weights = RankingWeights::default();
        let scores = CandidateScores {
            synonym_contribution: 0.2, // below threshold
            ..CandidateScores::default()
        };

        let adjusted = adjust_for_feedback(&weights, &scores, FeedbackDirection::Positive);
        // Untouched sub-score's *share* shouldn't have grown relative to siblings
        // that also went untouched — check it didn't get the +step treatment by
        // comparing directly against a no-op adjustment (all scores 0).
        let baseline = adjust_for_feedback(
            &weights,
            &CandidateScores::default(),
            FeedbackDirection::Positive,
        );
        assert_eq!(adjusted, baseline);
    }

    #[test]
    fn a_single_event_clamps_the_raw_step_at_the_ceiling_before_normalizing() {
        // The per-event guarantee this module actually makes: clamp the
        // pre-normalization step, not the eventual renormalized share (see
        // the doc comment on MIN_WEIGHT/MAX_WEIGHT for why those differ).
        // Starting at 0.595 + a 0.01 step would reach 0.605 uncapped; the
        // clamp should hold it at exactly 0.60 before the division below.
        let weights_near_ceiling = RankingWeights {
            keyword_overlap: 0.595,
            tfidf: 0.25,
            fuzzy_similarity: 0.15,
            synonym_contribution: 0.10,
            structured_match: 0.20,
        };
        let scores = CandidateScores {
            keyword_overlap: 0.9,
            ..CandidateScores::default()
        };

        let adjusted =
            adjust_for_feedback(&weights_near_ceiling, &scores, FeedbackDirection::Positive);

        let expected = 0.60 / (0.60 + 0.25 + 0.15 + 0.10 + 0.20);
        assert!((adjusted.keyword_overlap - expected).abs() < 1e-9);
    }

    #[test]
    fn repeated_one_sided_feedback_keeps_weights_valid_and_trending_toward_the_signal() {
        let mut weights = RankingWeights::default();
        let scores = CandidateScores {
            keyword_overlap: 0.9,
            ..CandidateScores::default()
        };

        for _ in 0..20 {
            weights = adjust_for_feedback(&weights, &scores, FeedbackDirection::Positive);
        }

        assert!(weights.keyword_overlap > RankingWeights::default().keyword_overlap);
        assert!((sum_of(&weights) - 1.0).abs() < 1e-9);
        assert!(
            weights.tfidf > 0.0
                && weights.fuzzy_similarity > 0.0
                && weights.synonym_contribution > 0.0
        );
    }

    #[test]
    fn multiple_responsible_sub_scores_are_all_adjusted_in_one_event() {
        let weights = RankingWeights::default();
        let scores = CandidateScores {
            keyword_overlap: 0.9,
            tfidf: 0.9,
            fuzzy_similarity: 0.9,
            synonym_contribution: 0.9,
            structured_match: 0.9,
        };

        let adjusted = adjust_for_feedback(&weights, &scores, FeedbackDirection::Positive);
        // Every sub-score moved by the same absolute step before
        // renormalization (a monotonic rescaling), so the §7 priority
        // order among them is preserved even though a flat step compresses
        // the ratios between unequal weights closer together.
        assert!(adjusted.keyword_overlap > adjusted.tfidf);
        assert!(adjusted.tfidf > adjusted.structured_match);
        assert!(adjusted.structured_match > adjusted.fuzzy_similarity);
        assert!(adjusted.fuzzy_similarity > adjusted.synonym_contribution);
    }
}
