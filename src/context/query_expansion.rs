//! Query term expansion for BIND retrieval.
//!
//! When a user's query has no lexical overlap with stored memories,
//! expand query terms with synonyms and morphology variants to cast
//! a wider net over the FTS5 index.

/// Static synonym map: each cluster is a set of related English words
/// organized loosely by domain.
static SYNONYM_MAP: &[(&str, &[&str])] = &[
    // Communication / scheduling
    ("todos",       &["tasks", "task", "todo", "to-do", "checklist"]),
    ("meetings",    &["meeting", "calls", "sync", "standup", "review"]),
    ("yesterday",   &["yesterday", "last night", "previous day"]),

    // Work context
    ("work",        &["job", "project", "assignment", "task", "duty"]),
    ("deadline",    &["due", "due date", "milestone", "timeline", "schedule"]),
    ("email",       &["mail", "message", "inbox", "correspondence"]),

    // Preferences
    ("theme",       &["theme", "mode", "style", "appearance", "ui", "skin"]),
    ("dark mode",   &["dark", "night mode", "dark theme", "night theme"]),
    ("preference",  &["preference", "like", "dislike", "hate", "love", "want"]),

    // Tools
    ("editor",      &["editor", "ide", "vim", "emacs", "vscode", "code"]),
    ("terminal",    &["terminal", "shell", "command line", "cli", "bash", "zsh"]),
];

fn is_stop_word(term: &str) -> bool {
    matches!(
        term,
        "the"
            | "a"
            | "an"
            | "is"
            | "are"
            | "was"
            | "were"
            | "be"
            | "been"
            | "have"
            | "has"
            | "had"
            | "do"
            | "does"
            | "did"
            | "will"
            | "would"
            | "should"
            | "could"
            | "may"
            | "might"
            | "can"
            | "shall"
    )
}

fn simple_stem(word: &str) -> Option<String> {
    let lower = word.to_lowercase();
    if let Some(stemmed) = lower
        .strip_suffix("ies")
        .and_then(|s| if s.len() > 1 { Some(format!("{}y", s)) } else { None })
    {
        return Some(stemmed);
    }
    if let Some(stemmed) = lower.strip_suffix("ied").and_then(|s| {
        if s.len() > 1 {
            Some(format!("{}y", s))
        } else {
            None
        }
    }) {
        return Some(stemmed);
    }
    for suffix in ["ing", "ings", "ed", "er", "est", "ly", "s"] {
        if let Some(stemmed) = lower.strip_suffix(suffix) {
            if stemmed.len() > 2 {
                return Some(stemmed.to_string());
            }
        }
    }
    None
}

/// Expand a list of query terms with synonyms and morphology variants.
///
/// Each input term is preserved. If it matches a synonym map key
/// (or appears in a synonym cluster), all cluster members are added.
/// A simple stemmer removes trailing suffixes such as `s`, `ed`, `ing`.
/// Stop-words are dropped.
pub fn expand_query(terms: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();

    for term in terms {
        let lower = term.to_lowercase();
        if is_stop_word(&lower) || lower.len() <= 2 {
            continue;
        }
        result.push(lower.clone());

        // 1. Synonym map: bidirectional match (term is key or is in synonyms)
        for (key, synonyms) in SYNONYM_MAP.iter() {
            let mut matched = false;
            if *key == lower {
                matched = true;
            } else if synonyms.contains(&lower.as_str()) {
                matched = true;
            }

            if matched {
                result.push(key.to_string());
                for &synonym in *synonyms {
                    result.push(synonym.to_string());
                }
            }
        }

        // 2. Morphology: add stemmed variant
        if let Some(stemmed) = simple_stem(&lower) {
            if stemmed != lower {
                result.push(stemmed);
            }
        }
    }

    result.sort();
    result.dedup();
    result
}

/// Build an FTS5 `OR`-joined query from expanded terms.
pub fn build_expanded_fts_query(terms: &[String]) -> String {
    terms.join(" OR ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_query_basic() {
        let expanded = expand_query(&vec!["todos".to_string()]);
        assert!(expanded.contains(&"todos".to_string()));
        assert!(expanded.contains(&"tasks".to_string()));
        assert!(expanded.contains(&"task".to_string()));
    }

    #[test]
    fn test_expand_query_dedup() {
        let expanded = expand_query(&vec!["todos".to_string(), "tasks".to_string()]);
        let unique_count = expanded.len();
        let set_count: usize = expanded.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(
            unique_count, set_count,
            "expand_query must not produce duplicates"
        );
    }

    #[test]
    fn test_expand_query_morphology() {
        let expanded = expand_query(&vec!["meetings".to_string()]);
        assert!(expanded.contains(&"meeting".to_string()));
        assert!(expanded.contains(&"meetings".to_string()));
    }

    #[test]
    fn test_stop_words_dropped() {
        let expanded = expand_query(&vec!["the".to_string(), "a".to_string()]);
        assert!(
            expanded.is_empty(),
            "Stop-words should be dropped from expansion"
        );
    }

    #[test]
    fn test_build_fts_query() {
        let q = build_expanded_fts_query(&vec![
            "todos".to_string(),
            "tasks".to_string(),
            "task".to_string(),
        ]);
        assert_eq!(q, "todos OR tasks OR task");
    }
}
