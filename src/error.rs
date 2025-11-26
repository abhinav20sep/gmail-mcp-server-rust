//! Error types for the Gmail MCP Server
//!
//! This module defines the error hierarchy for all operations in the server.

use thiserror::Error;

/// Main error type for the Gmail MCP Server
#[derive(Error, Debug)]
pub enum GmailMcpError {
    /// OAuth authentication errors
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    /// Gmail API errors
    #[error("Gmail API error: {0}")]
    Gmail(#[from] GmailApiError),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    /// MCP protocol errors
    #[error("MCP protocol error: {0}")]
    Mcp(#[from] McpError),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP client errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

/// OAuth authentication errors
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("OAuth keys file not found: {path}")]
    KeysFileNotFound { path: String },

    #[error("Invalid OAuth keys format: expected 'installed' or 'web' credentials")]
    InvalidKeysFormat,

    #[error("Credentials file not found: {path}")]
    CredentialsNotFound { path: String },

    #[error("Failed to refresh access token: {message}")]
    TokenRefreshFailed { message: String },

    #[error("OAuth callback error: {message}")]
    CallbackError { message: String },

    #[error("No authorization code provided")]
    NoAuthCode,

    #[error("Token exchange failed: {message}")]
    TokenExchangeFailed { message: String },

    #[error("OAuth2 error: {0}")]
    OAuth2(String),
}

/// Gmail API errors
#[derive(Error, Debug)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum GmailApiError {
    #[error("Message not found: {message_id}")]
    MessageNotFound { message_id: String },

    #[error("Label not found: {label_id}")]
    LabelNotFound { label_id: String },

    #[error("Label already exists: {name}")]
    LabelAlreadyExists { name: String },

    #[error("Cannot delete system label: {label_id}")]
    CannotDeleteSystemLabel { label_id: String },

    #[error("Filter not found: {filter_id}")]
    FilterNotFound { filter_id: String },

    #[error("Invalid filter criteria: {message}")]
    InvalidFilterCriteria { message: String },

    #[error("Attachment not found: {attachment_id}")]
    AttachmentNotFound { attachment_id: String },

    #[error("API request failed: {message}")]
    RequestFailed { message: String },

    #[error("Rate limited: retry after {retry_after_secs} seconds")]
    RateLimited { retry_after_secs: u64 },

    #[error("Insufficient permissions: {scope}")]
    InsufficientPermissions { scope: String },
}

/// Configuration errors
#[derive(Error, Debug)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum ConfigError {
    #[error("Config directory not found: {path}")]
    DirNotFound { path: String },

    #[error("Failed to create config directory: {path}")]
    DirCreationFailed { path: String },

    #[error("Missing required environment variable: {var}")]
    MissingEnvVar { var: String },

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },
}

/// Validation errors
#[derive(Error, Debug)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum ValidationError {
    #[error("Invalid email address: {email}")]
    InvalidEmail { email: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid parameter: {name} - {message}")]
    InvalidParameter { name: String, message: String },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Invalid MIME type: {mime_type}")]
    InvalidMimeType { mime_type: String },
}

/// MCP protocol errors
#[derive(Error, Debug)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum McpError {
    #[error("Unknown tool: {name}")]
    UnknownTool { name: String },

    #[error("Invalid tool arguments: {message}")]
    InvalidArguments { message: String },

    #[error("Protocol error: {message}")]
    ProtocolError { message: String },

    #[error("Transport error: {message}")]
    TransportError { message: String },
}

/// Result type alias for Gmail MCP operations
pub type Result<T> = std::result::Result<T, GmailMcpError>;

/// Convert yup-oauth2 errors to our AuthError
impl From<yup_oauth2::Error> for AuthError {
    fn from(err: yup_oauth2::Error) -> Self {
        AuthError::OAuth2(err.to_string())
    }
}

impl From<yup_oauth2::Error> for GmailMcpError {
    fn from(err: yup_oauth2::Error) -> Self {
        GmailMcpError::Auth(AuthError::from(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AuthError::KeysFileNotFound {
            path: "/path/to/keys.json".to_string(),
        };
        assert!(err.to_string().contains("/path/to/keys.json"));
    }

    #[test]
    fn test_error_conversion() {
        let auth_err = AuthError::NoAuthCode;
        let gmail_err: GmailMcpError = auth_err.into();
        assert!(matches!(gmail_err, GmailMcpError::Auth(_)));
    }
}
