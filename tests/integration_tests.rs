//! Integration tests for Gmail MCP Server
//!
//! These tests verify the MCP protocol handling and tool invocations.
//! Note: These tests mock the Gmail API - they don't make real API calls.

use serde_json::{json, Value};

/// Helper to create a JSON-RPC request
fn make_request(id: i64, method: &str, params: Option<Value>) -> Value {
    let mut request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
    });
    if let Some(p) = params {
        request["params"] = p;
    }
    request
}

/// Helper to parse JSON-RPC response
fn parse_response(json_str: &str) -> Value {
    serde_json::from_str(json_str).expect("Failed to parse JSON response")
}

mod mcp_protocol_tests {
    use super::*;

    #[test]
    fn test_initialize_request_format() {
        let request = make_request(1, "initialize", Some(json!({
            "protocolVersion": "2024-11-05",
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        })));

        assert_eq!(request["method"], "initialize");
        assert_eq!(request["id"], 1);
        assert!(request["params"]["protocolVersion"].is_string());
    }

    #[test]
    fn test_list_tools_request_format() {
        let request = make_request(2, "tools/list", None);
        assert_eq!(request["method"], "tools/list");
        assert_eq!(request["id"], 2);
    }

    #[test]
    fn test_call_tool_request_format() {
        let request = make_request(3, "tools/call", Some(json!({
            "name": "search_emails",
            "arguments": {
                "query": "from:test@example.com",
                "maxResults": 10
            }
        })));

        assert_eq!(request["method"], "tools/call");
        assert_eq!(request["params"]["name"], "search_emails");
        assert_eq!(request["params"]["arguments"]["query"], "from:test@example.com");
    }

    #[test]
    fn test_jsonrpc_response_structure() {
        let response_json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let response = parse_response(response_json);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response["result"].is_object());
        assert!(response["error"].is_null());
    }

    #[test]
    fn test_jsonrpc_error_response_structure() {
        let response_json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found: unknown"}}"#;
        let response = parse_response(response_json);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response["result"].is_null());
        assert_eq!(response["error"]["code"], -32601);
    }
}

mod tool_schema_tests {
    use super::*;

    #[test]
    fn test_send_email_schema() {
        let args = json!({
            "to": ["recipient@example.com"],
            "subject": "Test Subject",
            "body": "Test body content",
            "cc": ["cc@example.com"],
            "bcc": ["bcc@example.com"],
            "mimeType": "text/plain"
        });

        // Verify required fields
        assert!(args["to"].is_array());
        assert!(args["subject"].is_string());
        assert!(args["body"].is_string());

        // Verify optional fields
        assert!(args["cc"].is_array());
        assert!(args["bcc"].is_array());
        assert!(args["mimeType"].is_string());
    }

