#[derive(Debug, Clone, PartialEq)]
pub struct QueryIntent {
    pub confidence: f64,
    pub query_terms: Vec<String>,
    pub intent_type: IntentType,
    pub time_range: Option<String>,
    pub memory_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IntentType {
    RetrieveAll,     // "what do I have", "show me all"
    RetrieveRecent,  // "yesterday", "last time", "recently"
    RetrieveByTopic, // "my preferences", "about vim"
    RetrieveByDate,  // "on Monday", "May 1st"
    Store,           // "remember that..."
    Unknown,         // no clear intent
}

/// Keywords that trigger automatic memory retrieval
pub const RETRIEVE_KEYWORDS: &[&str] = &[
    "what",
    "tell me",
    "show me",
    "list",
    "do I have",
    "yesterday",
    "last time",
    "recently",
    "before",
    "about",
    "regarding",
    "my",
    "todos",
    "tasks",
    "completion",
    "completed",
    "done",
    "finished",
    "preferences",
    "likes",
    "settings",
    "config",
    "remind me",
    "what was",
    "what did",
    "how did",
];

pub const TIME_PATTERNS: &[&str] = &[
    "yesterday",
    "today",
    "tomorrow",
    "last week",
    "last month",
    "recently",
    "before",
    "earlier",
    "this morning",
    "this afternoon",
    "last night",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

/// Analyze user message and detect retrieval intent
///
/// Returns Some(QueryIntent) if the message appears to be asking
/// about stored memories, None otherwise.
pub fn analyze_intent(text: &str) -> Option<QueryIntent> {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    // Score different intent signals
    let mut retrieve_score: f64 = 0.0;
    let mut query_terms = Vec::new();
    let mut time_range = None;
    let mut topic_keywords = Vec::new();

    // Check for question patterns
    if text.trim_end().ends_with("?") {
        retrieve_score += 0.3;
    }

    // Check retrieve keywords
    for &keyword in RETRIEVE_KEYWORDS {
        if lower.contains(keyword) {
            retrieve_score += 0.2f64;
            // Extract topic keywords around the match
            if let Some(pos) = lower.find(keyword) {
                let start = pos.saturating_sub(20);
                let end = (pos + keyword.len() + 30).min(lower.len());
                let context = &lower[start..end];
                topic_keywords.push(context.to_string());
            }
        }
    }

    // Check time patterns
    for &pattern in TIME_PATTERNS {
        if lower.contains(pattern) {
            retrieve_score += 0.25f64;
            time_range = Some(pattern.to_string());
            query_terms.push(pattern.to_string());
        }
    }

    // Personal pronouns + possession about past
    if (lower.contains("my ") || lower.contains("i "))
        && (lower.contains("have") || lower.contains("did") || lower.contains("was"))
    {
        retrieve_score += 0.3f64;
        query_terms.push("user".to_string());
    }

    // Extract search terms (nouns and topics)
    let stop_words: &[&str] = &[
        "what", "the", "a", "an", "is", "are", "was", "were", "do", "does", "did", "have", "has",
        "had", "be", "been", "about", "for", "with", "from", "to", "of", "in", "on", "my", "mine",
    ];

    for word in &words {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
        if clean.len() > 2 && !stop_words.contains(&clean) {
            query_terms.push(clean.to_string());
        }
    }

    // Determine intent type
    let intent_type = if retrieve_score > 0.8f64 {
        IntentType::RetrieveAll
    } else if time_range.is_some() {
        IntentType::RetrieveRecent
    } else if !topic_keywords.is_empty() || retrieve_score > 0.4f64 {
        IntentType::RetrieveByTopic
    } else {
        IntentType::Unknown
    };

    // Need sufficient confidence to trigger implicit retrieval
    if retrieve_score < 0.3f64 {
        return None;
    }

    Some(QueryIntent {
        confidence: retrieve_score.min(1.0f64),
        query_terms: query_terms.into_iter().take(5).collect(),
        intent_type,
        time_range,
        memory_types: vec!["episodic".to_string(), "semantic".to_string()],
    })
}

/// Build a structured query string from detected intent
pub fn build_query(intent: &QueryIntent) -> String {
    if intent.query_terms.is_empty() {
        return "user completion items".to_string(); // fallback broad query
    }
    intent.query_terms.join(" ")
}

/// Check if a message should trigger extraction (store intent)
pub fn has_store_intent(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("remember")
        || lower.contains("note")
        || lower.starts_with("i prefer")
        || lower.starts_with("i like")
        || lower.starts_with("i use")
        || (lower.starts_with("i ") && !lower.starts_with("i have") && !lower.starts_with("i did"))
}

/// Suggest which tier based on content
pub fn suggest_tier(text: &str) -> String {
    let lower = text.to_lowercase();
    if lower.contains("yesterday")
        || lower.contains("today")
        || lower.contains("earlier")
        || lower.contains("just")
    {
        "episodic".to_string()
    } else {
        "semantic".to_string()
    }
}
