use std::collections::{HashMap, HashSet};

/// Maps each term to every document containing it. Built once per bank
/// load (§7) — not per query — so a query only has to look up its own
/// terms' postings instead of scanning every document in the bank, which is
/// what keeps search fast at the "10,000 rows, <150ms" scale from §3.
pub struct InvertedIndex {
    postings: HashMap<String, HashSet<String>>,
}

impl InvertedIndex {
    /// `documents` is `(doc_id, tokens)` pairs — typically doc_id is an
    /// `observation_bank`/`recommendation_bank` row id and tokens is the
    /// output of `preprocessing::preprocess` over that row's text.
    pub fn build(documents: &[(String, Vec<String>)]) -> Self {
        let mut postings: HashMap<String, HashSet<String>> = HashMap::new();
        for (doc_id, tokens) in documents {
            for term in tokens {
                postings
                    .entry(term.clone())
                    .or_default()
                    .insert(doc_id.clone());
            }
        }
        Self { postings }
    }

    /// Every document containing at least one query term — the candidate
    /// set actually worth scoring in detail (via TF-IDF/fuzzy/ranking),
    /// instead of every document in the bank.
    pub fn candidates(&self, query_tokens: &[String]) -> HashSet<String> {
        let mut result = HashSet::new();
        for term in query_tokens {
            if let Some(docs) = self.postings.get(term) {
                result.extend(docs.iter().cloned());
            }
        }
        result
    }

    pub fn document_frequency(&self, term: &str) -> usize {
        self.postings.get(term).map(HashSet::len).unwrap_or(0)
    }

    pub fn term_count(&self) -> usize {
        self.postings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(id: &str, words: &[&str]) -> (String, Vec<String>) {
        (
            id.to_string(),
            words.iter().map(|w| w.to_string()).collect(),
        )
    }

    #[test]
    fn candidates_returns_only_documents_sharing_a_query_term() {
        let index = InvertedIndex::build(&[
            doc("a", &["fire", "extinguisher", "missing"]),
            doc("b", &["scaffold", "toe", "board"]),
            doc("c", &["fire", "alarm", "silent"]),
        ]);

        let query = vec!["fire".to_string()];
        let candidates = index.candidates(&query);

        assert_eq!(candidates.len(), 2);
        assert!(candidates.contains("a"));
        assert!(candidates.contains("c"));
        assert!(!candidates.contains("b"));
    }

    #[test]
    fn candidates_unions_across_multiple_query_terms() {
        let index = InvertedIndex::build(&[
            doc("a", &["fire", "extinguisher"]),
            doc("b", &["scaffold", "board"]),
        ]);

        let query = vec!["fire".to_string(), "board".to_string()];
        let candidates = index.candidates(&query);

        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn unknown_query_term_yields_no_candidates() {
        let index = InvertedIndex::build(&[doc("a", &["fire"])]);
        let candidates = index.candidates(&["hydrant".to_string()]);
        assert!(candidates.is_empty());
    }

    #[test]
    fn document_frequency_counts_distinct_documents_not_occurrences() {
        let index = InvertedIndex::build(&[
            doc("a", &["fire", "fire", "fire"]),
            doc("b", &["fire"]),
            doc("c", &["scaffold"]),
        ]);
        assert_eq!(index.document_frequency("fire"), 2);
        assert_eq!(index.document_frequency("scaffold"), 1);
        assert_eq!(index.document_frequency("hydrant"), 0);
    }
}
