use std::collections::HashSet;
use std::sync::LazyLock;

/// General-English stopwords stripped before indexing/scoring — not
/// HSE-specific, just the words too common to carry any discriminating
/// weight in a TF-IDF sense (§8).
pub static STOPWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "at", "in", "on",
        "of", "to", "for", "and", "or", "nor", "with", "without", "that", "this", "these", "those",
        "has", "have", "had", "it", "its", "as", "by", "from", "not", "no", "so", "than", "then",
        "there", "their", "they", "them", "but", "if", "into", "onto", "over", "under", "near",
        "when", "while", "which", "who", "whom", "what", "where", "why", "how", "can", "could",
        "should", "would", "will", "shall", "may", "might", "do", "does", "did", "done", "also",
        "any", "all", "some", "such", "only", "own", "same", "very", "too", "just", "here", "out",
        "up", "down", "off", "again", "further", "once", "both", "each", "more", "most", "other",
        "because", "before", "after", "above", "below", "between",
    ]
    .into_iter()
    .collect()
});

pub fn is_stopword(token: &str) -> bool {
    STOPWORDS.contains(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_words_are_stopwords() {
        assert!(is_stopword("the"));
        assert!(is_stopword("without"));
    }

    #[test]
    fn domain_terms_are_not_stopwords() {
        assert!(!is_stopword("extinguisher"));
        assert!(!is_stopword("scaffold"));
    }
}
