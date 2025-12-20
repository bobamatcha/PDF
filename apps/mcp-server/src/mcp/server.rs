//! MCP Server core implementation

/// The Typst MCP Server
#[derive(Debug, Clone)]
pub struct TypstMcpServer {
    /// Server name
    name: String,
    /// Server version
    version: String,
    /// Compilation timeout in milliseconds
    timeout_ms: u64,
}

impl TypstMcpServer {
    /// Create a new Typst MCP Server
    pub fn new() -> Self {
        Self {
            name: "typst-mcp-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timeout_ms: 5000,
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Get server name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get server version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get timeout
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }
}

impl Default for TypstMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
