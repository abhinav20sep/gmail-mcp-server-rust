//! Configuration management for the Gmail MCP Server
//!
//! Handles paths, environment variables, and configuration loading.

use std::path::PathBuf;

use crate::error::{ConfigError, GmailMcpError, Result};

/// Configuration for the Gmail MCP Server
#[derive(Debug, Clone)]
pub struct Config {
    /// Directory for storing configuration files
    pub config_dir: PathBuf,

    /// Path to OAuth keys file (client credentials)
    pub oauth_path: PathBuf,

    /// Path to stored credentials (access/refresh tokens)
    pub credentials_path: PathBuf,

    /// OAuth callback URL
    pub oauth_callback_url: String,

    /// OAuth callback port
    pub oauth_callback_port: u16,

    /// Gmail API scopes
    pub scopes: Vec<String>,
}

impl Config {
    /// Create a new configuration with default paths
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;

        let oauth_path = std::env::var("GMAIL_OAUTH_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| config_dir.join("gcp-oauth.keys.json"));

        let credentials_path = std::env::var("GMAIL_CREDENTIALS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| config_dir.join("credentials.json"));

        let oauth_callback_port = std::env::var("GMAIL_OAUTH_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        let oauth_callback_url = format!("http://localhost:{}/oauth2callback", oauth_callback_port);

        Ok(Self {
            config_dir,
            oauth_path,
            credentials_path,
            oauth_callback_url,
            oauth_callback_port,
            scopes: vec![
                "https://www.googleapis.com/auth/gmail.modify".to_string(),
                "https://www.googleapis.com/auth/gmail.settings.basic".to_string(),
            ],
        })
    }

    /// Get the configuration directory, creating it if necessary
    fn get_config_dir() -> Result<PathBuf> {
        let config_dir = dirs::home_dir()
            .ok_or_else(|| {
                GmailMcpError::Config(ConfigError::DirNotFound {
                    path: "~".to_string(),
                })
            })?
            .join(".gmail-mcp");

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).map_err(|_| {
                GmailMcpError::Config(ConfigError::DirCreationFailed {
                    path: config_dir.display().to_string(),
                })
            })?;
        }

        Ok(config_dir)
    }

    /// Check if OAuth keys file exists
    pub fn oauth_keys_exist(&self) -> bool {
        self.oauth_path.exists()
    }

    /// Check if credentials (tokens) exist
    pub fn credentials_exist(&self) -> bool {
        self.credentials_path.exists()
    }

    /// Try to find OAuth keys in current directory and copy to config dir
    pub fn find_and_copy_oauth_keys(&self) -> Result<bool> {
        let local_oauth = std::env::current_dir()
            .map_err(GmailMcpError::Io)?
            .join("gcp-oauth.keys.json");

        if local_oauth.exists() && !self.oauth_keys_exist() {
            std::fs::copy(&local_oauth, &self.oauth_path).map_err(GmailMcpError::Io)?;
            return Ok(true);
        }

        Ok(false)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new().expect("Failed to create default config")
    }
}

/// Gmail API constants
pub mod gmail {
    /// Base URL for Gmail API
    pub const API_BASE_URL: &str = "https://gmail.googleapis.com/gmail/v1";

    /// User ID for the authenticated user
    pub const USER_ID: &str = "me";

    /// System label IDs (kept for reference/documentation)
    #[allow(dead_code)]
    pub mod labels {
        pub const INBOX: &str = "INBOX";
        pub const SENT: &str = "SENT";
        pub const TRASH: &str = "TRASH";
        pub const SPAM: &str = "SPAM";
        pub const STARRED: &str = "STARRED";
        pub const IMPORTANT: &str = "IMPORTANT";
        pub const UNREAD: &str = "UNREAD";
        pub const DRAFT: &str = "DRAFT";
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config::new();
        assert!(config.is_ok());
    }

    #[test]
    fn test_default_scopes() {
        let config = Config::new().unwrap();
        assert_eq!(config.scopes.len(), 2);
        assert!(config.scopes[0].contains("gmail.modify"));
    }
}

