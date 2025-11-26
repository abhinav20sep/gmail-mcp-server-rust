//! OAuth authentication for Gmail API
//!
//! Handles OAuth 2.0 authentication flow including:
//! - Loading client credentials
//! - Interactive browser-based authentication
//! - Token storage and refresh

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::Config;
use crate::error::{AuthError, GmailMcpError, Result};

/// OAuth client credentials
#[derive(Debug, Clone, Deserialize)]
pub struct OAuthKeys {
    /// Client ID
    pub client_id: String,

    /// Client secret
    pub client_secret: String,

    /// Auth URI
    pub auth_uri: String,

    /// Token URI
    pub token_uri: String,

    /// Redirect URIs
    #[serde(default)]
    #[allow(dead_code)]
    pub redirect_uris: Vec<String>,
}

/// OAuth keys file format (can be "installed" or "web")
#[derive(Debug, Deserialize)]
struct OAuthKeysFile {
    #[serde(alias = "web")]
    installed: Option<OAuthKeys>,
}

/// Stored credentials (tokens)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCredentials {
    /// Access token
    pub access_token: String,

    /// Refresh token
    pub refresh_token: Option<String>,

    /// Token type (usually "Bearer")
    #[serde(default = "default_token_type")]
    pub token_type: String,

    /// Expiry timestamp (Unix seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<i64>,

    /// Scopes
    #[serde(default)]
    pub scope: String,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

/// Token response from OAuth token endpoint
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default = "default_token_type")]
    token_type: String,
    expires_in: Option<i64>,
    #[serde(default)]
    scope: String,
}

/// OAuth authenticator
pub struct Authenticator {
    /// Configuration
    config: Config,

    /// HTTP client
    http_client: reqwest::Client,

    /// OAuth client credentials
    keys: OAuthKeys,

    /// Current credentials (tokens)
    credentials: Arc<RwLock<Option<StoredCredentials>>>,
}

impl Authenticator {
    /// Create a new authenticator
    pub async fn new(config: Config) -> Result<Self> {
        // Try to find and copy OAuth keys from current directory
        config.find_and_copy_oauth_keys()?;

        // Load OAuth keys
        let keys = Self::load_oauth_keys(&config.oauth_path)?;

        let http_client = reqwest::Client::new();

        let auth = Self {
            config,
            http_client,
            keys,
            credentials: Arc::new(RwLock::new(None)),
        };

        // Try to load existing credentials
        if auth.config.credentials_exist() {
            if let Ok(creds) = auth.load_credentials().await {
                *auth.credentials.write().await = Some(creds);
            }
        }

        Ok(auth)
    }

    /// Load OAuth keys from file
    fn load_oauth_keys(path: &Path) -> Result<OAuthKeys> {
        if !path.exists() {
            return Err(GmailMcpError::Auth(AuthError::KeysFileNotFound {
                path: path.display().to_string(),
            }));
        }

        let content = std::fs::read_to_string(path)?;
        let keys_file: OAuthKeysFile = serde_json::from_str(&content)?;

        keys_file.installed.ok_or_else(|| {
            GmailMcpError::Auth(AuthError::InvalidKeysFormat)
        })
    }

    /// Load stored credentials from file
    async fn load_credentials(&self) -> Result<StoredCredentials> {
        let content = tokio::fs::read_to_string(&self.config.credentials_path).await?;
        let creds: StoredCredentials = serde_json::from_str(&content)?;
        Ok(creds)
    }

    /// Save credentials to file
    async fn save_credentials(&self, credentials: &StoredCredentials) -> Result<()> {
        let content = serde_json::to_string_pretty(credentials)?;
        tokio::fs::write(&self.config.credentials_path, content).await?;
        Ok(())
    }

    /// Check if we have valid credentials
    pub async fn is_authenticated(&self) -> bool {
        self.credentials.read().await.is_some()
    }

    /// Get a valid access token, refreshing if necessary
    pub async fn get_access_token(&self) -> Result<String> {
        let creds = self.credentials.read().await;

        if let Some(ref creds) = *creds {
            // Check if token is expired or about to expire (within 5 minutes)
            if let Some(expiry) = creds.expiry_date {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                if expiry - now < 300 {
                    // Token expired or expiring soon, try to refresh
                    let _ = creds;
                    return self.refresh_token().await;
                }
            }

            return Ok(creds.access_token.clone());
        }

        Err(GmailMcpError::Auth(AuthError::CredentialsNotFound {
            path: self.config.credentials_path.display().to_string(),
        }))
    }

