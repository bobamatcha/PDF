//! Standard I/O transport for MCP
//!
//! This module implements the stdio transport for the MCP server,
//! allowing communication via standard input/output streams.
//!
//! IMPORTANT: All logging MUST go to stderr. stdout is reserved for
//! JSON-RPC protocol messages only.

use std::collections::HashMap;
use std::io::{BufRead, Write};

use serde_json::json;

use crate::compiler::errors::ServerError;
use crate::mcp::protocol::*;
use crate::mcp::{prompts, resources, tools, TypstMcpServer};

/// Run the MCP server using stdio transport
pub async fn run_stdio_server(timeout_ms: u64) -> Result<(), ServerError> {
    tracing::info!("Starting stdio transport");

    let server = TypstMcpServer::new().with_timeout(timeout_ms);
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();

    let mut stdin_lock = stdin.lock();
    let mut stdout_lock = stdout.lock();

    loop {
        // Read request
        let request = match read_message(&mut stdin_lock) {
            Ok(Some(req)) => req,
            Ok(None) => {
                tracing::info!("EOF reached, shutting down");
                break;
            }
            Err(e) => {
                tracing::error!("Failed to read message: {}", e);
                continue;
            }
        };

        tracing::debug!("Received request: {:?}", request.method);

        // Handle request
        let response = handle_request(&server, request, timeout_ms).await;

        // Write response
        if let Err(e) = write_message(&mut stdout_lock, &response) {
            tracing::error!("Failed to write response: {}", e);
        }
    }

    Ok(())
}

/// Read a JSON-RPC message from the input stream
fn read_message<R: BufRead>(reader: &mut R) -> Result<Option<JsonRpcRequest>, ServerError> {
    // Read Content-Length header
    let mut header = String::new();
    if reader.read_line(&mut header)? == 0 {
        return Ok(None); // EOF
    }

    let header = header.trim();
    if header.is_empty() {
        return Ok(None);
    }

    let content_length: usize = if header.starts_with("Content-Length:") {
        header
            .strip_prefix("Content-Length:")
            .unwrap()
            .trim()
            .parse()
            .map_err(|_| ServerError::ProtocolError("Invalid Content-Length".to_string()))?
    } else {
        return Err(ServerError::ProtocolError(format!(
            "Expected Content-Length header, got: {}",
            header
        )));
    };

    // Skip empty line after headers
    let mut empty = String::new();
    reader.read_line(&mut empty)?;

    // Read body
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;

    // Parse JSON
    let request: JsonRpcRequest = serde_json::from_slice(&body)?;
    Ok(Some(request))
}

/// Write a JSON-RPC message to the output stream
fn write_message<W: Write>(writer: &mut W, response: &JsonRpcResponse) -> Result<(), ServerError> {
    let body = serde_json::to_string(response)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());

    writer.write_all(header.as_bytes())?;
    writer.write_all(body.as_bytes())?;
    writer.flush()?;

    Ok(())
}

/// Handle a JSON-RPC request
async fn handle_request(
    server: &TypstMcpServer,
    request: JsonRpcRequest,
    timeout_ms: u64,
) -> JsonRpcResponse {
    let id = request.id.clone();

    match request.method.as_str() {
        "initialize" => handle_initialize(id, server),
        "initialized" => JsonRpcResponse::success(id, json!({})),
        "tools/list" => handle_list_tools(id),
        "tools/call" => handle_call_tool(id, request.params, timeout_ms).await,
        "resources/list" => handle_list_resources(id),
        "resources/read" => handle_read_resource(id, request.params),
        "prompts/list" => handle_list_prompts(id),
        "prompts/get" => handle_get_prompt(id, request.params),
        "ping" => JsonRpcResponse::success(id, json!({})),
        _ => JsonRpcResponse::error(id, -32601, format!("Method not found: {}", request.method)),
    }
}

fn handle_initialize(id: Option<serde_json::Value>, server: &TypstMcpServer) -> JsonRpcResponse {
    let result = InitializeResult {
        protocol_version: "2024-11-05".to_string(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability {
                list_changed: Some(false),
            }),
            resources: Some(ResourcesCapability {
                subscribe: Some(false),
                list_changed: Some(false),
            }),
            prompts: Some(PromptsCapability {
                list_changed: Some(false),
            }),
        },
        server_info: ServerInfo {
            name: server.name().to_string(),
            version: server.version().to_string(),
        },
    };

    JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
}

fn handle_list_tools(id: Option<serde_json::Value>) -> JsonRpcResponse {
    let tools = tools::get_tool_definitions();
    JsonRpcResponse::success(id, json!({ "tools": tools }))
}

async fn handle_call_tool(
    id: Option<serde_json::Value>,
    params: serde_json::Value,
    timeout_ms: u64,
) -> JsonRpcResponse {
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match tools::handle_tool_call(name, arguments, timeout_ms).await {
        Ok(content) => JsonRpcResponse::success(
            id,
            json!({
                "content": content,
                "isError": false
            }),
        ),
        Err(e) => JsonRpcResponse::success(
            id,
            json!({
                "content": [{"type": "text", "text": format!("Error: {}", e)}],
                "isError": true
            }),
        ),
    }
}

fn handle_list_resources(id: Option<serde_json::Value>) -> JsonRpcResponse {
    let resources = resources::get_resource_definitions();
    JsonRpcResponse::success(id, json!({ "resources": resources }))
}

fn handle_read_resource(
    id: Option<serde_json::Value>,
    params: serde_json::Value,
) -> JsonRpcResponse {
    let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");

    match resources::read_resource(uri) {
        Ok(content) => JsonRpcResponse::success(id, json!({ "contents": [content] })),
        Err(e) => JsonRpcResponse::error(id, -32602, e.to_string()),
    }
}

fn handle_list_prompts(id: Option<serde_json::Value>) -> JsonRpcResponse {
    let prompts = prompts::get_prompt_definitions();
    JsonRpcResponse::success(id, json!({ "prompts": prompts }))
}

fn handle_get_prompt(id: Option<serde_json::Value>, params: serde_json::Value) -> JsonRpcResponse {
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");

    let arguments: HashMap<String, String> = params
        .get("arguments")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    match prompts::get_prompt(name, arguments) {
        Ok(messages) => JsonRpcResponse::success(id, json!({ "messages": messages })),
        Err(e) => JsonRpcResponse::error(id, -32602, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_message() {
        let input = "Content-Length: 52\r\n\r\n{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":1,\"params\":{}}";
        let mut reader = input.as_bytes();

        let request = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(request.method, "ping");
    }

    #[test]
    fn test_write_message() {
        let response = JsonRpcResponse::success(Some(json!(1)), json!({"result": "ok"}));
        let mut output = Vec::new();

        write_message(&mut output, &response).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.starts_with("Content-Length:"));
        assert!(output_str.contains("result"));
    }
}
