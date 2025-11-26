//! Gmail MCP Server - Rust Implementation
//!
//! A Model Context Protocol (MCP) server for Gmail integration.
//! Provides tools for reading, sending, and managing emails via the Gmail API.

use std::sync::Arc;

use clap::{Parser, Subcommand};

use gmail_mcp_server_rust::config::Config;
use gmail_mcp_server_rust::error::Result;
use gmail_mcp_server_rust::gmail::auth::Authenticator;
use gmail_mcp_server_rust::gmail::client::GmailClient;
use gmail_mcp_server_rust::mcp::server::McpServer;

/// Gmail MCP Server
#[derive(Parser)]
#[command(name = "gmail-mcp-server")]
#[command(author, version, about = "Gmail MCP Server - A Model Context Protocol server for Gmail")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Gmail (run this first)
    Auth {
        /// Custom OAuth callback URL
        #[arg(long)]
        callback_url: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    // Load configuration
    let config = Config::new()?;

    match cli.command {
        Some(Commands::Auth { callback_url: _ }) => {
            // Run authentication flow
            let authenticator = Authenticator::new(config).await?;
            authenticator.authenticate_interactive().await?;
            eprintln!("Authentication completed successfully!");
            std::process::exit(0);
        }
        None => {
            // Run MCP server
            run_server(config).await?;
        }
    }

    Ok(())
}

async fn run_server(config: Config) -> Result<()> {
    // Check for OAuth keys
    if !config.oauth_keys_exist() {
        eprintln!("Error: OAuth keys file not found.");
        eprintln!(
            "Please place gcp-oauth.keys.json in current directory or {}",
            config.config_dir.display()
        );
        std::process::exit(1);
    }

    // Initialize authenticator
    let authenticator = Authenticator::new(config).await?;

    // Check if we have credentials
    if !authenticator.is_authenticated().await {
        eprintln!("Error: Not authenticated. Please run 'gmail-mcp-server auth' first.");
        std::process::exit(1);
    }

    // Create Gmail client
    let gmail_client = Arc::new(GmailClient::new(Arc::new(authenticator)));

    // Create and run MCP server
    let mut server = McpServer::new(gmail_client);
    server.run_stdio().await?;

    Ok(())
}
