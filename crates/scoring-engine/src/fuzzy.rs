/// Classic Levenshtein edit distance (single-character insert/delete/
/// substitute) between two strings, computed over Unicode scalar values
/// with a rolling two-row DP table (O(min(n,m)) space).
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();

    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut previous_row: Vec<usize> = (0..=b.len()).collect();
    let mut current_row = vec![0usize; b.len() + 1];

    for (i, &a_char) in a.iter().enumerate() {
        current_row[0] = i + 1;
        for (j, &b_char) in b.iter().enumerate() {
            let cost = if a_char == b_char { 0 } else { 1 };
            current_row[j + 1] = (previous_row[j + 1] + 1) // deletion
                .min(current_row[j] + 1) // insertion
                .min(previous_row[j] + cost); // substitution
        }
        std::mem::swap(&mut previous_row, &mut current_row);
    }

    previous_row[b.len()]
}

/// Normalized similarity in `[0, 1]`: `1.0` for identical strings, `0.0`
/// when the edit distance equals the longer string's full length.
pub fn levenshtein_similarity(a: &str, b: &str) -> f64 {
    let max_len = a.chars().count().max(b.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    1.0 - (levenshtein_distance(a, b) as f64 / max_len as f64)
}

/// Fuzzy similarity between two already-tokenized phrases — joins each
/// back into a space-separated string and compares those, so a
/// misspelling or word-order shuffle degrades the score gracefully instead
/// of causing a total (0 or 1) keyword-overlap miss.
pub fn fuzzy_similarity(query_tokens: &[String], doc_tokens: &[String]) -> f64 {
    levenshtein_similarity(&query_tokens.join(" "), &doc_tokens.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn identical_strings_have_zero_distance() {
        assert_eq!(levenshtein_distance("scaffold", "scaffold"), 0);
    }

    #[test]
    fn single_substitution_costs_one() {
        assert_eq!(levenshtein_distance("scaffold", "scaffolp"), 1);
    }

    #[test]
    fn single_insertion_costs_one() {
        assert_eq!(levenshtein_distance("cat", "cats"), 1);
    }

    #[test]
    fn empty_string_distance_equals_other_length() {
        assert_eq!(levenshtein_distance("", "scaffold"), 8);
        assert_eq!(levenshtein_distance("scaffold", ""), 8);
    }

    #[test]
    fn similarity_of_identical_strings_is_one() {
        assert_eq!(levenshtein_similarity("scaffold", "scaffold"), 1.0);
    }

    #[test]
    fn similarity_of_completely_different_strings_is_low() {
        assert!(levenshtein_similarity("scaffold", "extinguisher") < 0.3);
    }

    #[test]
    fn similarity_of_two_empty_strings_is_one_not_a_divide_by_zero() {
        assert_eq!(levenshtein_similarity("", ""), 1.0);
    }

    #[test]
    fn a_misspelling_scores_high_but_not_perfect() {
        let similarity = levenshtein_similarity("extinguisher", "extinguisher"); // exact
        let misspelled = levenshtein_similarity("extinguisher", "extinguiser"); // missing a letter
        assert_eq!(similarity, 1.0);
        assert!(misspelled > 0.85 && misspelled < 1.0);
    }

    #[test]
    fn fuzzy_similarity_operates_on_the_joined_phrase() {
        let query = tokens(&["fire", "extinguisher", "missing"]);
        let close_match = tokens(&["fire", "extinguisher", "missing", "seal"]);
        let unrelated = tokens(&["scaffold", "toe", "board"]);

        assert!(fuzzy_similarity(&query, &close_match) > fuzzy_similarity(&query, &unrelated));
    }
}
