use std::collections::{HashMap, HashSet};

/// Bidirectional lookup built from approved `synonym_bank` pairs (Module 6
/// owns the storage; this crate never talks to the database directly — the
/// caller loads the pairs and hands them here). Lets the ranking formula
/// credit a match achieved only through a known synonym relation, distinct
/// from an exact token match.
pub struct SynonymLookup {
    map: HashMap<String, HashSet<String>>,
}

impl SynonymLookup {
    pub fn build(pairs: &[(String, String)]) -> Self {
        let mut map: HashMap<String, HashSet<String>> = HashMap::new();
        for (a, b) in pairs {
            map.entry(a.clone()).or_default().insert(b.clone());
            map.entry(b.clone()).or_default().insert(a.clone());
        }
        Self { map }
    }

    pub fn synonyms_of(&self, term: &str) -> Vec<&str> {
        self.map
            .get(term)
            .map(|set| set.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    pub fn are_synonyms(&self, a: &str, b: &str) -> bool {
        self.map.get(a).is_some_and(|set| set.contains(b))
    }
}

/// Fraction of distinct query tokens that matched a document token *only*
/// via a synonym relation, not a direct/exact match — `0.0` if every query
/// token either matched directly or didn't match at all, up to `1.0` if
/// every query token matched exclusively through a synonym.
pub fn synonym_contribution_score(
    query_tokens: &[String],
    doc_tokens: &[String],
    synonyms: &SynonymLookup,
) -> f64 {
    let mut distinct_query: Vec<&str> = query_tokens.iter().map(String::as_str).collect();
    distinct_query.sort_unstable();
    distinct_query.dedup();
    if distinct_query.is_empty() {
        return 0.0;
    }

    let doc_set: HashSet<&str> = doc_tokens.iter().map(String::as_str).collect();

    let synonym_matches = distinct_query
        .iter()
        .filter(|q| !doc_set.contains(*q) && doc_set.iter().any(|d| synonyms.are_synonyms(q, d)))
        .count();

    synonym_matches as f64 / distinct_query.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    fn pair(a: &str, b: &str) -> (String, String) {
        (a.to_string(), b.to_string())
    }

    #[test]
    fn lookup_is_bidirectional() {
        let lookup = SynonymLookup::build(&[pair("extinguisher", "fire extinguisher")]);
        assert!(lookup.are_synonyms("extinguisher", "fire extinguisher"));
        assert!(lookup.are_synonyms("fire extinguisher", "extinguisher"));
        assert!(!lookup.are_synonyms("extinguisher", "hydrant"));
    }

    #[test]
    fn synonyms_of_returns_every_paired_term() {
        let lookup = SynonymLookup::build(&[
            pair("hydrant", "extinguisher"),
            pair("hydrant", "sprinkler"),
        ]);
        let mut result = lookup.synonyms_of("hydrant");
        result.sort_unstable();
        assert_eq!(result, vec!["extinguisher", "sprinkler"]);
    }

    #[test]
    fn direct_matches_do_not_count_as_synonym_contribution() {
        let lookup = SynonymLookup::build(&[pair("extinguisher", "hydrant")]);
        let query = tokens(&["extinguisher"]);
        let doc = tokens(&["extinguisher", "missing"]); // direct match, no synonym needed
        assert_eq!(synonym_contribution_score(&query, &doc, &lookup), 0.0);
    }

    #[test]
    fn a_synonym_only_match_counts_fully() {
        let lookup = SynonymLookup::build(&[pair("extinguisher", "hydrant")]);
        let query = tokens(&["extinguisher"]);
        let doc = tokens(&["hydrant", "leaking"]); // matched only via the synonym
        assert_eq!(synonym_contribution_score(&query, &doc, &lookup), 1.0);
    }

    #[test]
    fn partial_synonym_contribution_across_multiple_query_terms() {
        let lookup = SynonymLookup::build(&[pair("extinguisher", "hydrant")]);
        let query = tokens(&["extinguisher", "scaffold"]);
        let doc = tokens(&["hydrant", "scaffold"]); // one direct, one synonym-only
        assert_eq!(synonym_contribution_score(&query, &doc, &lookup), 0.5);
    }

    #[test]
    fn no_match_at_all_scores_zero() {
        let lookup = SynonymLookup::build(&[pair("extinguisher", "hydrant")]);
        let query = tokens(&["extinguisher"]);
        let doc = tokens(&["scaffold", "board"]);
        assert_eq!(synonym_contribution_score(&query, &doc, &lookup), 0.0);
    }

    #[test]
    fn empty_query_scores_zero() {
        let lookup = SynonymLookup::build(&[]);
        assert_eq!(
            synonym_contribution_score(&[], &tokens(&["anything"]), &lookup),
            0.0
        );
    }
}
