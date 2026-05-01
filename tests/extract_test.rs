use mnemo::extract::{ExtractResult, classify_tier, parse_extraction_json};

#[test]
fn test_classify_tier_episodic_event() {
    assert_eq!(
        classify_tier("User had a bad day at work"),
        "episodic"
    );
}

#[test]
fn test_classify_tier_semantic_fact() {
    assert_eq!(
        classify_tier("User prefers dark mode"),
        "semantic"
    );
}

#[test]
fn test_classify_tier_preference() {
    assert_eq!(
        classify_tier("I love using vim"),
        "semantic"
    );
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
