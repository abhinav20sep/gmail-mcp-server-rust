//! Gmail utility functions
//!
//! Email creation, validation, and content extraction utilities.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

use crate::error::{Result, ValidationError};
use crate::gmail::types::{EmailAttachment, EmailContent, MessagePart};

/// Validate an email address
pub fn validate_email(email: &str) -> bool {
    // Simple regex-like validation
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);

    // Check basic requirements
    !local.is_empty()
        && !domain.is_empty()
        && !local.contains(' ')
        && !domain.contains(' ')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

/// Encode text for MIME header (RFC 2047)
pub fn encode_mime_header(text: &str) -> String {
    // Check if encoding is needed (non-ASCII characters)
    if text.chars().all(|c| c.is_ascii() && c != '\r' && c != '\n') {
        return text.to_string();
    }

    // Use MIME Words encoding (RFC 2047) - Base64 variant
    format!(
        "=?UTF-8?B?{}?=",
        base64::engine::general_purpose::STANDARD.encode(text.as_bytes())
    )
}

/// Encode a raw email message for Gmail API (base64url, no padding)
pub fn encode_raw_message(message: &str) -> String {
    URL_SAFE_NO_PAD.encode(message.as_bytes())
}

/// Decode base64url data from Gmail API
/// Handles both padded and non-padded base64url encoding
pub fn decode_base64url(data: &str) -> Result<Vec<u8>> {
    // First try without padding (Gmail API typically returns this)
    URL_SAFE_NO_PAD
        .decode(data)
        .or_else(|_| {
            // If that fails, try with standard URL-safe base64 (with padding)
            base64::engine::general_purpose::URL_SAFE.decode(data)
        })
        .or_else(|_| {
            // Last resort: try standard base64
            base64::engine::general_purpose::STANDARD.decode(data)
        })
        .map_err(|e| crate::error::GmailMcpError::Validation(ValidationError::InvalidParameter {
            name: "base64 data".to_string(),
            message: e.to_string(),
        }))
}

/// Decode base64url data to string
pub fn decode_base64url_string(data: &str) -> Result<String> {
    let bytes = decode_base64url(data)?;
    String::from_utf8(bytes).map_err(|e| {
        crate::error::GmailMcpError::Validation(ValidationError::InvalidParameter {
            name: "UTF-8 content".to_string(),
            message: e.to_string(),
        })
    })
}

/// Recursively extract email body content from MIME message parts
pub fn extract_email_content(message_part: &MessagePart) -> EmailContent {
    let mut content = EmailContent::default();

    let mime_type = message_part.mime_type.as_deref().unwrap_or("");
    
    // If the part has a body with data, process it based on MIME type
    if let Some(ref body) = message_part.body {
        if let Some(ref data) = body.data {
            // Only decode text-based content, skip binary attachments
            if mime_type.starts_with("text/") {
                match decode_base64url_string(data) {
                    Ok(decoded) => {
                        if mime_type == "text/plain" {
                            content.text = decoded;
                        } else if mime_type == "text/html" {
                            content.html = decoded;
                        }
                    }
                    Err(e) => {
                        // Log decode errors but continue processing
                        tracing::debug!("Failed to decode {} part: {}", mime_type, e);
                    }
                }
            }
        }
    }

    // If the part has nested parts, recursively process them
    // This handles multipart/alternative, multipart/mixed, multipart/related, etc.
    for part in &message_part.parts {
        let nested = extract_email_content(part);
        if !nested.text.is_empty() {
            if content.text.is_empty() {
                content.text = nested.text;
            } else {
                content.text.push_str(&nested.text);
            }
        }
        if !nested.html.is_empty() {
            if content.html.is_empty() {
                content.html = nested.html;
            } else {
                content.html.push_str(&nested.html);
            }
        }
    }

    content
}

/// Extract attachment information from message parts
pub fn extract_attachments(message_part: &MessagePart) -> Vec<EmailAttachment> {
    let mut attachments = Vec::new();
    extract_attachments_recursive(message_part, &mut attachments);
    attachments
}

fn extract_attachments_recursive(part: &MessagePart, attachments: &mut Vec<EmailAttachment>) {
    if let Some(ref body) = part.body {
        if let Some(ref attachment_id) = body.attachment_id {
            let filename = part
                .filename
                .clone()
                .unwrap_or_else(|| format!("attachment-{}", attachment_id));

            attachments.push(EmailAttachment {
                id: attachment_id.clone(),
                filename,
                mime_type: part
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
                size: body.size,
            });
        }
    }

    for subpart in &part.parts {
        extract_attachments_recursive(subpart, attachments);
    }
}

/// Find header value by name (case-insensitive)
pub fn find_header<'a>(part: &'a MessagePart, name: &str) -> Option<&'a str> {
    part.headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

