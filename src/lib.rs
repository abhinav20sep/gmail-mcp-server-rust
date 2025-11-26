//! Gmail MCP Server Library
//!
//! A Model Context Protocol (MCP) server for Gmail integration.
//! Provides tools for reading, sending, and managing emails via the Gmail API.

pub mod config;
pub mod error;
pub mod gmail;
pub mod mcp;

pub use config::Config;
pub use error::{GmailMcpError, Result};

