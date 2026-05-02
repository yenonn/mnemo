use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("mnemo 0.1.0"));
}

#[test]
fn test_remember_and_recall() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-agent-integration";

    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("remember").arg("User likes blue theme");
    cmd.arg("--memory-type").arg("semantic");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("recall").arg("blue theme");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("User likes blue theme"));
}

#[test]
fn test_init_and_status() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-agent-status";

    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("init");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("status");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Agent:"));
}

#[test]
fn test_repl_mode() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-agent-repl";

    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("--repl");
    cmd.write_stdin("INIT;\nSTATUS;\nquit\n");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("mnemo>"))
        .stdout(predicate::str::contains("<ok>"))
        .stdout(predicate::str::contains("<status>"));
}

#[test]
fn test_consolidate_working_to_episodic() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-agent-consolidate";

    // Remember a working memory
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("remember").arg("First conversation");
    cmd.arg("--memory-type").arg("working");
    cmd.assert().success();

    // Consolidate to episodic
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("consolidate").arg("working").arg("episodic");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Consolidated"));
}

#[test]
fn test_pragma_get_set() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-agent-pragma";

    // Set pragma
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("pragma").arg("test_key").arg("test_value");
    cmd.assert().success();

    // Get pragma
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("pragma").arg("test_key");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test_key = test_value"));
}

#[test]
fn test_bind_retrieve_intent() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-agent-bind-retrieve";

    // Store memory with matching keywords
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("remember")
        .arg("User preferences include dark themes for all applications");
    cmd.arg("--memory-type").arg("semantic");
    cmd.assert().success();

    // Use BIND to retrieve using exact matching terms
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("bind").arg("What are my preferences?");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Found"))
        .stdout(predicates::str::contains("preferences"));
}

#[test]
fn test_bind_store_intent() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-agent-bind-store";

    // Use BIND with clear store signal
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("bind").arg("I prefer using vim for all my coding");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Extracted and stored"))
        .stdout(predicates::str::contains("mem-"));
}