/// Email content types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MimeType {
    TextPlain,
    TextHtml,
    MultipartAlternative,
    #[allow(dead_code)] // Reserved for future attachment handling
    MultipartMixed,
}

impl MimeType {
    /// Get the MIME type string
    #[allow(dead_code)] // Reserved for future use
    pub fn as_str(&self) -> &'static str {
        match self {
            MimeType::TextPlain => "text/plain",
            MimeType::TextHtml => "text/html",
            MimeType::MultipartAlternative => "multipart/alternative",
            MimeType::MultipartMixed => "multipart/mixed",
        }
    }
}

/// Email attachment data
#[derive(Debug, Clone)]
pub struct AttachmentData {
    /// Filename
    pub filename: String,
    /// MIME type
    pub mime_type: String,
    /// File content (raw bytes)
    pub data: Vec<u8>,
}

/// Parameters for creating an email message
#[derive(Debug, Clone)]
pub struct EmailParams {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub html_body: Option<String>,
    pub mime_type: Option<MimeType>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub thread_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub attachments: Option<Vec<AttachmentData>>,
}

/// Load an attachment from a file path
pub fn load_attachment(path: &str) -> Result<AttachmentData> {
    use std::path::Path;

    let path = Path::new(path);
    if !path.exists() {
        return Err(crate::error::GmailMcpError::Validation(
            ValidationError::FileNotFound {
                path: path.display().to_string(),
            },
        ));
    }

    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "attachment".to_string());

    let data = std::fs::read(path)?;

    // Guess MIME type from extension
    let mime_type = match path.extension().and_then(|e| e.to_str()) {
        Some("pdf") => "application/pdf",
        Some("doc") => "application/msword",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("xls") => "application/vnd.ms-excel",
        Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("txt") => "text/plain",
        Some("html") | Some("htm") => "text/html",
        Some("csv") => "text/csv",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("zip") => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string();

    Ok(AttachmentData {
        filename,
        mime_type,
        data,
    })
}

