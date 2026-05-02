use serde::Deserialize;
use std::fmt;

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractResult {
    pub content: String,
    pub tier: String,
    pub importance: f64,
}

impl Default for ExtractResult {
    fn default() -> Self {
        ExtractResult {
            content: String::new(),
            tier: "semantic".to_string(),
            importance: 0.5,
        }
    }
}

impl fmt::Display for ExtractResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{} | {:.2}] {}",
            self.tier, self.importance, self.content
        )
    }
}

/// Parse extraction JSON returned by an LLM.
/// Expected format: [{"content": "...", "tier": "semantic|episodic", "importance": 0.9}]
pub fn parse_extraction_json(
    json_str: &str,
) -> Result<Vec<ExtractResult>, Box<dyn std::error::Error>> {
    let raw_results: Vec<serde_json::Value> = serde_json::from_str(json_str)?;
    let mut results = Vec::new();

    for item in raw_results {
        let content = item
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tier = item
            .get("tier")
            .and_then(|v| v.as_str())
            .unwrap_or("semantic")
            .to_string();
        let importance = item
            .get("importance")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        if !content.is_empty() {
            results.push(ExtractResult {
                content,
                tier,
                importance,
            });
        }
    }

    Ok(results)
}

/// Heuristic tier classification.
pub fn classify_tier(content: &str) -> &str {
    let lower = content.to_lowercase();

    if lower.contains("said")
        || lower.contains("today")
        || lower.contains("yesterday")
        || lower.contains("had a")
        || lower.contains("told me")
    {
        return "episodic";
    }

    if lower.contains("prefer")
        || lower.contains("like")
        || lower.contains("hate")
        || lower.contains("use")
        || lower.contains("always")
        || lower.contains("never")
    {
        return "semantic";
    }

    "semantic"
}

/// Build the LLM extraction prompt.
pub fn build_extraction_prompt(text: &str) -> String {
    let prompt_template = r#"You are a memory extraction system. Given a natural language message from a user, extract factual memories and events.

For each memory, return:
- content: a concise, third-person factual statement (e.g., 'User prefers dark mode')
- tier: 'episodic' for transient events or 'semantic' for persistent facts/preferences
- importance: 0.0-1.0 score indicating how important this memory is

Return ONLY a JSON array. No markdown, no extra text.

Example input: "I had a bad day. Dark mode helps."
Example output:
[{"content": "User had a bad day", "tier": "episodic", "importance": 0.6}, {"content": "User prefers dark mode", "tier": "semantic", "importance": 0.9}]

Now extract from this text:
"#;
    format!("{}{}", prompt_template, text)
}

/// OpenAI provider configuration.
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl OpenAiConfig {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("MNEMO_OPENAI_API_KEY").ok()?;
        let model =
            std::env::var("MNEMO_OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        let base_url = std::env::var("MNEMO_OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        Some(OpenAiConfig {
            api_key,
            model,
            base_url,
        })
    }
}

/// Call OpenAI Chat Completions API to extract memories.
pub async fn extract_with_openai(
    config: &OpenAiConfig,
    text: &str,
) -> Result<Vec<ExtractResult>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let prompt = build_extraction_prompt(text);

    let body = serde_json::json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": "You extract factual memories from text."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.3,
        "max_tokens": 500,
    });

    let response = client
        .post(format!("{}/chat/completions", config.base_url))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let err_text = response.text().await?;
        return Err(format!("OpenAI API error {}: {}", status, err_text).into());
    }

    let json: serde_json::Value = response.json().await?;
    let raw_content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Empty response from LLM")?;

    parse_extraction_json(raw_content.trim())
}

/// Extract memories from text using local heuristic as fallback.
pub async fn extract_memories(
    text: &str,
    config: Option<&OpenAiConfig>,
) -> Result<Vec<ExtractResult>, Box<dyn std::error::Error>> {
    if let Some(config) = config {
        extract_with_openai(config, text).await
    } else {
        local_extract(text)
    }
}

/// Simple local heuristic extraction.
fn local_extract(text: &str) -> Result<Vec<ExtractResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();
    let sentences: Vec<&str> = text
        .split(['.', '!', '?'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for sentence in sentences {
        let lower = sentence.to_lowercase();
        if lower.starts_with("i ")
            || lower.starts_with("my ")
            || lower.starts_with("i'm ")
            || lower.starts_with("i am ")
        {
            let tier = classify_tier(sentence);
            let importance = if tier == "semantic" { 0.8 } else { 0.6 };
            results.push(ExtractResult {
                content: rephrase_third_person(sentence),
                tier: tier.to_string(),
                importance,
            });
        }
    }

    Ok(results)
}

fn rephrase_third_person(text: &str) -> String {
    let cleaned = text
        .trim_start_matches("I ")
        .trim_start_matches("I'm ")
        .trim_start_matches("I am ")
        .trim_start_matches("My ");
    format!("User {}", cleaned)
}
