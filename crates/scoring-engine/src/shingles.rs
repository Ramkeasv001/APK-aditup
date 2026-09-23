use std::collections::HashSet;

/// Default shingle width (word trigrams) used for near-duplicate detection,
/// per §7 — three consecutive tokens is enough to distinguish "fire
/// extinguisher missing seal" from "fire extinguisher missing pin" while
/// still tolerating minor rewording elsewhere in a longer document.
pub const DEFAULT_SHINGLE_SIZE: usize = 3;

/// Default cosine-similarity threshold above which two documents are
/// treated as probable duplicates (§7) — flagged for human merge-resolution
/// (Module 9), never auto-merged.
pub const DEFAULT_DUPLICATE_THRESHOLD: f64 = 0.85;

/// Word-level n-gram shingles: every run of `shingle_size` consecutive
/// tokens, joined by a space. Documents shorter than `shingle_size` fall
/// back to a single shingle of their full token sequence, so short entries
/// still produce a non-empty set instead of comparing as trivially
/// dissimilar to everything.
pub fn shingles(tokens: &[String], shingle_size: usize) -> HashSet<String> {
    let size = shingle_size.max(1);
    if tokens.is_empty() {
        return HashSet::new();
    }
    if tokens.len() <= size {
        return HashSet::from([tokens.join(" ")]);
    }
    tokens.windows(size).map(|w| w.join(" ")).collect()
}

/// Cosine similarity between two shingle sets, treated as binary
/// (presence/absence) vectors over the shared shingle vocabulary:
/// `|A ∩ B| / sqrt(|A| * |B|)`.
pub fn cosine_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f64;
    intersection / ((a.len() as f64).sqrt() * (b.len() as f64).sqrt())
}

/// Convenience wrapper: shingles both token lists at `shingle_size` and
/// checks whether their cosine similarity meets `threshold`.
pub fn is_probable_duplicate(
    a_tokens: &[String],
    b_tokens: &[String],
    shingle_size: usize,
    threshold: f64,
) -> bool {
    let a = shingles(a_tokens, shingle_size);
    let b = shingles(b_tokens, shingle_size);
    cosine_similarity(&a, &b) >= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn shingles_of_a_short_document_fall_back_to_one_shingle() {
        let result = shingles(&tokens(&["fire", "alarm"]), 3);
        assert_eq!(result.len(), 1);
        assert!(result.contains("fire alarm"));
    }

    #[test]
    fn shingles_of_empty_tokens_is_empty() {
        assert!(shingles(&[], 3).is_empty());
    }

    #[test]
    fn shingles_produces_expected_trigrams() {
        let result = shingles(&tokens(&["a", "b", "c", "d"]), 3);
        assert_eq!(result.len(), 2);
        assert!(result.contains("a b c"));
        assert!(result.contains("b c d"));
    }

    #[test]
    fn identical_documents_have_cosine_similarity_one() {
        let a = shingles(&tokens(&["fire", "extinguisher", "missing", "seal"]), 3);
        let b = shingles(&tokens(&["fire", "extinguisher", "missing", "seal"]), 3);
        // sqrt(n)*sqrt(n) isn't always bit-exact for n, hence the epsilon.
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn unrelated_documents_have_low_cosine_similarity() {
        let a = shingles(&tokens(&["fire", "extinguisher", "missing", "seal"]), 3);
        let b = shingles(&tokens(&["scaffold", "toe", "board", "loose"]), 3);
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn near_duplicate_wording_scores_high_but_not_perfect() {
        // Same document except the last word — 3 of 4 trigrams still match.
        let a = shingles(
            &tokens(&[
                "fire",
                "extinguisher",
                "missing",
                "pressure",
                "gauge",
                "cover",
            ]),
            3,
        );
        let b = shingles(
            &tokens(&[
                "fire",
                "extinguisher",
                "missing",
                "pressure",
                "gauge",
                "cap",
            ]),
            3,
        );
        let similarity = cosine_similarity(&a, &b);
        assert!(similarity > 0.6 && similarity < 1.0);
    }

    #[test]
    fn is_probable_duplicate_respects_the_threshold() {
        let identical_a = tokens(&["fire", "extinguisher", "missing", "seal"]);
        let identical_b = tokens(&["fire", "extinguisher", "missing", "seal"]);
        let unrelated = tokens(&["scaffold", "toe", "board", "loose"]);

        assert!(is_probable_duplicate(
            &identical_a,
            &identical_b,
            DEFAULT_SHINGLE_SIZE,
            DEFAULT_DUPLICATE_THRESHOLD
        ));
        assert!(!is_probable_duplicate(
            &identical_a,
            &unrelated,
            DEFAULT_SHINGLE_SIZE,
            DEFAULT_DUPLICATE_THRESHOLD
        ));
    }

    #[test]
    fn cosine_similarity_with_an_empty_set_is_zero_not_a_panic() {
        let a: HashSet<String> = HashSet::new();
        let b = shingles(&tokens(&["fire", "alarm"]), 3);
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }
}
