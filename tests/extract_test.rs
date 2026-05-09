use mnemo::extract::{classify_tier, parse_extraction_json, ExtractResult};

#[test]
fn test_classify_tier_episodic_event() {
    assert_eq!(classify_tier("User had a bad day at work"), "episodic");
}

#[test]
fn test_classify_tier_semantic_fact() {
    assert_eq!(classify_tier("User prefers dark mode"), "semantic");
}

#[test]
fn test_classify_tier_preference() {
    assert_eq!(classify_tier("I love using vim"), "semantic");
}

#[test]
fn test_parse_extraction_json_valid() {
    let json = r#"[
        {"content": "User prefers dark mode", "tier": "semantic", "importance": 0.9},
        {"content": "User had a bad day", "tier": "episodic", "importance": 0.5}
    ]"#;

    let results = parse_extraction_json(json).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].content, "User prefers dark mode");
    assert_eq!(results[0].tier, "semantic");
    assert_eq!(results[0].importance, 0.9);
    assert_eq!(results[1].tier, "episodic");
}

#[test]
fn test_parse_extraction_json_invalid() {
    let json = "not valid json";
    assert!(parse_extraction_json(json).is_err());
}

#[test]
fn test_parse_extraction_json_empty_array() {
    let json = "[]";
    let results = parse_extraction_json(json).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_parse_extraction_json_missing_fields() {
    let json = r#"[{"content": "test"}]"#;
    let results = parse_extraction_json(json).unwrap();
    assert_eq!(results[0].tier, "semantic"); // default
    assert_eq!(results[0].importance, 0.5); // default
}

#[test]
fn test_extract_result_display() {
    let result = ExtractResult {
        content: "User prefers dark mode".to_string(),
        tier: "semantic".to_string(),
        importance: 0.9,
    };
    assert_eq!(
        format!("{}", result),
        "[semantic | 0.90] User prefers dark mode"
    );
}

#[test]
fn test_build_extraction_prompt_contains_text() {
    let prompt = mnemo::extract::build_extraction_prompt("hello world");
    assert!(prompt.contains("hello world"));
    assert!(prompt.contains("memory extraction"));
}

#[test]
fn test_local_extract_personal_statements() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let results = rt
        .block_on(mnemo::extract::extract_memories(
            "I prefer dark mode. I had a bad day. Hello world.",
            None,
        ))
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].content, "User prefer dark mode");
    assert_eq!(results[0].tier, "semantic");
    assert_eq!(results[1].content, "User had a bad day");
    assert_eq!(results[1].tier, "episodic");
}

#[test]
fn test_local_extract_no_personal() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let results = rt
        .block_on(mnemo::extract::extract_memories(
            "The weather is nice today.",
            None,
        ))
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_extract_memories_empty() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let results = rt
        .block_on(mnemo::extract::extract_memories("", None))
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_openai_config_from_env_none() {
    let _ = std::env::remove_var("MNEMO_OPENAI_API_KEY");
    let _ = std::env::remove_var("MNEMO_OLLAMA_ENDPOINT");
    assert!(mnemo::extract::OpenAiConfig::from_env().is_none());
}
