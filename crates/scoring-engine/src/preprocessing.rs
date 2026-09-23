use crate::abbreviations::normalize_abbreviations as expand_abbreviations;
use crate::stopwords::is_stopword;

/// Toggles for each preprocessing step. All four default to `true`,
/// matching the reference behavior this formalizes; callers that need the
/// raw tokens for some other purpose (e.g. displaying a highlighted
/// original phrase) can selectively disable steps instead of hand-rolling
/// their own tokenizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreprocessConfig {
    pub lowercase: bool,
    pub normalize_abbreviations: bool,
    pub remove_punctuation: bool,
    pub remove_stopwords: bool,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            lowercase: true,
            normalize_abbreviations: true,
            remove_punctuation: true,
            remove_stopwords: true,
        }
    }
}

/// Runs the full pipeline — lowercase, abbreviation expansion, punctuation
/// stripping, tokenization, stopword removal, in that order — and returns
/// the resulting tokens. This is the single place index-time and
/// query-time text both pass through (Module 7 depends on that: the same
/// function must run on both sides or terms silently stop matching).
pub fn preprocess(text: &str, config: &PreprocessConfig) -> Vec<String> {
    let mut working = text.to_string();

    if config.lowercase {
        working = working.to_lowercase();
    }
    if config.normalize_abbreviations {
        working = expand_abbreviations(&working);
    }
    if config.remove_punctuation {
        working = strip_punctuation(&working);
    }

    let tokens: Vec<String> = working
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect();

    if config.remove_stopwords {
        tokens.into_iter().filter(|t| !is_stopword(t)).collect()
    } else {
        tokens
    }
}

/// Convenience wrapper for the common case of every step enabled.
pub fn preprocess_default(text: &str) -> Vec<String> {
    preprocess(text, &PreprocessConfig::default())
}

/// Replaces any character that isn't alphanumeric or whitespace with a
/// space — so e.g. "toe-board" becomes "toe board", two indexable tokens
/// instead of one token joined by a hyphen a query is unlikely to type.
pub fn strip_punctuation(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_pipeline_lowercases_expands_strips_and_removes_stopwords() {
        let tokens = preprocess_default("Worker was NOT wearing PPE near the scaffold.");
        assert_eq!(
            tokens,
            vec![
                "worker",
                "wearing",
                "personal",
                "protective",
                "equipment",
                "scaffold"
            ]
        );
    }

    #[test]
    fn hyphenated_terms_split_into_separate_tokens() {
        let tokens = preprocess_default("Scaffold missing toe-board");
        assert!(tokens.contains(&"toe".to_string()));
        assert!(tokens.contains(&"board".to_string()));
    }

    #[test]
    fn disabling_stopword_removal_keeps_them() {
        let config = PreprocessConfig {
            remove_stopwords: false,
            ..PreprocessConfig::default()
        };
        let tokens = preprocess("the scaffold is missing a board", &config);
        assert!(tokens.contains(&"the".to_string()));
        assert!(tokens.contains(&"is".to_string()));
    }

    #[test]
    fn disabling_lowercase_preserves_case() {
        let config = PreprocessConfig {
            lowercase: false,
            normalize_abbreviations: false,
            ..PreprocessConfig::default()
        };
        let tokens = preprocess("Scaffold Missing Board", &config);
        assert_eq!(tokens, vec!["Scaffold", "Missing", "Board"]);
    }

    #[test]
    fn empty_input_produces_no_tokens() {
        assert!(preprocess_default("").is_empty());
        assert!(preprocess_default("   ").is_empty());
    }

    #[test]
    fn index_time_and_query_time_text_normalize_identically() {
        let indexed = preprocess_default("Fire Extinguisher - Missing Pressure Gauge");
        let queried = preprocess_default("fire extinguisher missing pressure gauge");
        assert_eq!(indexed, queried);
    }
}
