use std::collections::HashMap;

/// Inverse-document-frequency table built once over a bank corpus (e.g. all
/// `observation_bank` rows for a given `bank_key`, tokenized). Rebuilding is
/// cheap enough to do whenever the corpus changes (Module 7 rebuilds it on
/// bank reload, not per query) — this type holds no reference back to the
/// corpus itself, just the derived per-term weights.
pub struct TfIdfCorpus {
    /// ln(N / (1 + document_frequency)) + 1 per term — the "+1" smoothing
    /// keeps a term that appears in every document from getting an IDF of
    /// zero and vanishing from scoring entirely.
    idf: HashMap<String, f64>,
    document_count: usize,
}

impl TfIdfCorpus {
    /// Builds the IDF table from a tokenized corpus. Each inner `Vec<String>`
    /// is one document's tokens (typically the output of
    /// `preprocessing::preprocess` over one bank row's text).
    pub fn build(documents: &[Vec<String>]) -> Self {
        let document_count = documents.len();
        let mut document_frequency: HashMap<&str, usize> = HashMap::new();

        for doc in documents {
            let mut seen_in_this_doc: HashMap<&str, bool> = HashMap::new();
            for term in doc {
                if seen_in_this_doc.insert(term.as_str(), true).is_none() {
                    *document_frequency.entry(term.as_str()).or_insert(0) += 1;
                }
            }
        }

        let idf = document_frequency
            .into_iter()
            .map(|(term, df)| {
                let weight = ((document_count as f64) / (1.0 + df as f64)).ln() + 1.0;
                (term.to_string(), weight)
            })
            .collect();

        Self {
            idf,
            document_count,
        }
    }

    /// IDF weight for `term`, or `0.0` if it never appeared in the corpus
    /// this was built from (a term absent from the corpus contributes
    /// nothing to a TF-IDF score, rather than being an error case).
    pub fn idf(&self, term: &str) -> f64 {
        self.idf.get(term).copied().unwrap_or(0.0)
    }

    pub fn document_count(&self) -> usize {
        self.document_count
    }
}

/// Raw term frequency of `term` within `doc_tokens` — how many times it
/// appears, not normalized by document length. Module 7's ranking formula
/// decides how to combine this with IDF and the other sub-scores from §7.
pub fn term_frequency(term: &str, doc_tokens: &[String]) -> usize {
    doc_tokens.iter().filter(|t| t.as_str() == term).count()
}

/// Sum of `tf(term) * idf(term)` over every distinct term in `query_tokens`
/// that also appears in `doc_tokens` — the core TF-IDF relevance score
/// between a query (e.g. a Stage 3 observation) and one candidate document
/// (e.g. one bank row), against a given corpus.
pub fn tfidf_score(query_tokens: &[String], doc_tokens: &[String], corpus: &TfIdfCorpus) -> f64 {
    let mut distinct_query_terms: Vec<&str> = query_tokens.iter().map(String::as_str).collect();
    distinct_query_terms.sort_unstable();
    distinct_query_terms.dedup();

    distinct_query_terms
        .into_iter()
        .map(|term| {
            let tf = term_frequency(term, doc_tokens) as f64;
            tf * corpus.idf(term)
        })
        .sum()
}

