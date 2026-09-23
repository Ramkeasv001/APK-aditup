use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use aditup_db::repositories::legal_bank::LegalBankRepository;
use aditup_db::repositories::observation_bank::{ObservationBankEntry, ObservationBankRepository};
use aditup_db::repositories::recommendation_bank::RecommendationBankRepository;
use aditup_db::repositories::synonym_bank::SynonymBankRepository;
use aditup_db::Database;
use aditup_scoring_engine::{
    confidence_band, confidence_percent, fuzzy_similarity, keyword_overlap_score,
    normalized_tfidf_score, preprocess_default, structured_match_score, synonym_contribution_score,
    weighted_score, CandidateScores, InvertedIndex, RankingWeights, StructuredTags, SynonymLookup,
    TfIdfCorpus,
};

use crate::error::ProviderError;
use crate::provider::RecommendationProvider;
use crate::types::{ObservationQuery, RankedCandidate, RankedLegalClause, RecommendationText};

/// Mode 1's implementation of `RecommendationProvider` — deterministic
/// scoring over the local encrypted banks (Modules 4-7), no network, no
/// LLM. See `provider::RecommendationProvider` for the seam Modes 2/3
/// implement later against the same interface.
pub struct RuleBasedProvider<'a> {
    db: &'a Database,
    weights: RankingWeights,
}

