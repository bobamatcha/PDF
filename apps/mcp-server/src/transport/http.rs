//! HTTP/SSE transport for MCP
//!
//! This module implements the HTTP transport for the MCP server,
//! allowing communication via HTTP POST requests and Server-Sent Events.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::State,
    response::{sse::Event, IntoResponse, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::stream::{self, Stream};
use serde_json::json;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};

use crate::compiler::errors::ServerError;
use crate::compiler::{compile_document, RenderRequest};
use crate::mcp::protocol::*;
use crate::mcp::{prompts, resources, tools, TypstMcpServer};
use crate::templates;

/// REST API request for template rendering
#[derive(Debug, serde::Deserialize)]
pub struct RenderApiRequest {
    /// Template name (e.g., "florida_lease") or raw Typst source
    pub template: String,
    /// Whether `template` is a template name (true) or raw source (false)
    #[serde(default)]
    pub is_template: bool,
    /// Variables for the template
    #[serde(default)]
    pub inputs: std::collections::HashMap<String, serde_json::Value>,
    /// Output format: pdf, svg, png
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "pdf".to_string()
}

/// REST API response for template rendering
#[derive(Debug, serde::Serialize)]
pub struct RenderApiResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

/// Shared state for the HTTP server
#[derive(Clone)]
pub struct HttpServerState {
    /// The MCP server instance
    server: Arc<TypstMcpServer>,
    /// Compilation timeout in milliseconds
    timeout_ms: u64,
    /// Broadcast channel for SSE notifications
    sse_tx: broadcast::Sender<String>,
}

/// Run the MCP server using HTTP transport
pub async fn run_http_server(addr: &str, timeout_ms: u64) -> Result<(), ServerError> {
    tracing::info!("Starting HTTP transport on {}", addr);

    let server = Arc::new(TypstMcpServer::new().with_timeout(timeout_ms));
    let (sse_tx, _) = broadcast::channel::<String>(100);

    let state = HttpServerState {
        server,
        timeout_ms,
        sse_tx,
    };

    // Configure CORS for browser clients
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // MCP JSON-RPC endpoint
        .route("/mcp", post(handle_mcp_request))
        // SSE endpoint for streaming notifications
        .route("/sse", get(handle_sse))
        // REST API endpoints for web clients
        .route("/api/templates", get(handle_api_templates))
        .route("/api/render", post(handle_api_render))
        // Health check
        .route("/health", get(handle_health))
        // Server info
        .route("/", get(handle_info))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(ServerError::IoError)?;

    tracing::info!("HTTP server listening on {}", addr);

    axum::serve(listener, app)
        .await
        .map_err(|e| ServerError::IoError(std::io::Error::other(e)))?;

    Ok(())
}

/// Handle MCP JSON-RPC requests
async fn handle_mcp_request(
    State(state): State<HttpServerState>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    tracing::debug!("HTTP request: {:?}", request.method);

    let response = process_request(&state, request).await;

    Json(response)
}

/// Process a JSON-RPC request
async fn process_request(state: &HttpServerState, request: JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone();

    match request.method.as_str() {
        "initialize" => handle_initialize(id, &state.server),
        "initialized" => JsonRpcResponse::success(id, json!({})),
        "tools/list" => handle_list_tools(id),
        "tools/call" => handle_call_tool(id, request.params, state.timeout_ms).await,
        "resources/list" => handle_list_resources(id),
        "resources/read" => handle_read_resource(id, request.params),
        "prompts/list" => handle_list_prompts(id),
        "prompts/get" => handle_get_prompt(id, request.params),
        "ping" => JsonRpcResponse::success(id, json!({})),
        _ => JsonRpcResponse::error(id, -32601, format!("Method not found: {}", request.method)),
    }
}

/// Handle SSE connections for streaming notifications
async fn handle_sse(
    State(state): State<HttpServerState>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.sse_tx.subscribe();

    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(msg) => {
                let event = Event::default().data(msg);
                Some((Ok(event), rx))
            }
            Err(_) => None,
        }
    });

    Sse::new(stream)
}

// ============================================================================
// REST API Endpoints (for web clients)
// ============================================================================

/// GET /api/templates - List available templates
async fn handle_api_templates() -> impl IntoResponse {
    let templates = templates::list_templates();
    Json(json!({
        "success": true,
        "templates": templates,
        "count": templates.len()
    }))
}

/// POST /api/render - Render a template to PDF/SVG/PNG
async fn handle_api_render(
    State(state): State<HttpServerState>,
    Json(request): Json<RenderApiRequest>,
) -> impl IntoResponse {
    // Build the source - either template URI or raw source
    let source = if request.is_template {
        format!("typst://templates/{}", request.template)
    } else {
        request.template
    };

    // Build render request
    let render_request = RenderRequest {
        source,
        inputs: request.inputs,
        assets: std::collections::HashMap::new(),
        format: match request.format.as_str() {
            "svg" => crate::compiler::OutputFormat::Svg,
            "png" => crate::compiler::OutputFormat::Png,
            _ => crate::compiler::OutputFormat::Pdf,
        },
        ppi: Some(144),
    };

    // Compile document
    match compile_document(render_request, state.timeout_ms).await {
        Ok(response) => {
            if let Some(artifact) = response.artifact {
                let warnings: Vec<String> = response
                    .warnings
                    .iter()
                    .map(|w| w.message.clone())
                    .collect();

                Json(RenderApiResponse {
                    success: true,
                    data: Some(artifact.data_base64),
                    error: None,
                    warnings: if warnings.is_empty() {
                        None
                    } else {
                        Some(warnings)
                    },
                })
            } else {
                let errors: Vec<String> =
                    response.errors.iter().map(|e| e.message.clone()).collect();
                Json(RenderApiResponse {
                    success: false,
                    data: None,
                    error: Some(errors.join("; ")),
                    warnings: None,
                })
            }
        }
        Err(e) => Json(RenderApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            warnings: None,
        }),
    }
}

