use mnemo::mcp::{handle_request, McpRequest};
use std::env;
use std::sync::Mutex;

// Global lock to prevent parallel HOME env mutation causing flakiness
static HOME_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_mcp_initialize() {
    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            })),
        },
        "test-agent",
    );
    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.error.is_none());
    assert!(resp.result.is_some());
}

#[test]
fn test_mcp_tools_list() {
    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/list".to_string(),
            params: None,
        },
        "test-agent",
    );
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let tools = result.get("tools").unwrap().as_array().unwrap();
    assert!(!tools.is_empty());

    let tool_names: Vec<String> = tools
        .iter()
        .map(|t| t.get("name").unwrap().as_str().unwrap().to_string())
        .collect();
    assert!(tool_names.contains(&"remember".to_string()));
    assert!(tool_names.contains(&"recall".to_string()));
    assert!(tool_names.contains(&"extract".to_string()));
    assert!(tool_names.contains(&"status".to_string()));
    assert!(tool_names.contains(&"forget".to_string()));
}

#[test]
fn test_mcp_remember_tool() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _tmp = tempfile::tempdir().unwrap();
    env::set_var("HOME", _tmp.path());

    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "remember",
                "arguments": {
                    "content": "User prefers dark mode",
                    "memory_type": "semantic"
                }
            })),
        },
        "mcp-test-agent",
    );
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let content = result.get("content").unwrap().as_array().unwrap();
    assert!(!content.is_empty());
}

#[test]
fn test_mcp_recall_tool() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _tmp = tempfile::tempdir().unwrap();
    env::set_var("HOME", _tmp.path());

    let _ = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "remember",
                "arguments": {
                    "content": "User likes vim",
                    "memory_type": "semantic"
                }
            })),
        },
        "mcp-recall-agent",
    );

    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(2.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "recall",
                "arguments": {
                    "query": "vim"
                }
            })),
        },
        "mcp-recall-agent",
    );
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let content = result.get("content").unwrap().as_array().unwrap();
    assert!(!content.is_empty());
}

#[test]
fn test_mcp_extract_tool() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _tmp = tempfile::tempdir().unwrap();
    env::set_var("HOME", _tmp.path());
    env::set_var("MNEMO_OPENAI_API_KEY", "");

    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "extract",
                "arguments": {
                    "text": "I prefer dark mode and I use vim"
                }
            })),
        },
        "mcp-extract-agent",
    );
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let content = result.get("content").unwrap().as_array().unwrap();
    assert!(!content.is_empty());
}

#[test]
fn test_mcp_forget_tool() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _tmp = tempfile::tempdir().unwrap();
    env::set_var("HOME", _tmp.path());

    let remember_resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "remember",
                "arguments": {
                    "content": "To be deleted",
                    "memory_type": "semantic"
                }
            })),
        },
        "mcp-forget-agent",
    );
    let result = remember_resp.result.unwrap();
    let content_str = result.get("content").unwrap().as_array().unwrap()[0]
        .get("text")
        .unwrap()
        .as_str()
        .unwrap();
    let mem_id = content_str.split_whitespace().last().unwrap();

    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(2.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "forget",
                "arguments": {
                    "id": mem_id
                }
            })),
        },
        "mcp-forget-agent",
    );
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let text = result.get("content").unwrap().as_array().unwrap()[0]
        .get("text")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(text.contains("Deleted"));
}

#[test]
fn test_mcp_status_tool() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _tmp = tempfile::tempdir().unwrap();
    env::set_var("HOME", _tmp.path());

    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "status",
                "arguments": {}
            })),
        },
        "mcp-status-agent",
    );
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let text = result.get("content").unwrap().as_array().unwrap()[0]
        .get("text")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(text.contains("Working:"));
    assert!(text.contains("Episodic:"));
    assert!(text.contains("Semantic:"));
}

#[test]
fn test_mcp_unknown_method() {
    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "unknown/method".to_string(),
            params: None,
        },
        "test-agent",
    );
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32601);
}

#[test]
fn test_mcp_unknown_tool() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _tmp = tempfile::tempdir().unwrap();
    env::set_var("HOME", _tmp.path());

    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "nonexistent",
                "arguments": {}
            })),
        },
        "test-agent",
    );
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32602);
}

#[test]
fn test_mcp_remember_empty_content() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _tmp = tempfile::tempdir().unwrap();
    env::set_var("HOME", _tmp.path());

    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "remember",
                "arguments": {
                    "content": "",
                    "memory_type": "semantic"
                }
            })),
        },
        "test-agent",
    );
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("Missing content"));
}

#[test]
fn test_mcp_extract_empty_text() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _tmp = tempfile::tempdir().unwrap();
    env::set_var("HOME", _tmp.path());
    env::set_var("MNEMO_OPENAI_API_KEY", "");

    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "extract",
                "arguments": {
                    "text": ""
                }
            })),
        },
        "test-agent",
    );
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("Missing text"));
}

// Test for Bug 2 — MCP tool calls should accept per-request agent_id override
#[test]
fn test_mcp_recall_with_per_request_agent_id() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _tmp = tempfile::tempdir().unwrap();
    env::set_var("HOME", _tmp.path());

    // Store a memory for agent "agent-a"
    let _ = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "remember",
                "arguments": {
                    "content": "User prefers dark mode",
                    "memory_type": "semantic"
                }
            })),
        },
        "agent-a",
    );

    // Now recall using startup agent_id = "default", but with per-request
    // arguments.agent_id = "agent-a". Without the fix this reads from the empty
    // default DB and returns nothing.
    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(2.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "recall",
                "arguments": {
                    "query": "dark mode",
                    "agent_id": "agent-a"
                }
            })),
        },
        "default",
    );
    assert!(resp.error.is_none(), "Should not error: {:?}", resp.error);
    let result = resp.result.unwrap();
    let content = result.get("content").unwrap().as_array().unwrap();
    let text = content[0].get("text").unwrap().as_str().unwrap();
    assert!(
        text.contains("dark mode"),
        "Expected to recall 'dark mode' from agent-a via per-request agent_id, but got: {}",
        text
    );
}
