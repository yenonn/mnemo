use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn test_recall_finds_memory_with_different_phrasing() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-agent-recall-expansion";

    // Step 1: Store a memory about code coverage
    let mut remember = Command::cargo_bin("mnemo").unwrap();
    remember.env("HOME", dir.path());
    remember.arg("--agent-id").arg(agent_id);
    remember.arg("remember").arg("User prefers code coverage of at least 80%");
    remember.arg("--memory-type").arg("semantic");
    remember.assert().success();

    // Step 2: Recall with different phrasing — "coverage preferences"
    let mut recall = Command::cargo_bin("mnemo").unwrap();
    recall.env("HOME", dir.path());
    recall.arg("--agent-id").arg(agent_id);
    recall.arg("recall").arg("coverage preferences");
    recall.arg("--limit").arg("10");

    recall
        .assert()
        .success()
        .stdout(predicates::str::contains("code coverage"));
}