/// Create an email message with optional attachments
pub fn create_email_message(params: &EmailParams) -> Result<String> {
    // Validate email addresses
    for email in &params.to {
        if !validate_email(email) {
            return Err(crate::error::GmailMcpError::Validation(
                ValidationError::InvalidEmail {
                    email: email.clone(),
                },
            ));
        }
    }

    let encoded_subject = encode_mime_header(&params.subject);
    let has_attachments = params
        .attachments
        .as_ref()
        .map(|a| !a.is_empty())
        .unwrap_or(false);

    // Determine content type
    let mime_type = params.mime_type.unwrap_or(MimeType::TextPlain);
    let use_html = params.html_body.is_some() && mime_type != MimeType::TextPlain;

    let mut lines = Vec::new();

    // Headers
    lines.push("From: me".to_string());
    lines.push(format!("To: {}", params.to.join(", ")));

    if let Some(ref cc) = params.cc {
        if !cc.is_empty() {
            lines.push(format!("Cc: {}", cc.join(", ")));
        }
    }

    if let Some(ref bcc) = params.bcc {
        if !bcc.is_empty() {
            lines.push(format!("Bcc: {}", bcc.join(", ")));
        }
    }

    lines.push(format!("Subject: {}", encoded_subject));

    if let Some(ref in_reply_to) = params.in_reply_to {
        lines.push(format!("In-Reply-To: {}", in_reply_to));
        lines.push(format!("References: {}", in_reply_to));
    }

    lines.push("MIME-Version: 1.0".to_string());

    if has_attachments {
        // Multipart/mixed for attachments
        let mixed_boundary = format!("----=_MixedPart_{}", generate_boundary());
        lines.push(format!(
            "Content-Type: multipart/mixed; boundary=\"{}\"",
            mixed_boundary
        ));
        lines.push(String::new());

        // Text content part
        lines.push(format!("--{}", mixed_boundary));

        if use_html {
            // Multipart alternative for text + HTML
            let alt_boundary = format!("----=_AltPart_{}", generate_boundary());
            lines.push(format!(
                "Content-Type: multipart/alternative; boundary=\"{}\"",
                alt_boundary
            ));
            lines.push(String::new());

            // Plain text
            lines.push(format!("--{}", alt_boundary));
            lines.push("Content-Type: text/plain; charset=UTF-8".to_string());
            lines.push("Content-Transfer-Encoding: 7bit".to_string());
            lines.push(String::new());
            lines.push(params.body.clone());
            lines.push(String::new());

            // HTML
            lines.push(format!("--{}", alt_boundary));
            lines.push("Content-Type: text/html; charset=UTF-8".to_string());
            lines.push("Content-Transfer-Encoding: 7bit".to_string());
            lines.push(String::new());
            lines.push(params.html_body.clone().unwrap_or_else(|| params.body.clone()));
            lines.push(String::new());

            lines.push(format!("--{}--", alt_boundary));
        } else if mime_type == MimeType::TextHtml {
            lines.push("Content-Type: text/html; charset=UTF-8".to_string());
            lines.push("Content-Transfer-Encoding: 7bit".to_string());
            lines.push(String::new());
            lines.push(params.html_body.clone().unwrap_or_else(|| params.body.clone()));
        } else {
            lines.push("Content-Type: text/plain; charset=UTF-8".to_string());
            lines.push("Content-Transfer-Encoding: 7bit".to_string());
            lines.push(String::new());
            lines.push(params.body.clone());
        }
        lines.push(String::new());

        // Attachment parts
        if let Some(ref attachments) = params.attachments {
            for attachment in attachments {
                lines.push(format!("--{}", mixed_boundary));
                lines.push(format!(
                    "Content-Type: {}; name=\"{}\"",
                    attachment.mime_type,
                    encode_mime_header(&attachment.filename)
                ));
                lines.push("Content-Transfer-Encoding: base64".to_string());
                lines.push(format!(
                    "Content-Disposition: attachment; filename=\"{}\"",
                    encode_mime_header(&attachment.filename)
                ));
                lines.push(String::new());

                // Base64 encode the attachment data, wrapped at 76 chars
                let encoded = base64::engine::general_purpose::STANDARD.encode(&attachment.data);
                for chunk in encoded.as_bytes().chunks(76) {
                    lines.push(String::from_utf8_lossy(chunk).to_string());
                }
                lines.push(String::new());
            }
        }

        // Close mixed boundary
        lines.push(format!("--{}--", mixed_boundary));
    } else if use_html {
        // Multipart alternative (no attachments)
        let boundary = format!("----=_NextPart_{}", generate_boundary());
        lines.push(format!(
            "Content-Type: multipart/alternative; boundary=\"{}\"",
            boundary
        ));
        lines.push(String::new());

        // Plain text part
        lines.push(format!("--{}", boundary));
        lines.push("Content-Type: text/plain; charset=UTF-8".to_string());
        lines.push("Content-Transfer-Encoding: 7bit".to_string());
        lines.push(String::new());
        lines.push(params.body.clone());
        lines.push(String::new());

        // HTML part
        lines.push(format!("--{}", boundary));
        lines.push("Content-Type: text/html; charset=UTF-8".to_string());
        lines.push("Content-Transfer-Encoding: 7bit".to_string());
        lines.push(String::new());
        lines.push(params.html_body.clone().unwrap_or_else(|| params.body.clone()));
        lines.push(String::new());

        // Close boundary
        lines.push(format!("--{}--", boundary));
    } else if mime_type == MimeType::TextHtml {
        // HTML only
        lines.push("Content-Type: text/html; charset=UTF-8".to_string());
        lines.push("Content-Transfer-Encoding: 7bit".to_string());
        lines.push(String::new());
        lines.push(params.html_body.clone().unwrap_or_else(|| params.body.clone()));
    } else {
        // Plain text
        lines.push("Content-Type: text/plain; charset=UTF-8".to_string());
        lines.push("Content-Transfer-Encoding: 7bit".to_string());
        lines.push(String::new());
        lines.push(params.body.clone());
    }

    Ok(lines.join("\r\n"))
}

/// Generate a random boundary string for multipart messages
fn generate_boundary() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", timestamp)
}

/// Format file size for display
pub fn format_size(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("test@example.com"));
        assert!(validate_email("user.name@domain.co.uk"));
        assert!(validate_email("a@b.co"));
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(!validate_email("not-an-email"));
        assert!(!validate_email("@domain.com"));
        assert!(!validate_email("user@"));
        assert!(!validate_email("user@.com"));
        assert!(!validate_email("user@domain."));
    }

    #[test]
    fn test_encode_mime_header_ascii() {
        let text = "Hello World";
        assert_eq!(encode_mime_header(text), text);
    }

    #[test]
    fn test_encode_mime_header_unicode() {
        let text = "Héllo Wörld";
        let encoded = encode_mime_header(text);
        assert!(encoded.starts_with("=?UTF-8?B?"));
        assert!(encoded.ends_with("?="));
    }

    #[test]
    fn test_decode_base64url() {
        let encoded = "SGVsbG8gV29ybGQ"; // "Hello World" in base64url
        let decoded = decode_base64url_string(encoded).unwrap();
        assert_eq!(decoded, "Hello World");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 bytes");
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(1536), "2 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_create_email_message() {
        let params = EmailParams {
            to: vec!["test@example.com".to_string()],
            subject: "Test Subject".to_string(),
            body: "Test body".to_string(),
            html_body: None,
            mime_type: None,
            cc: None,
            bcc: None,
            thread_id: None,
            in_reply_to: None,
            attachments: None,
        };
        let message = create_email_message(&params).unwrap();
        assert!(message.contains("To: test@example.com"));
        assert!(message.contains("Subject: Test Subject"));
        assert!(message.contains("Test body"));
    }
}

