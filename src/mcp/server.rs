//! MCP Server implementation
//!
//! Implements the Model Context Protocol server for stdio transport.

use std::io::{BufRead, Write};
use std::sync::Arc;

use serde_json::Value;

use crate::error::Result;
use crate::gmail::client::GmailClient;
use crate::mcp::tools::ToolHandler;
use crate::mcp::types::*;

/// MCP Server info
const SERVER_NAME: &str = "gmail";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// MCP Server for Gmail
pub struct McpServer {
    /// Gmail client (kept for potential future use)
    #[allow(dead_code)]
    gmail_client: Arc<GmailClient>,

    /// Tool handler
    tool_handler: ToolHandler,

    /// Whether initialized
    initialized: bool,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(gmail_client: Arc<GmailClient>) -> Self {
        let tool_handler = ToolHandler::new(gmail_client.clone());

        Self {
            gmail_client,
            tool_handler,
            initialized: false,
        }
    }

    /// Run the server on stdio
    pub async fn run_stdio(&mut self) -> Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        let reader = stdin.lock();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match self.handle_message(&line).await {
                Ok(Some(response)) => {
                    let response_str = serde_json::to_string(&response)?;
                    writeln!(stdout, "{}", response_str)?;
                    stdout.flush()?;
                }
                Ok(None) => {
                    // Notification, no response needed
                }
                Err(e) => {
                    eprintln!("Error handling message: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Handle an incoming JSON-RPC message
    async fn handle_message(&mut self, message: &str) -> Result<Option<JsonRpcResponse>> {
        // Try to parse as request
        let request: JsonRpcRequest = match serde_json::from_str(message) {
            Ok(req) => req,
            Err(e) => {
                return Ok(Some(JsonRpcResponse::error(
                    RequestId::Number(0),
                    JsonRpcError::parse_error(e.to_string()),
                )));
            }
        };

        // Handle the request
        match request.method.as_str() {
            methods::INITIALIZE => {
                let result = self.handle_initialize(&request).await?;
                Ok(Some(JsonRpcResponse::success(request.id, result)))
            }
            methods::INITIALIZED => {
                self.initialized = true;
                Ok(None) // Notification, no response
            }
            methods::PING => {
                Ok(Some(JsonRpcResponse::success(
                    request.id,
                    serde_json::json!({}),
                )))
            }
            methods::LIST_TOOLS => {
                let result = self.handle_list_tools().await?;
                Ok(Some(JsonRpcResponse::success(request.id, result)))
            }
            methods::CALL_TOOL => {
                let result = self.handle_call_tool(&request).await;
                Ok(Some(JsonRpcResponse::success(request.id, result)))
            }
            _ => Ok(Some(JsonRpcResponse::error(
                request.id,
                JsonRpcError::method_not_found(&request.method),
            ))),
        }
    }

    /// Handle initialize request
    async fn handle_initialize(&self, _request: &JsonRpcRequest) -> Result<Value> {
        let result = InitializeResult {
            protocol_version: MCP_VERSION.to_string(),
            server_info: ServerInfo {
                name: SERVER_NAME.to_string(),
                version: SERVER_VERSION.to_string(),
            },
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {}),
                resources: None,
                prompts: None,
            },
        };

        Ok(serde_json::to_value(result)?)
    }

    /// Handle list tools request
    async fn handle_list_tools(&self) -> Result<Value> {
        let result = ListToolsResult {
            tools: self.tool_handler.list_tools(),
        };

        Ok(serde_json::to_value(result)?)
    }

    /// Handle call tool request
    async fn handle_call_tool(&self, request: &JsonRpcRequest) -> Value {
        let params: CallToolParams = match request.params.as_ref() {
            Some(p) => match serde_json::from_value(p.clone()) {
                Ok(params) => params,
                Err(e) => {
                    return serde_json::to_value(CallToolResult::error(format!(
                        "Invalid tool parameters: {}",
                        e
                    )))
                    .unwrap();
                }
            },
            None => {
                return serde_json::to_value(CallToolResult::error("Missing tool parameters"))
                    .unwrap();
            }
        };

        let result = self.tool_handler.call_tool(&params.name, params.arguments).await;
        serde_json::to_value(result).unwrap_or_else(|e| {
            serde_json::to_value(CallToolResult::error(e.to_string())).unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_info() {
        assert_eq!(SERVER_NAME, "gmail");
    }
}

