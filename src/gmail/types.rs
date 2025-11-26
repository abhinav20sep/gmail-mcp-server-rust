//! Gmail API type definitions
//!
//! These types mirror the Gmail API responses and are used for serialization/deserialization.

use serde::{Deserialize, Serialize};

/// A Gmail message part (MIME part)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MessagePart {
    /// Part ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_id: Option<String>,

    /// MIME type of this part
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Filename for attachments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// Headers for this part
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<Header>,

    /// Body of this part
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<MessagePartBody>,

    /// Nested parts (for multipart messages)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<MessagePart>,
}

/// Header in a message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    /// Header name
    pub name: String,

    /// Header value
    pub value: String,
}

/// Body of a message part
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MessagePartBody {
    /// Attachment ID (if this is an attachment)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_id: Option<String>,

    /// Size in bytes
    #[serde(default)]
    pub size: i64,

    /// Base64url-encoded data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// A Gmail message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// Message ID
    pub id: String,

    /// Thread ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,

    /// Label IDs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub label_ids: Vec<String>,

    /// Snippet (preview text)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,

    /// Message payload (MIME structure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<MessagePart>,

    /// Size estimate in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_estimate: Option<i64>,

    /// Raw RFC822 message (only with format=RAW)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,

    /// Internal date (epoch millis)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_date: Option<String>,
}

/// List of messages response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageList {
    /// Messages in this page
    #[serde(default)]
    pub messages: Vec<MessageRef>,

    /// Next page token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,

    /// Result size estimate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_size_estimate: Option<u32>,
}

/// Reference to a message (id and thread_id only)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRef {
    /// Message ID
    pub id: String,

    /// Thread ID
    pub thread_id: String,
}

/// A Gmail label
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    /// Label ID
    pub id: String,

    /// Label name
    pub name: String,

    /// Label type (system or user)
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub label_type: Option<String>,

    /// Message list visibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_list_visibility: Option<String>,

    /// Label list visibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_list_visibility: Option<String>,

    /// Total message count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages_total: Option<i32>,

    /// Unread message count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages_unread: Option<i32>,

    /// Label color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<LabelColor>,
}

/// Label color settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelColor {
    /// Text color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,

    /// Background color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
}

/// List of labels response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelList {
    /// Labels
    #[serde(default)]
    pub labels: Vec<Label>,
}

/// Request to create a label
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLabelRequest {
    /// Label name
    pub name: String,

    /// Message list visibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_list_visibility: Option<String>,

    /// Label list visibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_list_visibility: Option<String>,
}

/// Request to update a label
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLabelRequest {
    /// New name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Message list visibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_list_visibility: Option<String>,

    /// Label list visibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_list_visibility: Option<String>,
}

/// Request to modify message labels
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModifyMessageRequest {
    /// Label IDs to add
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_label_ids: Option<Vec<String>>,

    /// Label IDs to remove
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_label_ids: Option<Vec<String>>,
}

/// Gmail filter criteria
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FilterCriteria {
    /// Sender email to match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,

    /// Recipient email to match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,

    /// Subject to match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Search query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// Negated query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negated_query: Option<String>,

    /// Whether message has attachment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_attachment: Option<bool>,

    /// Whether to exclude chats
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_chats: Option<bool>,

    /// Size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,

    /// Size comparison operator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_comparison: Option<SizeComparison>,
}

/// Size comparison for filters
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SizeComparison {
    Unspecified,
    Smaller,
    Larger,
}

/// Gmail filter action
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FilterAction {
    /// Label IDs to add
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_label_ids: Option<Vec<String>>,

    /// Label IDs to remove
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_label_ids: Option<Vec<String>>,

    /// Email to forward to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward: Option<String>,
}

/// A Gmail filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    /// Filter ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Filter criteria
    pub criteria: FilterCriteria,

    /// Filter action
    pub action: FilterAction,
}

/// List of filters response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterList {
    /// Filters
    #[serde(default)]
    pub filter: Vec<Filter>,
}

/// Gmail draft
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    /// Draft ID
    pub id: String,

    /// The message
    pub message: Message,
}

/// Request to send or create a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    /// Raw RFC822 message (base64url encoded)
    pub raw: String,

    /// Thread ID (for replies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

/// Request to create a draft
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDraftRequest {
    /// The message
    pub message: SendMessageRequest,
}

/// Attachment data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentData {
    /// Size in bytes
    pub size: i64,

    /// Base64url-encoded data
    pub data: String,
}

/// Extracted email content
#[derive(Debug, Clone, Default)]
pub struct EmailContent {
    /// Plain text content
    pub text: String,

    /// HTML content
    pub html: String,
}

/// Email attachment info
#[derive(Debug, Clone)]
pub struct EmailAttachment {
    /// Attachment ID
    pub id: String,

    /// Filename
    pub filename: String,

    /// MIME type
    pub mime_type: String,

    /// Size in bytes
    pub size: i64,
}

/// Visibility options for labels in message list
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)] // Reserved for future use
pub enum MessageListVisibility {
    #[default]
    Show,
    Hide,
}

/// Visibility options for labels in label list
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Reserved for future use
pub enum LabelListVisibility {
    #[default]
    #[serde(rename = "labelShow")]
    Show,
    #[serde(rename = "labelShowIfUnread")]
    ShowIfUnread,
    #[serde(rename = "labelHide")]
    Hide,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_deserialize() {
        let json = r#"{"id":"123","threadId":"456","labelIds":["INBOX"]}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.id, "123");
        assert_eq!(msg.thread_id, Some("456".to_string()));
    }

    #[test]
    fn test_label_deserialize() {
        let json = r#"{"id":"Label_1","name":"Test","type":"user"}"#;
        let label: Label = serde_json::from_str(json).unwrap();
        assert_eq!(label.id, "Label_1");
        assert_eq!(label.name, "Test");
        assert_eq!(label.label_type, Some("user".to_string()));
    }

    #[test]
    fn test_filter_serialize() {
        let filter = Filter {
            id: None,
            criteria: FilterCriteria {
                from: Some("test@example.com".to_string()),
                ..Default::default()
            },
            action: FilterAction {
                add_label_ids: Some(vec!["INBOX".to_string()]),
                ..Default::default()
            },
        };
        let json = serde_json::to_string(&filter).unwrap();
        assert!(json.contains("test@example.com"));
    }
}