/// Health check endpoint
async fn handle_health() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "service": "typst-mcp-server"
    }))
}

/// Server info endpoint
async fn handle_info(State(state): State<HttpServerState>) -> impl IntoResponse {
    Json(json!({
        "name": state.server.name(),
        "version": state.server.version(),
        "transport": "http",
        "endpoints": {
            "mcp": "/mcp",
            "sse": "/sse",
            "api_templates": "/api/templates",
            "api_render": "/api/render",
            "health": "/health"
        }
    }))
}

// Request handlers - same logic as stdio transport

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

    fn create_test_state() -> HttpServerState {
        let server = Arc::new(TypstMcpServer::new());
        let (sse_tx, _) = broadcast::channel(10);
        HttpServerState {
            server,
            timeout_ms: 5000,
            sse_tx,
        }
    }

    #[tokio::test]
    async fn test_process_ping_request() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "ping".to_string(),
            params: json!({}),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_process_initialize() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {}
            }),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"]["tools"].is_object());
        assert!(result["capabilities"]["resources"].is_object());
        assert!(result["capabilities"]["prompts"].is_object());
        assert!(result["serverInfo"]["name"].is_string());
        assert!(result["serverInfo"]["version"].is_string());
    }

    #[tokio::test]
    async fn test_process_initialized() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialized".to_string(),
            params: json!({}),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_process_tools_list() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert!(!tools.is_empty());

        // Check expected tools exist
        let tool_names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(tool_names.contains(&"render_document"));
        assert!(tool_names.contains(&"validate_syntax"));
        assert!(tool_names.contains(&"list_fonts"));
        assert!(tool_names.contains(&"list_templates"));
    }

    #[tokio::test]
    async fn test_process_tools_call_validate_syntax() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "validate_syntax",
                "arguments": {
                    "source": "#let x = 1\nHello, World!"
                }
            }),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        assert_eq!(result["isError"], false);
    }

    #[tokio::test]
    async fn test_process_tools_call_list_fonts() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "list_fonts",
                "arguments": {}
            }),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        assert_eq!(result["isError"], false);
        assert!(result["content"].is_array());
    }

    #[tokio::test]
    async fn test_process_tools_call_list_templates() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(5)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "list_templates",
                "arguments": {}
            }),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        assert_eq!(result["isError"], false);
    }

    #[tokio::test]
    async fn test_process_resources_list() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(6)),
            method: "resources/list".to_string(),
            params: json!({}),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        let resources = result.get("resources").unwrap().as_array().unwrap();
        assert!(!resources.is_empty());
    }

    #[tokio::test]
    async fn test_process_resources_read_template() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(7)),
            method: "resources/read".to_string(),
            params: json!({
                "uri": "typst://templates/invoice"
            }),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        let contents = result.get("contents").unwrap().as_array().unwrap();
        assert!(!contents.is_empty());
    }

    #[tokio::test]
    async fn test_process_resources_read_fonts() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(8)),
            method: "resources/read".to_string(),
            params: json!({
                "uri": "typst://fonts"
            }),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_process_resources_read_invalid() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(9)),
            method: "resources/read".to_string(),
            params: json!({
                "uri": "typst://invalid/path"
            }),
        };

        let response = process_request(&state, request).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32602);
    }

    #[tokio::test]
    async fn test_process_prompts_list() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(10)),
            method: "prompts/list".to_string(),
            params: json!({}),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        let prompts = result.get("prompts").unwrap().as_array().unwrap();
        assert!(!prompts.is_empty());

        // Check expected prompts exist
        let prompt_names: Vec<&str> = prompts
            .iter()
            .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(prompt_names.contains(&"generate_invoice"));
        assert!(prompt_names.contains(&"generate_florida_lease"));
    }

    #[tokio::test]
    async fn test_process_prompts_get() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(11)),
            method: "prompts/get".to_string(),
            params: json!({
                "name": "generate_invoice",
                "arguments": {
                    "company_name": "Test Corp"
                }
            }),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        let messages = result.get("messages").unwrap().as_array().unwrap();
        assert!(!messages.is_empty());
    }

    #[tokio::test]
    async fn test_process_prompts_get_invalid() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(12)),
            method: "prompts/get".to_string(),
            params: json!({
                "name": "nonexistent_prompt",
                "arguments": {}
            }),
        };

        let response = process_request(&state, request).await;
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_process_unknown_method() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(13)),
            method: "unknown/method".to_string(),
            params: json!({}),
        };

        let response = process_request(&state, request).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn test_process_null_id() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "ping".to_string(),
            params: json!({}),
        };

        let response = process_request(&state, request).await;
        assert!(response.id.is_none());
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_process_render_document_simple() {
        let state = create_test_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(14)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "render_document",
                "arguments": {
                    "source": "= Hello World\n\nThis is a test document.",
                    "format": "pdf"
                }
            }),
        };

        let response = process_request(&state, request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        // Either success or error in result, but should not fail at protocol level
        assert!(result.get("content").is_some() || result.get("isError").is_some());
    }
}