    /// Refresh the access token using the refresh token
    async fn refresh_token(&self) -> Result<String> {
        let creds = self.credentials.read().await;
        let refresh_token = creds
            .as_ref()
            .and_then(|c| c.refresh_token.clone())
            .ok_or_else(|| {
                GmailMcpError::Auth(AuthError::TokenRefreshFailed {
                    message: "No refresh token available".to_string(),
                })
            })?;
        drop(creds);

        let params = [
            ("client_id", self.keys.client_id.as_str()),
            ("client_secret", self.keys.client_secret.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let response = self
            .http_client
            .post(&self.keys.token_uri)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(GmailMcpError::Auth(AuthError::TokenRefreshFailed {
                message: text,
            }));
        }

        let token_response: TokenResponse = response.json().await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let new_credentials = StoredCredentials {
            access_token: token_response.access_token.clone(),
            refresh_token: token_response.refresh_token.or(Some(refresh_token)),
            token_type: token_response.token_type,
            expiry_date: token_response.expires_in.map(|e| now + e),
            scope: token_response.scope,
        };

        self.save_credentials(&new_credentials).await?;
        *self.credentials.write().await = Some(new_credentials.clone());

        Ok(new_credentials.access_token)
    }

    /// Generate the authorization URL
    pub fn generate_auth_url(&self) -> String {
        let scopes = self.config.scopes.join(" ");
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
            self.keys.auth_uri,
            urlencoding::encode(&self.keys.client_id),
            urlencoding::encode(&self.config.oauth_callback_url),
            urlencoding::encode(&scopes)
        )
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(&self, code: &str) -> Result<StoredCredentials> {
        let params = [
            ("client_id", self.keys.client_id.as_str()),
            ("client_secret", self.keys.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", self.config.oauth_callback_url.as_str()),
        ];

        let response = self
            .http_client
            .post(&self.keys.token_uri)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(GmailMcpError::Auth(AuthError::TokenExchangeFailed {
                message: text,
            }));
        }

        let token_response: TokenResponse = response.json().await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let credentials = StoredCredentials {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response.token_type,
            expiry_date: token_response.expires_in.map(|e| now + e),
            scope: token_response.scope,
        };

        self.save_credentials(&credentials).await?;
        *self.credentials.write().await = Some(credentials.clone());

        Ok(credentials)
    }

    /// Run interactive authentication flow with local HTTP server
    pub async fn authenticate_interactive(&self) -> Result<()> {
        use axum::{extract::Query, response::Html, routing::get, Router};
        use std::collections::HashMap;
        use tokio::sync::oneshot;

        let auth_url = self.generate_auth_url();
        eprintln!("\nPlease visit this URL to authenticate:");
        eprintln!("{}\n", auth_url);

        // Try to open in browser
        if let Err(e) = open::that(&auth_url) {
            eprintln!("Could not open browser automatically: {}", e);
            eprintln!("Please open the URL manually.");
        }

        // Create channel for receiving the auth code
        let (tx, rx) = oneshot::channel::<String>();
        let tx = Arc::new(std::sync::Mutex::new(Some(tx)));

        // Create the callback handler
        let tx_clone = tx.clone();
        let callback_handler = move |Query(params): Query<HashMap<String, String>>| async move {
            if let Some(code) = params.get("code") {
                if let Some(tx) = tx_clone.lock().unwrap().take() {
                    let _ = tx.send(code.clone());
                }
                Html("<html><body><h1>Authentication successful!</h1><p>You can close this window.</p></body></html>")
            } else {
                Html("<html><body><h1>Authentication failed</h1><p>No authorization code received.</p></body></html>")
            }
        };

        let app = Router::new().route("/oauth2callback", get(callback_handler));

        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], self.config.oauth_callback_port));
        let listener = tokio::net::TcpListener::bind(addr).await?;

        eprintln!("Waiting for authentication callback on port {}...", self.config.oauth_callback_port);

        // Run server until we receive the code
        let server = axum::serve(listener, app);

        tokio::select! {
            result = server => {
                if let Err(e) = result {
                    return Err(GmailMcpError::Auth(AuthError::CallbackError {
                        message: e.to_string(),
                    }));
                }
            }
            code = rx => {
                match code {
                    Ok(code) => {
                        eprintln!("Received authorization code, exchanging for tokens...");
                        self.exchange_code(&code).await?;
                        eprintln!("Authentication completed successfully!");
                    }
                    Err(_) => {
                        return Err(GmailMcpError::Auth(AuthError::NoAuthCode));
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_keys_deserialize() {
        let json = r#"{
            "installed": {
                "client_id": "test-client-id",
                "client_secret": "test-secret",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "redirect_uris": ["http://localhost"]
            }
        }"#;

        let keys_file: OAuthKeysFile = serde_json::from_str(json).unwrap();
        assert!(keys_file.installed.is_some());
        assert_eq!(keys_file.installed.unwrap().client_id, "test-client-id");
    }

    #[test]
    fn test_stored_credentials_serialize() {
        let creds = StoredCredentials {
            access_token: "test-token".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            token_type: "Bearer".to_string(),
            expiry_date: Some(1234567890),
            scope: "https://www.googleapis.com/auth/gmail.modify".to_string(),
        };

        let json = serde_json::to_string(&creds).unwrap();
        assert!(json.contains("test-token"));
        assert!(json.contains("refresh-token"));
    }
}