/// `tfidf_score` normalized against the query's own best-possible score
/// (matching itself exactly) so the result lands in roughly `[0, 1]` and is
/// comparable across queries of different lengths — needed to combine it
/// with the other `[0, 1]` sub-scores in the ranking formula (§7). Clamped
/// to `1.0` in case a document repeats a query term more often than the
/// query itself does. `0.0` for a query with no scorable terms (e.g. every
/// token was a stopword or otherwise absent from the corpus) rather than a
/// division by zero.
pub fn normalized_tfidf_score(
    query_tokens: &[String],
    doc_tokens: &[String],
    corpus: &TfIdfCorpus,
) -> f64 {
    let self_score = tfidf_score(query_tokens, query_tokens, corpus);
    if self_score <= 0.0 {
        return 0.0;
    }
    (tfidf_score(query_tokens, doc_tokens, corpus) / self_score).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn a_rare_term_gets_a_higher_idf_than_a_common_one() {
        let corpus = TfIdfCorpus::build(&[
            tokens(&["fire", "extinguisher", "missing"]),
            tokens(&["fire", "hydrant", "leaking"]),
            tokens(&["fire", "alarm", "silent"]),
            tokens(&["scaffold", "toe", "board", "missing"]),
        ]);

        // "fire" appears in 3/4 documents; "scaffold" in 1/4.
        assert!(corpus.idf("scaffold") > corpus.idf("fire"));
    }

    #[test]
    fn a_term_in_every_document_still_has_a_positive_idf() {
        let corpus = TfIdfCorpus::build(&[tokens(&["fire", "safety"]), tokens(&["fire", "drill"])]);
        assert!(corpus.idf("fire") > 0.0);
    }

    #[test]
    fn a_term_absent_from_the_corpus_has_zero_idf() {
        let corpus = TfIdfCorpus::build(&[tokens(&["fire", "safety"])]);
        assert_eq!(corpus.idf("scaffold"), 0.0);
    }

    #[test]
    fn empty_corpus_does_not_panic() {
        let corpus = TfIdfCorpus::build(&[]);
        assert_eq!(corpus.document_count(), 0);
        assert_eq!(corpus.idf("anything"), 0.0);
    }

    #[test]
    fn term_frequency_counts_repeated_occurrences() {
        let doc = tokens(&["fire", "fire", "extinguisher"]);
        assert_eq!(term_frequency("fire", &doc), 2);
        assert_eq!(term_frequency("hydrant", &doc), 0);
    }

    #[test]
    fn tfidf_score_rewards_matching_a_rare_term_over_a_common_one() {
        let corpus = TfIdfCorpus::build(&[
            tokens(&["fire", "extinguisher", "missing"]),
            tokens(&["fire", "hydrant", "leaking"]),
            tokens(&["fire", "alarm", "silent"]),
            tokens(&["scaffold", "toe", "board", "missing"]),
        ]);

        let query = tokens(&["fire"]);
        let matches_common_term = tokens(&["fire", "hydrant", "leaking"]);

        let query_rare = tokens(&["scaffold"]);
        let matches_rare_term = tokens(&["scaffold", "toe", "board", "missing"]);

        let common_score = tfidf_score(&query, &matches_common_term, &corpus);
        let rare_score = tfidf_score(&query_rare, &matches_rare_term, &corpus);

        assert!(rare_score > common_score);
    }

    #[test]
    fn tfidf_score_is_zero_when_nothing_overlaps() {
        let corpus =
            TfIdfCorpus::build(&[tokens(&["fire", "safety"]), tokens(&["scaffold", "board"])]);
        let query = tokens(&["hydrant"]);
        let doc = tokens(&["scaffold", "board"]);
        assert_eq!(tfidf_score(&query, &doc, &corpus), 0.0);
    }

    #[test]
    fn duplicate_query_terms_do_not_double_count() {
        let corpus = TfIdfCorpus::build(&[tokens(&["fire", "fire", "safety"])]);
        let query_once = tokens(&["fire"]);
        let query_twice = tokens(&["fire", "fire"]);
        let doc = tokens(&["fire", "fire", "safety"]);

        assert_eq!(
            tfidf_score(&query_once, &doc, &corpus),
            tfidf_score(&query_twice, &doc, &corpus)
        );
    }

    #[test]
    fn normalized_score_of_an_exact_match_is_one() {
        let corpus = TfIdfCorpus::build(&[
            tokens(&["fire", "extinguisher", "missing"]),
            tokens(&["scaffold", "board"]),
        ]);
        let query = tokens(&["fire", "extinguisher", "missing"]);
        assert_eq!(normalized_tfidf_score(&query, &query, &corpus), 1.0);
    }

    #[test]
    fn normalized_score_of_no_overlap_is_zero() {
        let corpus = TfIdfCorpus::build(&[
            tokens(&["fire", "extinguisher", "missing"]),
            tokens(&["scaffold", "board"]),
        ]);
        let query = tokens(&["fire", "extinguisher", "missing"]);
        let unrelated_doc = tokens(&["scaffold", "board"]);
        assert_eq!(normalized_tfidf_score(&query, &unrelated_doc, &corpus), 0.0);
    }

    #[test]
    fn normalized_score_of_a_partial_match_is_between_zero_and_one() {
        let corpus = TfIdfCorpus::build(&[
            tokens(&["fire", "extinguisher", "missing"]),
            tokens(&["scaffold", "board"]),
        ]);
        let query = tokens(&["fire", "extinguisher", "missing"]);
        let partial_doc = tokens(&["fire", "alarm"]);
        let score = normalized_tfidf_score(&query, &partial_doc, &corpus);
        assert!(score > 0.0 && score < 1.0);
    }

    #[test]
    fn normalized_score_for_a_query_with_no_scorable_terms_is_zero() {
        let corpus = TfIdfCorpus::build(&[tokens(&["fire", "safety"])]);
        assert_eq!(
            normalized_tfidf_score(&[], &tokens(&["fire"]), &corpus),
            0.0
        );
    }
}