impl<'a> RuleBasedProvider<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self {
            db,
            weights: RankingWeights::default(),
        }
    }

    /// For Module 14 (Settings) or Module 9 (KLE feedback) to supply
    /// weights tuned away from the §7 defaults.
    pub fn with_weights(db: &'a Database, weights: RankingWeights) -> Self {
        Self { db, weights }
    }

    /// Legal clause suggestions for `text` — a lighter-weight sibling to
    /// `suggest`: `legal_bank` has no synonym or structured-tag data to draw
    /// on, so this blends only keyword overlap, TF-IDF, and fuzzy phrase
    /// similarity. Optionally scoped to one `standard` (e.g. "IS 2190:2024").
    pub fn suggest_legal_clauses(
        &self,
        text: &str,
        standard: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RankedLegalClause>, ProviderError> {
        let repo = LegalBankRepository::new(self.db);
        let rows = match standard {
            Some(s) => repo.list_by_standard(s)?,
            None => repo.list_all()?,
        };
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let query_tokens = preprocess_default(text);
        let doc_tokens: HashMap<String, Vec<String>> = rows
            .iter()
            .map(|r| (r.id.clone(), preprocess_default(&r.text)))
            .collect();
        let corpus = TfIdfCorpus::build(&doc_tokens.values().cloned().collect::<Vec<_>>());

        let mut scored: Vec<RankedLegalClause> = rows
            .iter()
            .map(|row| {
                let tokens = &doc_tokens[&row.id];
                let overlap = keyword_overlap_score(&query_tokens, tokens);
                let tfidf = normalized_tfidf_score(&query_tokens, tokens, &corpus);
                let fuzzy = fuzzy_similarity(&query_tokens, tokens);
                let combined = overlap * 0.5 + tfidf * 0.3 + fuzzy * 0.2;
                RankedLegalClause {
                    legal_bank_id: row.id.clone(),
                    standard: row.standard.clone(),
                    clause: row.clause.clone(),
                    text: row.text.clone(),
                    confidence_percent: confidence_percent(combined),
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.confidence_percent
                .partial_cmp(&a.confidence_percent)
                .unwrap_or(Ordering::Equal)
        });
        scored.truncate(limit);
        Ok(scored)
    }
}

impl<'a> RecommendationProvider for RuleBasedProvider<'a> {
    fn suggest(
        &self,
        query: &ObservationQuery,
        limit: usize,
    ) -> Result<Vec<RankedCandidate>, ProviderError> {
        let obs_repo = ObservationBankRepository::new(self.db);

        let bank_keys: Vec<String> = if query.bank_keys.is_empty() {
            obs_repo.list_bank_keys()?
        } else {
            query.bank_keys.clone()
        };

        let mut rows: Vec<ObservationBankEntry> = Vec::new();
        for bank_key in &bank_keys {
            rows.extend(obs_repo.list_for_bank(bank_key)?);
        }
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let query_tokens = preprocess_default(&query.text);

        let doc_tokens: HashMap<String, Vec<String>> = rows
            .iter()
            .map(|r| (r.id.clone(), preprocess_default(&r.text)))
            .collect();

        let index_docs: Vec<(String, Vec<String>)> = doc_tokens
            .iter()
            .map(|(id, toks)| (id.clone(), toks.clone()))
            .collect();
        let index = InvertedIndex::build(&index_docs);
        let corpus = TfIdfCorpus::build(&doc_tokens.values().cloned().collect::<Vec<_>>());

        let synonym_pairs: Vec<(String, String)> = SynonymBankRepository::new(self.db)
            .list_all()?
            .into_iter()
            .map(|s| (s.term_a, s.term_b))
            .collect();
        let synonym_lookup = SynonymLookup::build(&synonym_pairs);

        // Keyword search narrows to documents sharing a term first (the fast
        // common case). If nothing shares even one term — e.g. every word in
        // the observation is misspelled relative to the bank — fall back to
        // scoring every row in the scoped bank(s) so fuzzy matching still
        // gets a chance, at the cost of a full scan only on that rarer path.
        let hits = index.candidates(&query_tokens);
        let candidate_ids: HashSet<String> = if hits.is_empty() {
            rows.iter().map(|r| r.id.clone()).collect()
        } else {
            hits
        };

        let rec_repo = RecommendationBankRepository::new(self.db);

        let mut ranked: Vec<RankedCandidate> = Vec::new();
        for row in rows.iter().filter(|r| candidate_ids.contains(&r.id)) {
            let tokens = &doc_tokens[&row.id];

            // observation_bank has no department/equipment columns in the
            // current schema — only `topic` has a natural correspondence to
            // a Stage 2 `category` tag. If the query asserts department or
            // equipment, those dimensions honestly score as non-matches
            // rather than being silently ignored or fabricated.
            let candidate_tags = StructuredTags {
                category: Some(row.topic.clone()),
                department: None,
                equipment: None,
            };

            let scores = CandidateScores {
                keyword_overlap: keyword_overlap_score(&query_tokens, tokens),
                tfidf: normalized_tfidf_score(&query_tokens, tokens, &corpus),
                fuzzy_similarity: fuzzy_similarity(&query_tokens, tokens),
                synonym_contribution: synonym_contribution_score(
                    &query_tokens,
                    tokens,
                    &synonym_lookup,
                ),
                structured_match: structured_match_score(&query.tags, &candidate_tags),
            };

            let percent = confidence_percent(weighted_score(&scores, &self.weights));

            let recommendations = rec_repo
                .list_for_observation(&row.id)?
                .into_iter()
                .map(|r| RecommendationText {
                    id: r.id,
                    text: r.text,
                })
                .collect();

            ranked.push(RankedCandidate {
                observation_bank_id: row.id.clone(),
                bank_key: row.bank_key.clone(),
                topic: row.topic.clone(),
                label: row.label.clone(),
                text: row.text.clone(),
                recommendations,
                confidence_percent: percent,
                confidence_band: confidence_band(percent),
                scores,
            });
        }

        ranked.sort_by(|a, b| {
            b.confidence_percent
                .partial_cmp(&a.confidence_percent)
                .unwrap_or(Ordering::Equal)
        });
        ranked.truncate(limit);
        Ok(ranked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:"), "test-passphrase").unwrap()
    }

    fn seed_observation(
        db: &Database,
        bank_key: &str,
        topic: &str,
        label: &str,
        text: &str,
    ) -> ObservationBankEntry {
        ObservationBankRepository::new(db)
            .create(bank_key, topic, label, text)
            .unwrap()
    }

    #[test]
    fn suggest_ranks_the_best_textual_match_first() {
        let db = test_db();
        seed_observation(
            &db,
            "fire_extinguisher",
            "Fire Protection",
            "Missing pressure gauge",
            "Fire extinguisher pressure gauge is missing or unreadable.",
        );
        seed_observation(
            &db,
            "fire_extinguisher",
            "Fire Protection",
            "Blocked access",
            "Access to the fire extinguisher is obstructed by stored material.",
        );

        let provider = RuleBasedProvider::new(&db);
        let query = ObservationQuery {
            text: "Fire extinguisher pressure gauge missing".to_string(),
            ..Default::default()
        };

        let results = provider.suggest(&query, 5).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].label, "Missing pressure gauge");
        assert!(
            results[0].confidence_percent
                > results.get(1).map(|r| r.confidence_percent).unwrap_or(0.0)
        );
    }

    #[test]
    fn suggest_attaches_linked_recommendations() {
        let db = test_db();
        let obs = seed_observation(
            &db,
            "fire_extinguisher",
            "Fire Protection",
            "Missing pressure gauge",
            "Fire extinguisher pressure gauge is missing or unreadable.",
        );
        RecommendationBankRepository::new(&db)
            .create(
                Some(&obs.id),
                "Fire Protection",
                "Replace the extinguisher immediately.",
            )
            .unwrap();

        let provider = RuleBasedProvider::new(&db);
        let query = ObservationQuery {
            text: "pressure gauge missing".to_string(),
            ..Default::default()
        };

        let results = provider.suggest(&query, 5).unwrap();
        assert_eq!(results[0].recommendations.len(), 1);
        assert_eq!(
            results[0].recommendations[0].text,
            "Replace the extinguisher immediately."
        );
    }

    #[test]
    fn suggest_respects_bank_key_scoping() {
        let db = test_db();
        seed_observation(
            &db,
            "fire_extinguisher",
            "Topic",
            "Extinguisher finding",
            "extinguisher missing seal",
        );
        seed_observation(
            &db,
            "sprinkler",
            "Topic",
            "Sprinkler finding",
            "sprinkler head corroded",
        );

        let provider = RuleBasedProvider::new(&db);
        let query = ObservationQuery {
            text: "missing seal".to_string(),
            bank_keys: vec!["sprinkler".to_string()],
            ..Default::default()
        };

        let results = provider.suggest(&query, 5).unwrap();
        assert!(results.iter().all(|r| r.bank_key == "sprinkler"));
    }

    #[test]
    fn suggest_falls_back_to_fuzzy_when_no_keyword_overlap() {
        let db = test_db();
        seed_observation(
            &db,
            "fire_extinguisher",
            "Topic",
            "Missing pressure gauge",
            "extinguisher pressure gauge",
        );

        let provider = RuleBasedProvider::new(&db);
        // Deliberately misspelled so no token exact-matches the indexed doc.
        let query = ObservationQuery {
            text: "extinguisherr presure gaug".to_string(),
            ..Default::default()
        };

        let results = provider.suggest(&query, 5).unwrap();
        // The inverted index alone would find nothing; the full-scan
        // fallback should still surface the one row in the bank via fuzzy
        // similarity, with a nonzero (if modest) score.
        assert_eq!(results.len(), 1);
        assert!(results[0].confidence_percent > 0.0);
    }

    #[test]
    fn suggest_rewards_a_matching_category_tag() {
        let db = test_db();
        seed_observation(
            &db,
            "bank_a",
            "Fire Protection",
            "A",
            "extinguisher missing",
        );
        seed_observation(&db, "bank_a", "Electrical", "B", "extinguisher missing");

        let provider = RuleBasedProvider::new(&db);
        let query = ObservationQuery {
            text: "extinguisher missing".to_string(),
            tags: StructuredTags {
                category: Some("Fire Protection".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let results = provider.suggest(&query, 5).unwrap();
        assert_eq!(results[0].topic, "Fire Protection");
    }

    #[test]
    fn suggest_on_an_empty_bank_returns_an_empty_list_not_an_error() {
        let db = test_db();
        let provider = RuleBasedProvider::new(&db);
        let query = ObservationQuery {
            text: "anything at all".to_string(),
            ..Default::default()
        };
        assert!(provider.suggest(&query, 5).unwrap().is_empty());
    }

    #[test]
    fn suggest_truncates_to_the_requested_limit() {
        let db = test_db();
        for i in 0..5 {
            seed_observation(
                &db,
                "bank_a",
                "Topic",
                &format!("Label {i}"),
                "fire extinguisher missing",
            );
        }

        let provider = RuleBasedProvider::new(&db);
        let query = ObservationQuery {
            text: "fire extinguisher missing".to_string(),
            ..Default::default()
        };

        let results = provider.suggest(&query, 2).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn suggest_legal_clauses_ranks_by_relevance_and_respects_standard_scope() {
        let db = test_db();
        let legal = LegalBankRepository::new(&db);
        legal
            .create(
                "IS 2190:2024",
                "Clause 6.2",
                "Fire extinguishers shall be inspected monthly for pressure loss.",
            )
            .unwrap();
        legal
            .create(
                "IS 2190:2024",
                "Clause 4.1",
                "Fire extinguishers shall be mounted at accessible locations.",
            )
            .unwrap();
        legal
            .create(
                "NBC Part F",
                "Clause 1",
                "Unrelated building code clause about staircases.",
            )
            .unwrap();

        let provider = RuleBasedProvider::new(&db);
        let results = provider
            .suggest_legal_clauses("pressure loss inspection", Some("IS 2190:2024"), 5)
            .unwrap();

        assert!(results.iter().all(|r| r.standard == "IS 2190:2024"));
        assert_eq!(results[0].clause, "Clause 6.2");
    }
}
