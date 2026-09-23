use std::collections::HashMap;
use std::sync::LazyLock;

/// Fixed, built-in HSE abbreviation expansions — distinct from
/// `synonym_bank` (Module 6's learned/approved pairs, which grow over time
/// via KLE admin approval in Module 9). These are common enough across
/// every deployment that they don't need to be learned or admin-approved;
/// treat this as a small, hand-maintained seed list, not an exhaustive
/// dictionary.
pub static ABBREVIATIONS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    [
        ("ppe", "personal protective equipment"),
        ("loto", "lockout tagout"),
        ("sop", "standard operating procedure"),
        ("ptw", "permit to work"),
        ("msds", "material safety data sheet"),
        ("sds", "safety data sheet"),
        ("ehs", "environment health and safety"),
        ("hse", "health safety and environment"),
        ("ncr", "non conformance report"),
        ("capa", "corrective and preventive action"),
        ("jsa", "job safety analysis"),
        ("ip", "ingress protection"),
        ("rcd", "residual current device"),
        ("elcb", "earth leakage circuit breaker"),
        ("pm", "preventive maintenance"),
    ]
    .into_iter()
    .collect()
});

/// Replaces whole-word abbreviation occurrences in `text` with their
/// expansion (e.g. "ppe" -> "personal protective equipment"). Matching is
/// case-insensitive and word-boundary aware, so it won't touch "ppe" inside
/// a longer token like "steppe". Callers normally run this before
/// tokenizing, on already-lowercased text.
pub fn normalize_abbreviations(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let stripped: String = word
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            match ABBREVIATIONS.get(stripped.as_str()) {
                Some(expansion) => (*expansion).to_string(),
                None => word.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_a_known_abbreviation() {
        assert_eq!(
            normalize_abbreviations("worker missing ppe"),
            "worker missing personal protective equipment"
        );
    }

    #[test]
    fn is_case_insensitive() {
        assert_eq!(
            normalize_abbreviations("PPE not worn"),
            "personal protective equipment not worn"
        );
    }

    #[test]
    fn does_not_touch_words_that_merely_contain_an_abbreviation() {
        assert_eq!(normalize_abbreviations("steppe region"), "steppe region");
    }

    #[test]
    fn leaves_unknown_words_untouched() {
        assert_eq!(
            normalize_abbreviations("scaffold missing toe-board"),
            "scaffold missing toe-board"
        );
    }
}