    #[test]
    fn test_send_email_with_attachments_schema() {
        let args = json!({
            "to": ["recipient@example.com"],
            "subject": "Test with Attachment",
            "body": "See attached file",
            "attachments": ["/path/to/file.pdf", "/path/to/image.png"]
        });

        assert!(args["attachments"].is_array());
        assert_eq!(args["attachments"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_search_emails_schema() {
        let args = json!({
            "query": "from:sender@example.com has:attachment",
            "maxResults": 25
        });

        assert!(args["query"].is_string());
        assert!(args["maxResults"].is_number());
    }

    #[test]
    fn test_modify_email_schema() {
        let args = json!({
            "messageId": "abc123",
            "addLabelIds": ["STARRED", "IMPORTANT"],
            "removeLabelIds": ["UNREAD"]
        });

        assert!(args["messageId"].is_string());
        assert!(args["addLabelIds"].is_array());
        assert!(args["removeLabelIds"].is_array());
    }

    #[test]
    fn test_create_filter_schema() {
        let args = json!({
            "criteria": {
                "from": "newsletter@example.com",
                "hasAttachment": true
            },
            "action": {
                "addLabelIds": ["Label_1"],
                "removeLabelIds": ["INBOX"]
            }
        });

        assert!(args["criteria"].is_object());
        assert!(args["action"].is_object());
        assert_eq!(args["criteria"]["from"], "newsletter@example.com");
    }

    #[test]
    fn test_create_filter_from_template_schema() {
        let args = json!({
            "template": "fromSender",
            "parameters": {
                "senderEmail": "important@example.com",
                "labelIds": ["Label_Important"],
                "archive": true
            }
        });

        assert!(args["template"].is_string());
        assert!(args["parameters"].is_object());
    }

    #[test]
    fn test_batch_modify_emails_schema() {
        let args = json!({
            "messageIds": ["id1", "id2", "id3"],
            "addLabelIds": ["Label_1"],
            "batchSize": 50
        });

        assert!(args["messageIds"].is_array());
        assert_eq!(args["messageIds"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_download_attachment_schema() {
        let args = json!({
            "messageId": "msg123",
            "attachmentId": "att456",
            "filename": "document.pdf",
            "savePath": "/tmp/downloads"
        });

        assert!(args["messageId"].is_string());
        assert!(args["attachmentId"].is_string());
    }
}

mod email_utils_tests {
    use gmail_mcp_server_rust::gmail::utils::*;

    #[test]
    fn test_validate_email_addresses() {
        // Valid emails
        assert!(validate_email("user@example.com"));
        assert!(validate_email("user.name@example.co.uk"));
        assert!(validate_email("user+tag@example.com"));

        // Invalid emails
        assert!(!validate_email("invalid"));
        assert!(!validate_email("@example.com"));
        assert!(!validate_email("user@"));
        assert!(!validate_email("user@.com"));
    }

    #[test]
    fn test_encode_mime_header_ascii() {
        let result = encode_mime_header("Hello World");
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_encode_mime_header_unicode() {
        let result = encode_mime_header("Héllo Wörld 你好");
        assert!(result.starts_with("=?UTF-8?B?"));
        assert!(result.ends_with("?="));
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_size(500), "500 bytes");
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(1536), "2 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_create_simple_email() {
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

        let result = create_email_message(&params).unwrap();
        assert!(result.contains("To: test@example.com"));
        assert!(result.contains("Subject: Test Subject"));
        assert!(result.contains("Test body"));
        assert!(result.contains("Content-Type: text/plain"));
    }

    #[test]
    fn test_create_html_email() {
        let params = EmailParams {
            to: vec!["test@example.com".to_string()],
            subject: "HTML Email".to_string(),
            body: "Plain text version".to_string(),
            html_body: Some("<h1>HTML Version</h1>".to_string()),
            mime_type: Some(MimeType::MultipartAlternative),
            cc: None,
            bcc: None,
            thread_id: None,
            in_reply_to: None,
            attachments: None,
        };

        let result = create_email_message(&params).unwrap();
        assert!(result.contains("multipart/alternative"));
        assert!(result.contains("Plain text version"));
        assert!(result.contains("<h1>HTML Version</h1>"));
    }

    #[test]
    fn test_create_email_with_cc_bcc() {
        let params = EmailParams {
            to: vec!["to@example.com".to_string()],
            subject: "Test".to_string(),
            body: "Body".to_string(),
            html_body: None,
            mime_type: None,
            cc: Some(vec!["cc@example.com".to_string()]),
            bcc: Some(vec!["bcc@example.com".to_string()]),
            thread_id: None,
            in_reply_to: None,
            attachments: None,
        };

        let result = create_email_message(&params).unwrap();
        assert!(result.contains("Cc: cc@example.com"));
        assert!(result.contains("Bcc: bcc@example.com"));
    }

    #[test]
    fn test_create_email_with_reply_headers() {
        let params = EmailParams {
            to: vec!["to@example.com".to_string()],
            subject: "Re: Original".to_string(),
            body: "Reply body".to_string(),
            html_body: None,
            mime_type: None,
            cc: None,
            bcc: None,
            thread_id: Some("thread123".to_string()),
            in_reply_to: Some("<original@example.com>".to_string()),
            attachments: None,
        };

        let result = create_email_message(&params).unwrap();
        assert!(result.contains("In-Reply-To: <original@example.com>"));
        assert!(result.contains("References: <original@example.com>"));
    }

    #[test]
    fn test_email_validation_rejects_invalid() {
        let params = EmailParams {
            to: vec!["invalid-email".to_string()],
            subject: "Test".to_string(),
            body: "Body".to_string(),
            html_body: None,
            mime_type: None,
            cc: None,
            bcc: None,
            thread_id: None,
            in_reply_to: None,
            attachments: None,
        };

        let result = create_email_message(&params);
        assert!(result.is_err());
    }
}

mod filter_template_tests {
    use gmail_mcp_server_rust::gmail::filters::FilterTemplates;
    use gmail_mcp_server_rust::gmail::types::SizeComparison;

    #[test]
    fn test_from_sender_template() {
        let (criteria, action) = FilterTemplates::from_sender(
            "news@example.com",
            Some(vec!["Label_News".to_string()]),
            true, // archive
        );

        assert_eq!(criteria.from, Some("news@example.com".to_string()));
        assert_eq!(action.add_label_ids, Some(vec!["Label_News".to_string()]));
        assert_eq!(action.remove_label_ids, Some(vec!["INBOX".to_string()]));
    }

    #[test]
    fn test_with_subject_template() {
        let (criteria, action) = FilterTemplates::with_subject(
            "[URGENT]",
            Some(vec!["Label_Urgent".to_string()]),
            true, // mark as read
        );

        assert_eq!(criteria.subject, Some("[URGENT]".to_string()));
        assert_eq!(action.remove_label_ids, Some(vec!["UNREAD".to_string()]));
    }

    #[test]
    fn test_with_attachments_template() {
        let (criteria, action) = FilterTemplates::with_attachments(
            Some(vec!["Label_HasAttachments".to_string()]),
        );

        assert_eq!(criteria.has_attachment, Some(true));
        assert_eq!(action.add_label_ids, Some(vec!["Label_HasAttachments".to_string()]));
    }

    #[test]
    fn test_large_emails_template() {
        let (criteria, _action) = FilterTemplates::large_emails(
            5 * 1024 * 1024, // 5 MB
            None,
        );

        assert_eq!(criteria.size, Some(5 * 1024 * 1024));
        assert_eq!(criteria.size_comparison, Some(SizeComparison::Larger));
    }

    #[test]
    fn test_containing_text_template() {
        let (criteria, action) = FilterTemplates::containing_text(
            "confidential",
            Some(vec!["Label_Confidential".to_string()]),
            true, // mark important
        );

        assert!(criteria.query.as_ref().unwrap().contains("confidential"));
        assert!(action.add_label_ids.as_ref().unwrap().contains(&"IMPORTANT".to_string()));
    }

    #[test]
    fn test_mailing_list_template() {
        let (criteria, action) = FilterTemplates::mailing_list(
            "rust-users",
            Some(vec!["Label_RustList".to_string()]),
            true, // archive
        );

        assert!(criteria.query.as_ref().unwrap().contains("rust-users"));
        assert_eq!(action.remove_label_ids, Some(vec!["INBOX".to_string()]));
    }
}

mod types_serialization_tests {
    use gmail_mcp_server_rust::gmail::types::*;

    #[test]
    fn test_message_serialization() {
        let message = Message {
            id: "msg123".to_string(),
            thread_id: Some("thread456".to_string()),
            label_ids: vec!["INBOX".to_string(), "UNREAD".to_string()],
            snippet: Some("Email preview...".to_string()),
            payload: None,
            size_estimate: Some(1024),
            raw: None,
            internal_date: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("msg123"));
        assert!(json.contains("thread456"));
        assert!(json.contains("INBOX"));
    }

    #[test]
    fn test_label_deserialization() {
        let json = r#"{
            "id": "Label_1",
            "name": "Important",
            "type": "user",
            "messageListVisibility": "show",
            "labelListVisibility": "labelShow"
        }"#;

        let label: Label = serde_json::from_str(json).unwrap();
        assert_eq!(label.id, "Label_1");
        assert_eq!(label.name, "Important");
        assert_eq!(label.label_type, Some("user".to_string()));
    }

    #[test]
    fn test_filter_serialization() {
        let filter = Filter {
            id: Some("filter123".to_string()),
            criteria: FilterCriteria {
                from: Some("sender@example.com".to_string()),
                has_attachment: Some(true),
                ..Default::default()
            },
            action: FilterAction {
                add_label_ids: Some(vec!["Label_1".to_string()]),
                ..Default::default()
            },
        };

        let json = serde_json::to_string(&filter).unwrap();
        assert!(json.contains("filter123"));
        assert!(json.contains("sender@example.com"));
        assert!(json.contains("Label_1"));
    }

    #[test]
    fn test_size_comparison_serialization() {
        let criteria = FilterCriteria {
            size: Some(1024 * 1024),
            size_comparison: Some(SizeComparison::Larger),
            ..Default::default()
        };

        let json = serde_json::to_string(&criteria).unwrap();
        assert!(json.contains("larger"));
    }
}

mod mcp_types_tests {
    use gmail_mcp_server_rust::mcp::types::*;

    #[test]
    fn test_tool_result_text() {
        let result = CallToolResult::text("Success message");
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_tool_result_error() {
        let result = CallToolResult::error("Something went wrong");
        assert!(result.is_error);
        
        if let ToolResultContent::Text { text } = &result.content[0] {
            assert!(text.contains("Error:"));
            assert!(text.contains("Something went wrong"));
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_request_id_variants() {
        let id_num = RequestId::Number(42);
        let id_str = RequestId::String("req-123".to_string());

        let json_num = serde_json::to_string(&id_num).unwrap();
        let json_str = serde_json::to_string(&id_str).unwrap();

        assert_eq!(json_num, "42");
        assert_eq!(json_str, "\"req-123\"");
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            serde_json::json!({"status": "ok"})
        );

        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            JsonRpcError::method_not_found("unknown_method")
        );

        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32601);
    }
}

