use mnemo::mcp::{McpRequest, handle_request};
use std::env;

fn setup_home() {
    let temp_dir = tempfile::tempdir().unwrap();
    env::set_var("HOME", temp_dir.path());
}

#[test]
fn test_mcp_initialize() {
    let req = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::Number(1.into())),
        method: "initialize".to_string(),
        params: Some(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        })),
    };

    let resp = handle_request(req, "test-agent");
    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.error.is_none());
    assert!(resp.result.is_some());
}

#[test]
fn test_mcp_tools_list() {
    let req = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::Number(1.into())),
        method: "tools/list".to_string(),
        params: None,
    };

    let resp = handle_request(req, "test-agent");
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
    setup_home();

    let req = McpRequest {
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
    };

    let resp = handle_request(req, "mcp-test-agent");
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let content = result.get("content").unwrap().as_array().unwrap();
    assert!(!content.is_empty());
}

#[test]
fn test_mcp_recall_tool() {
    setup_home();

    let remember_req = McpRequest {
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
    };
    let _ = handle_request(remember_req, "mcp-recall-agent");

    let recall_req = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::Number(2.into())),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "recall",
            "arguments": {
                "query": "vim"
            }
        })),
    };

    let resp = handle_request(recall_req, "mcp-recall-agent");
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let content = result.get("content").unwrap().as_array().unwrap();
    assert!(!content.is_empty());
}

#[test]
fn test_mcp_extract_tool() {
    setup_home();

    let req = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::Number(1.into())),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "extract",
            "arguments": {
                "text": "I prefer dark mode and I use vim"
            }
        })),
    };

    let resp = handle_request(req, "mcp-extract-agent");
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let content = result.get("content").unwrap().as_array().unwrap();
    assert!(!content.is_empty());
}

#[test]
fn test_mcp_forget_tool() {
    setup_home();

    let remember_req = McpRequest {
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
    };
    let remember_resp = handle_request(remember_req, "mcp-forget-agent");
    let result = remember_resp.result.unwrap();
    let content_str = result.get("content").unwrap().as_array().unwrap()[0]
        .get("text").unwrap().as_str().unwrap();
    let mem_id = content_str.split_whitespace().last().unwrap();

    let forget_req = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::Number(2.into())),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "forget",
            "arguments": {
                "id": mem_id
            }
        })),
    };

    let resp = handle_request(forget_req, "mcp-forget-agent");
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let text = result.get("content").unwrap().as_array().unwrap()[0]
        .get("text").unwrap().as_str().unwrap();
    assert!(text.contains("Deleted"));
}

#[test]
fn test_mcp_status_tool() {
    setup_home();

    let req = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::Number(1.into())),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "status",
            "arguments": {}
        })),
    };

    let resp = handle_request(req, "mcp-status-agent");
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let text = result.get("content").unwrap().as_array().unwrap()[0]
        .get("text").unwrap().as_str().unwrap();
    assert!(text.contains("Working:"));
    assert!(text.contains("Episodic:"));
    assert!(text.contains("Semantic:"));
}

#[test]
fn test_mcp_unknown_method() {
    let req = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::Number(1.into())),
        method: "unknown/method".to_string(),
        params: None,
    };

    let resp = handle_request(req, "test-agent");
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32601);
}

#[test]
fn test_mcp_unknown_tool() {
    setup_home();

    let req = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::Number(1.into())),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "nonexistent",
            "arguments": {}
        })),
    };

    let resp = handle_request(req, "test-agent");
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32602);
}

#[test]
fn test_mcp_remember_empty_content() {
    setup_home();

    let req = McpRequest {
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
    };

    let resp = handle_request(req, "test-agent");
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("Missing content"));
}

#[test]
fn test_mcp_extract_empty_text() {
    setup_home();

    let req = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::Number(1.into())),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "extract",
            "arguments": {
                "text": ""
            }
        })),
    };

    let resp = handle_request(req, "test-agent");
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("Missing text"));
}
