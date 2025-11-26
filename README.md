# Gmail MCP Server (Rust)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

A high-performance Rust implementation of the Gmail MCP Server for Claude/AI integration via the Model Context Protocol.

> **Inspired by:** [GongRzhe/Gmail-MCP-Server](https://github.com/GongRzhe/Gmail-MCP-Server) (TypeScript)
> 
> This is a complete Rust rewrite providing better performance, memory safety, and native binaries.

## Features

- **OAuth 2.0 Authentication**: Secure Google OAuth flow with token refresh
- **Email Operations**: Send, draft, read, search, modify, and delete emails
- **Attachment Support**: Send emails with file attachments, download attachments
- **Label Management**: Create, update, delete, and list Gmail labels
- **Filter Management**: Create filters with criteria and actions, includes templates
- **Batch Operations**: Efficient batch modify/delete for multiple messages
- **Full MCP Protocol**: Implements the Model Context Protocol for AI tool integration
- **Comprehensive Tests**: 61 tests (24 unit + 37 integration) with 0 clippy warnings

## Available Tools (19 total)

| Tool | Description |
|------|-------------|
| `send_email` | Send a new email (with optional attachments) |
| `draft_email` | Create a draft email |
| `read_email` | Read a specific email by ID |
| `search_emails` | Search emails with Gmail query syntax |
| `modify_email` | Add/remove labels from an email |
| `delete_email` | Move email to trash |
| `list_email_labels` | List all Gmail labels |
| `batch_modify_emails` | Modify labels on multiple emails |
| `batch_delete_emails` | Delete multiple emails |
| `create_label` | Create a new label |
| `update_label` | Update a label's properties |
| `delete_label` | Delete a label |
| `get_or_create_label` | Get existing or create new label |
| `create_filter` | Create a new filter |
| `list_filters` | List all filters |
| `get_filter` | Get a specific filter |
| `delete_filter` | Delete a filter |
| `create_filter_from_template` | Create filter from predefined templates |
| `download_attachment` | Download an email attachment |

## Prerequisites

- Rust 1.70+ (installed via [rustup](https://rustup.rs/))
- Google Cloud Project with Gmail API enabled
- OAuth 2.0 credentials (Desktop app type)

## Installation

### Build from Source

```bash
git clone https://github.com/abhinav-copilot/gmail-mcp-server-rust.git
cd gmail-mcp-server-rust
cargo build --release
```

The binary will be at `target/release/gmail-mcp-server`.

## Setup

### 1. Create Google Cloud OAuth Credentials

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select existing
3. Enable the Gmail API
4. Go to Credentials → Create Credentials → OAuth 2.0 Client IDs
5. Select "Desktop app" as application type
6. Download the JSON file

### 2. Configure Credentials

Save the downloaded OAuth file as either:
- `~/.gmail-mcp/gcp-oauth.keys.json` (recommended)
- `./gcp-oauth.keys.json` (current directory)

Or set environment variable:
```bash
export GMAIL_OAUTH_PATH=/path/to/gcp-oauth.keys.json
```

### 3. Authenticate

```bash
# Run authentication flow
./gmail-mcp-server auth
```

This opens your browser for Google OAuth consent. After approval, credentials are stored in `~/.gmail-mcp/credentials.json`.

## Usage

### Standalone Server

```bash
# Start the MCP server (communicates via stdio)
./gmail-mcp-server
```

### With Claude Desktop / Cursor

Add to your MCP configuration (e.g., `~/.config/claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "gmail": {
      "command": "/path/to/gmail-mcp-server",
      "args": []
    }
  }
}
```

### Sending Emails with Attachments

The `send_email` and `draft_email` tools support file attachments:

```json
{
  "name": "send_email",
  "arguments": {
    "to": ["recipient@example.com"],
    "subject": "Document attached",
    "body": "Please see the attached file.",
    "attachments": ["/path/to/document.pdf", "/path/to/image.png"]
  }
}
```

Supported attachment types: PDF, Word, Excel, images (PNG, JPG, GIF), text, CSV, JSON, XML, ZIP.

## Project Structure

```
src/
├── lib.rs               # Library crate root
├── main.rs              # Binary entry point, CLI handling
├── config.rs            # Configuration management
├── error.rs             # Error types with thiserror
├── gmail/
│   ├── mod.rs           # Gmail module exports
│   ├── types.rs         # Gmail API types (serde)
│   ├── auth.rs          # OAuth 2.0 authentication
│   ├── client.rs        # Gmail API client
│   ├── utils.rs         # Email utilities, attachment support
│   ├── labels.rs        # Label management
│   └── filters.rs       # Filter management
└── mcp/
    ├── mod.rs           # MCP module exports
    ├── types.rs         # MCP protocol types
    ├── server.rs        # MCP server (stdio transport)
    └── tools.rs         # Tool definitions & handlers
tests/
└── integration_tests.rs # Integration tests
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Lint
cargo clippy

# Format
cargo fmt

# Build optimized release
cargo build --release
```

## Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `GMAIL_OAUTH_PATH` | Path to OAuth keys file | `~/.gmail-mcp/gcp-oauth.keys.json` |
| `GMAIL_CREDENTIALS_PATH` | Path to stored tokens | `~/.gmail-mcp/credentials.json` |
| `GMAIL_OAUTH_PORT` | OAuth callback port | `3000` |
| `RUST_LOG` | Log level (trace, debug, info, warn, error) | `info` |

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **Original Implementation**: [GongRzhe/Gmail-MCP-Server](https://github.com/GongRzhe/Gmail-MCP-Server) - The TypeScript implementation that inspired this Rust port
- **Model Context Protocol**: [modelcontextprotocol.io](https://modelcontextprotocol.io) - The protocol specification
- **Anthropic**: For creating Claude and the MCP ecosystem
