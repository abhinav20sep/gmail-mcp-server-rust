//! MCP Tool definitions and handlers
//!
//! Defines all available tools and their implementations.

use std::sync::Arc;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::gmail::client::GmailClient;
use crate::gmail::filters::FilterTemplates;
use crate::gmail::types::{FilterAction, FilterCriteria, SizeComparison, UpdateLabelRequest};
use crate::gmail::utils::{decode_base64url, format_size, EmailParams, MimeType};
use crate::mcp::types::{CallToolResult, Tool};

/// Tool handler
pub struct ToolHandler {
    gmail_client: Arc<GmailClient>,
}

impl ToolHandler {
    /// Create a new tool handler
    pub fn new(gmail_client: Arc<GmailClient>) -> Self {
        Self { gmail_client }
    }

    /// List all available tools
    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            tool_def("send_email", "Sends a new email", send_email_schema()),
            tool_def("draft_email", "Create a new email draft", send_email_schema()),
            tool_def("read_email", "Retrieves the content of a specific email", read_email_schema()),
            tool_def("search_emails", "Searches for emails using Gmail search syntax", search_emails_schema()),
            tool_def("modify_email", "Modifies email labels (move to different folders)", modify_email_schema()),
            tool_def("delete_email", "Permanently deletes an email", delete_email_schema()),
            tool_def("list_email_labels", "Retrieves all available Gmail labels", json!({"type": "object", "properties": {}})),
            tool_def("batch_modify_emails", "Modifies labels for multiple emails in batches", batch_modify_emails_schema()),
            tool_def("batch_delete_emails", "Permanently deletes multiple emails in batches", batch_delete_emails_schema()),
            tool_def("create_label", "Creates a new Gmail label", create_label_schema()),
            tool_def("update_label", "Updates an existing Gmail label", update_label_schema()),
            tool_def("delete_label", "Deletes a Gmail label", delete_label_schema()),
            tool_def("get_or_create_label", "Gets an existing label by name or creates it if it doesn't exist", get_or_create_label_schema()),
            tool_def("create_filter", "Creates a new Gmail filter with custom criteria and actions", create_filter_schema()),
            tool_def("list_filters", "Retrieves all Gmail filters", json!({"type": "object", "properties": {}})),
            tool_def("get_filter", "Gets details of a specific Gmail filter", get_filter_schema()),
            tool_def("delete_filter", "Deletes a Gmail filter", delete_filter_schema()),
            tool_def("create_filter_from_template", "Creates a filter using a pre-defined template for common scenarios", create_filter_from_template_schema()),
            tool_def("download_attachment", "Downloads an email attachment to a specified location", download_attachment_schema()),
        ]
    }

    /// Call a tool by name
    pub async fn call_tool(&self, name: &str, args: Value) -> CallToolResult {
        match name {
            "send_email" => self.handle_send_email(args, false).await,
            "draft_email" => self.handle_send_email(args, true).await,
            "read_email" => self.handle_read_email(args).await,
            "search_emails" => self.handle_search_emails(args).await,
            "modify_email" => self.handle_modify_email(args).await,
            "delete_email" => self.handle_delete_email(args).await,
            "list_email_labels" => self.handle_list_labels().await,
            "batch_modify_emails" => self.handle_batch_modify(args).await,
            "batch_delete_emails" => self.handle_batch_delete(args).await,
            "create_label" => self.handle_create_label(args).await,
            "update_label" => self.handle_update_label(args).await,
            "delete_label" => self.handle_delete_label(args).await,
            "get_or_create_label" => self.handle_get_or_create_label(args).await,
            "create_filter" => self.handle_create_filter(args).await,
            "list_filters" => self.handle_list_filters().await,
            "get_filter" => self.handle_get_filter(args).await,
            "delete_filter" => self.handle_delete_filter(args).await,
            "create_filter_from_template" => self.handle_create_filter_template(args).await,
            "download_attachment" => self.handle_download_attachment(args).await,
            _ => CallToolResult::error(format!("Unknown tool: {}", name)),
        }
    }

    // ==================== Tool Handlers ====================

    async fn handle_send_email(&self, args: Value, draft: bool) -> CallToolResult {
        use crate::gmail::utils::load_attachment;

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            to: Vec<String>,
            subject: String,
            body: String,
            html_body: Option<String>,
            mime_type: Option<String>,
            cc: Option<Vec<String>>,
            bcc: Option<Vec<String>>,
            thread_id: Option<String>,
            in_reply_to: Option<String>,
            attachments: Option<Vec<String>>,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        let mime_type = match args.mime_type.as_deref() {
            Some("text/html") => Some(MimeType::TextHtml),
            Some("multipart/alternative") => Some(MimeType::MultipartAlternative),
            _ => None,
        };

        // Load attachments from file paths
        let attachments = match args.attachments {
            Some(paths) if !paths.is_empty() => {
                let mut loaded = Vec::new();
                for path in paths {
                    match load_attachment(&path) {
                        Ok(attachment) => loaded.push(attachment),
                        Err(e) => {
                            return CallToolResult::error(format!(
                                "Failed to load attachment '{}': {}",
                                path, e
                            ))
                        }
                    }
                }
                Some(loaded)
            }
            _ => None,
        };

        let params = EmailParams {
            to: args.to,
            subject: args.subject,
            body: args.body,
            html_body: args.html_body,
            mime_type,
            cc: args.cc,
            bcc: args.bcc,
            thread_id: args.thread_id,
            in_reply_to: args.in_reply_to,
            attachments,
        };

        if draft {
            match self.gmail_client.create_draft(params).await {
                Ok(d) => CallToolResult::text(format!("Email draft created successfully with ID: {}", d.id)),
                Err(e) => CallToolResult::error(e.to_string()),
            }
        } else {
            match self.gmail_client.send_email(params).await {
                Ok(m) => CallToolResult::text(format!("Email sent successfully with ID: {}", m.id)),
                Err(e) => CallToolResult::error(e.to_string()),
            }
        }
    }

    async fn handle_read_email(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            message_id: String,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        match self.gmail_client.read_message(&args.message_id).await {
            Ok(result) => {
                let mut text = format!(
                    "Thread ID: {}\nSubject: {}\nFrom: {}\nTo: {}\nDate: {}\n\n",
                    result.thread_id, result.subject, result.from, result.to, result.date
                );

                if result.is_html_only {
                    text.push_str("[Note: This email is HTML-formatted. Plain text version not available.]\n\n");
                }

                text.push_str(&result.body);

                if !result.attachments.is_empty() {
                    text.push_str(&format!("\n\nAttachments ({}):\n", result.attachments.len()));
                    for a in &result.attachments {
                        text.push_str(&format!(
                            "- {} ({}, {}, ID: {})\n",
                            a.filename,
                            a.mime_type,
                            format_size(a.size),
                            a.id
                        ));
                    }
                }

                CallToolResult::text(text)
            }
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_search_emails(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            query: String,
            max_results: Option<u32>,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        match self.gmail_client.search_messages(&args.query, args.max_results).await {
            Ok(results) => {
                let text = results
                    .iter()
                    .map(|r| {
                        format!(
                            "ID: {}\nSubject: {}\nFrom: {}\nDate: {}\n",
                            r.id, r.subject, r.from, r.date
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                CallToolResult::text(text)
            }
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_modify_email(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            message_id: String,
            label_ids: Option<Vec<String>>,
            add_label_ids: Option<Vec<String>>,
            remove_label_ids: Option<Vec<String>>,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        let add = args.add_label_ids.or(args.label_ids);

        match self
            .gmail_client
            .modify_message(&args.message_id, add, args.remove_label_ids)
            .await
        {
            Ok(_) => CallToolResult::text(format!(
                "Email {} labels updated successfully",
                args.message_id
            )),
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_delete_email(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            message_id: String,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        match self.gmail_client.delete_message(&args.message_id).await {
            Ok(_) => CallToolResult::text(format!(
                "Email {} deleted successfully",
                args.message_id
            )),
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_list_labels(&self) -> CallToolResult {
        match self.gmail_client.list_labels().await {
            Ok(result) => {
                let mut text = format!(
                    "Found {} labels ({} system, {} user):\n\n",
                    result.count.total, result.count.system, result.count.user
                );

                text.push_str("System Labels:\n");
                for label in &result.system {
                    text.push_str(&format!("ID: {}\nName: {}\n\n", label.id, label.name));
                }

                text.push_str("\nUser Labels:\n");
                for label in &result.user {
                    text.push_str(&format!("ID: {}\nName: {}\n\n", label.id, label.name));
                }

                CallToolResult::text(text)
            }
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_batch_modify(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            message_ids: Vec<String>,
            add_label_ids: Option<Vec<String>>,
            remove_label_ids: Option<Vec<String>>,
            batch_size: Option<usize>,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        match self
            .gmail_client
            .batch_modify_messages(
                &args.message_ids,
                args.add_label_ids,
                args.remove_label_ids,
                args.batch_size.unwrap_or(50),
            )
            .await
        {
            Ok(result) => {
                let mut text = format!(
                    "Batch label modification complete.\nSuccessfully processed: {} messages\n",
                    result.success_count
                );

                if result.failure_count > 0 {
                    text.push_str(&format!(
                        "Failed to process: {} messages\n\nFailed message IDs:\n",
                        result.failure_count
                    ));
                    for (id, err) in &result.failures {
                        text.push_str(&format!("- {}... ({})\n", &id[..16.min(id.len())], err));
                    }
                }

                CallToolResult::text(text)
            }
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_batch_delete(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            message_ids: Vec<String>,
            batch_size: Option<usize>,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        match self
            .gmail_client
            .batch_delete_messages(&args.message_ids, args.batch_size.unwrap_or(50))
            .await
        {
            Ok(result) => {
                let mut text = format!(
                    "Batch delete operation complete.\nSuccessfully deleted: {} messages\n",
                    result.success_count
                );

                if result.failure_count > 0 {
                    text.push_str(&format!(
                        "Failed to delete: {} messages\n\nFailed message IDs:\n",
                        result.failure_count
                    ));
                    for (id, err) in &result.failures {
                        text.push_str(&format!("- {}... ({})\n", &id[..16.min(id.len())], err));
                    }
                }

                CallToolResult::text(text)
            }
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_create_label(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            name: String,
            message_list_visibility: Option<String>,
            label_list_visibility: Option<String>,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        match self
            .gmail_client
            .create_label(
                &args.name,
                args.message_list_visibility.as_deref(),
                args.label_list_visibility.as_deref(),
            )
            .await
        {
            Ok(label) => CallToolResult::text(format!(
                "Label created successfully:\nID: {}\nName: {}\nType: {}",
                label.id,
                label.name,
                label.label_type.unwrap_or_default()
            )),
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_update_label(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            #[serde(alias = "labelId")]
            id: String,
            name: Option<String>,
            message_list_visibility: Option<String>,
            label_list_visibility: Option<String>,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        let updates = UpdateLabelRequest {
            name: args.name,
            message_list_visibility: args.message_list_visibility,
            label_list_visibility: args.label_list_visibility,
        };

        match self.gmail_client.update_label(&args.id, updates).await {
            Ok(label) => CallToolResult::text(format!(
                "Label updated successfully:\nID: {}\nName: {}\nType: {}",
                label.id,
                label.name,
                label.label_type.unwrap_or_default()
            )),
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_delete_label(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        struct Args {
            #[serde(alias = "labelId")]
            id: String,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        match self.gmail_client.delete_label(&args.id).await {
            Ok(_) => CallToolResult::text(format!("Label {} deleted successfully", args.id)),
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_get_or_create_label(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            name: String,
            message_list_visibility: Option<String>,
            label_list_visibility: Option<String>,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        match self
            .gmail_client
            .get_or_create_label(
                &args.name,
                args.message_list_visibility.as_deref(),
                args.label_list_visibility.as_deref(),
            )
            .await
        {
            Ok(label) => CallToolResult::text(format!(
                "Label:\nID: {}\nName: {}\nType: {}",
                label.id,
                label.name,
                label.label_type.unwrap_or_default()
            )),
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_create_filter(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            criteria: CriteriaArgs,
            action: ActionArgs,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CriteriaArgs {
            from: Option<String>,
            to: Option<String>,
            subject: Option<String>,
            query: Option<String>,
            negated_query: Option<String>,
            has_attachment: Option<bool>,
            exclude_chats: Option<bool>,
            size: Option<i64>,
            size_comparison: Option<String>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ActionArgs {
            add_label_ids: Option<Vec<String>>,
            remove_label_ids: Option<Vec<String>>,
            forward: Option<String>,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        let criteria = FilterCriteria {
            from: args.criteria.from,
            to: args.criteria.to,
            subject: args.criteria.subject,
            query: args.criteria.query,
            negated_query: args.criteria.negated_query,
            has_attachment: args.criteria.has_attachment,
            exclude_chats: args.criteria.exclude_chats,
            size: args.criteria.size,
            size_comparison: args.criteria.size_comparison.map(|s| match s.as_str() {
                "smaller" => SizeComparison::Smaller,
                "larger" => SizeComparison::Larger,
                _ => SizeComparison::Unspecified,
            }),
        };

        let action = FilterAction {
            add_label_ids: args.action.add_label_ids,
            remove_label_ids: args.action.remove_label_ids,
            forward: args.action.forward,
        };

        match self.gmail_client.create_filter(criteria, action).await {
            Ok(filter) => CallToolResult::text(format!(
                "Filter created successfully:\nID: {}",
                filter.id.unwrap_or_default()
            )),
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_list_filters(&self) -> CallToolResult {
        match self.gmail_client.list_filters().await {
            Ok(result) => {
                if result.filters.is_empty() {
                    return CallToolResult::text("No filters found.");
                }

                let mut text = format!("Found {} filters:\n\n", result.count);

                for filter in &result.filters {
                    text.push_str(&format!("ID: {}\n", filter.id.as_deref().unwrap_or("")));

                    // Format criteria
                    let criteria_parts: Vec<String> = [
                        filter.criteria.from.as_ref().map(|v| format!("from: {}", v)),
                        filter.criteria.to.as_ref().map(|v| format!("to: {}", v)),
                        filter.criteria.subject.as_ref().map(|v| format!("subject: {}", v)),
                        filter.criteria.query.as_ref().map(|v| format!("query: {}", v)),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();

                    text.push_str(&format!("Criteria: {}\n", criteria_parts.join(", ")));

                    // Format actions
                    let action_parts: Vec<String> = [
                        filter.action.add_label_ids.as_ref().map(|v| format!("addLabelIds: {}", v.join(", "))),
                        filter.action.remove_label_ids.as_ref().map(|v| format!("removeLabelIds: {}", v.join(", "))),
                        filter.action.forward.as_ref().map(|v| format!("forward: {}", v)),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();

                    text.push_str(&format!("Actions: {}\n\n", action_parts.join(", ")));
                }

                CallToolResult::text(text)
            }
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_get_filter(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            filter_id: String,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        match self.gmail_client.get_filter(&args.filter_id).await {
            Ok(filter) => {
                let mut text = format!("Filter details:\nID: {}\n", filter.id.as_deref().unwrap_or(""));

                let criteria_parts: Vec<String> = [
                    filter.criteria.from.as_ref().map(|v| format!("from: {}", v)),
                    filter.criteria.to.as_ref().map(|v| format!("to: {}", v)),
                    filter.criteria.subject.as_ref().map(|v| format!("subject: {}", v)),
                ]
                .into_iter()
                .flatten()
                .collect();

                text.push_str(&format!("Criteria: {}\n", criteria_parts.join(", ")));

                CallToolResult::text(text)
            }
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_delete_filter(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            filter_id: String,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        match self.gmail_client.delete_filter(&args.filter_id).await {
            Ok(_) => CallToolResult::text(format!("Filter {} deleted successfully", args.filter_id)),
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_create_filter_template(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize, Default)]
        #[serde(rename_all = "camelCase")]
        struct TemplateParams {
            sender_email: Option<String>,
            subject_text: Option<String>,
            search_text: Option<String>,
            list_identifier: Option<String>,
            size_in_bytes: Option<i64>,
            label_ids: Option<Vec<String>>,
            archive: Option<bool>,
            mark_as_read: Option<bool>,
            mark_important: Option<bool>,
        }

        // Accept both nested `parameters` object and flat parameters for better UX
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            #[serde(alias = "templateName")]
            template: String,
            // Nested parameters object (preferred)
            #[serde(default)]
            parameters: Option<TemplateParams>,
            // Flat parameters (for convenience)
            sender_email: Option<String>,
            subject_text: Option<String>,
            search_text: Option<String>,
            list_identifier: Option<String>,
            size_in_bytes: Option<i64>,
            label_ids: Option<Vec<String>>,
            #[serde(alias = "labelId")]
            label_id: Option<String>,
            archive: Option<bool>,
            mark_as_read: Option<bool>,
            mark_important: Option<bool>,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        // Merge flat params with nested params (flat params take precedence)
        let nested = args.parameters.unwrap_or_default();
        let params = TemplateParams {
            sender_email: args.sender_email.or(nested.sender_email),
            subject_text: args.subject_text.or(nested.subject_text),
            search_text: args.search_text.or(nested.search_text),
            list_identifier: args.list_identifier.or(nested.list_identifier),
            size_in_bytes: args.size_in_bytes.or(nested.size_in_bytes),
            // Handle both labelIds array and single labelId
            label_ids: args.label_ids.or(nested.label_ids).or_else(|| {
                args.label_id.map(|id| vec![id])
            }),
            archive: args.archive.or(nested.archive),
            mark_as_read: args.mark_as_read.or(nested.mark_as_read),
            mark_important: args.mark_important.or(nested.mark_important),
        };

        let (criteria, action) = match args.template.as_str() {
            "fromSender" => {
                let email = match params.sender_email {
                    Some(e) => e,
                    None => return CallToolResult::error("senderEmail is required for fromSender template"),
                };
                FilterTemplates::from_sender(&email, params.label_ids, params.archive.unwrap_or(false))
            }
            "withSubject" => {
                let subject = match params.subject_text {
                    Some(s) => s,
                    None => return CallToolResult::error("subjectText is required for withSubject template"),
                };
                FilterTemplates::with_subject(&subject, params.label_ids, params.mark_as_read.unwrap_or(false))
            }
            "withAttachments" => {
                FilterTemplates::with_attachments(params.label_ids)
            }
            "largeEmails" => {
                let size = match params.size_in_bytes {
                    Some(s) => s,
                    None => return CallToolResult::error("sizeInBytes is required for largeEmails template"),
                };
                FilterTemplates::large_emails(size, params.label_ids)
            }
            "containingText" => {
                let text = match params.search_text {
                    Some(t) => t,
                    None => return CallToolResult::error("searchText is required for containingText template"),
                };
                FilterTemplates::containing_text(&text, params.label_ids, params.mark_important.unwrap_or(false))
            }
            "mailingList" => {
                let list = match params.list_identifier {
                    Some(l) => l,
                    None => return CallToolResult::error("listIdentifier is required for mailingList template"),
                };
                FilterTemplates::mailing_list(&list, params.label_ids, params.archive.unwrap_or(true))
            }
            _ => return CallToolResult::error(format!("Unknown template: {}", args.template)),
        };

        match self.gmail_client.create_filter(criteria, action).await {
            Ok(filter) => CallToolResult::text(format!(
                "Filter created from template '{}':\nID: {}",
                args.template,
                filter.id.unwrap_or_default()
            )),
            Err(e) => CallToolResult::error(e.to_string()),
        }
    }

    async fn handle_download_attachment(&self, args: Value) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            message_id: String,
            attachment_id: String,
            filename: Option<String>,
            save_path: Option<String>,
        }

        let args: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(format!("Invalid arguments: {}", e)),
        };

        // Get attachment data
        let attachment = match self
            .gmail_client
            .get_attachment(&args.message_id, &args.attachment_id)
            .await
        {
            Ok(a) => a,
            Err(e) => return CallToolResult::error(e.to_string()),
        };

        // Decode the data
        let data = match decode_base64url(&attachment.data) {
            Ok(d) => d,
            Err(e) => return CallToolResult::error(format!("Failed to decode attachment: {}", e)),
        };

        // Determine filename
        let filename = args.filename.unwrap_or_else(|| format!("attachment-{}", args.attachment_id));

        // Determine save path
        let save_dir = args.save_path.unwrap_or_else(|| ".".to_string());
        let full_path = std::path::Path::new(&save_dir).join(&filename);

        // Ensure directory exists
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return CallToolResult::error(format!("Failed to create directory: {}", e));
                }
            }
        }

        // Write file
        if let Err(e) = std::fs::write(&full_path, &data) {
            return CallToolResult::error(format!("Failed to write file: {}", e));
        }

        CallToolResult::text(format!(
            "Attachment downloaded successfully:\nFile: {}\nSize: {} bytes\nSaved to: {}",
            filename,
            data.len(),
            full_path.display()
        ))
    }
}

// ==================== Schema Definitions ====================

fn tool_def(name: &str, description: &str, input_schema: Value) -> Tool {
    Tool {
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema,
    }
}

fn send_email_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "to": {
                "type": "array",
                "items": {"type": "string"},
                "description": "List of recipient email addresses"
            },
            "subject": {
                "type": "string",
                "description": "Email subject"
            },
            "body": {
                "type": "string",
                "description": "Email body content"
            },
            "htmlBody": {
                "type": "string",
                "description": "HTML version of the email body"
            },
            "mimeType": {
                "type": "string",
                "enum": ["text/plain", "text/html", "multipart/alternative"],
                "description": "Email content type"
            },
            "cc": {
                "type": "array",
                "items": {"type": "string"},
                "description": "List of CC recipients"
            },
            "bcc": {
                "type": "array",
                "items": {"type": "string"},
                "description": "List of BCC recipients"
            },
            "threadId": {
                "type": "string",
                "description": "Thread ID to reply to"
            },
            "inReplyTo": {
                "type": "string",
                "description": "Message ID being replied to"
            }
        },
        "required": ["to", "subject", "body"]
    })
}

fn read_email_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "messageId": {
                "type": "string",
                "description": "ID of the email message to retrieve"
            }
        },
        "required": ["messageId"]
    })
}

fn search_emails_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Gmail search query"
            },
            "maxResults": {
                "type": "number",
                "description": "Maximum number of results"
            }
        },
        "required": ["query"]
    })
}

fn modify_email_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "messageId": {
                "type": "string",
                "description": "ID of the email message to modify"
            },
            "labelIds": {
                "type": "array",
                "items": {"type": "string"},
                "description": "List of label IDs to apply"
            },
            "addLabelIds": {
                "type": "array",
                "items": {"type": "string"},
                "description": "List of label IDs to add"
            },
            "removeLabelIds": {
                "type": "array",
                "items": {"type": "string"},
                "description": "List of label IDs to remove"
            }
        },
        "required": ["messageId"]
    })
}

fn delete_email_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "messageId": {
                "type": "string",
                "description": "ID of the email message to delete"
            }
        },
        "required": ["messageId"]
    })
}

fn batch_modify_emails_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "messageIds": {
                "type": "array",
                "items": {"type": "string"},
                "description": "List of message IDs to modify"
            },
            "addLabelIds": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Label IDs to add"
            },
            "removeLabelIds": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Label IDs to remove"
            },
            "batchSize": {
                "type": "number",
                "description": "Batch size (default: 50)"
            }
        },
        "required": ["messageIds"]
    })
}

fn batch_delete_emails_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "messageIds": {
                "type": "array",
                "items": {"type": "string"},
                "description": "List of message IDs to delete"
            },
            "batchSize": {
                "type": "number",
                "description": "Batch size (default: 50)"
            }
        },
        "required": ["messageIds"]
    })
}

fn create_label_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Name for the new label"
            },
            "messageListVisibility": {
                "type": "string",
                "enum": ["show", "hide"],
                "description": "Message list visibility"
            },
            "labelListVisibility": {
                "type": "string",
                "enum": ["labelShow", "labelShowIfUnread", "labelHide"],
                "description": "Label list visibility"
            }
        },
        "required": ["name"]
    })
}

fn update_label_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "string",
                "description": "ID of the label to update"
            },
            "name": {
                "type": "string",
                "description": "New name for the label"
            },
            "messageListVisibility": {
                "type": "string",
                "enum": ["show", "hide"]
            },
            "labelListVisibility": {
                "type": "string",
                "enum": ["labelShow", "labelShowIfUnread", "labelHide"]
            }
        },
        "required": ["id"]
    })
}

fn delete_label_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "string",
                "description": "ID of the label to delete"
            }
        },
        "required": ["id"]
    })
}

fn get_or_create_label_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Name of the label"
            },
            "messageListVisibility": {
                "type": "string",
                "enum": ["show", "hide"]
            },
            "labelListVisibility": {
                "type": "string",
                "enum": ["labelShow", "labelShowIfUnread", "labelHide"]
            }
        },
        "required": ["name"]
    })
}

fn create_filter_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "criteria": {
                "type": "object",
                "properties": {
                    "from": {"type": "string"},
                    "to": {"type": "string"},
                    "subject": {"type": "string"},
                    "query": {"type": "string"},
                    "negatedQuery": {"type": "string"},
                    "hasAttachment": {"type": "boolean"},
                    "excludeChats": {"type": "boolean"},
                    "size": {"type": "number"},
                    "sizeComparison": {"type": "string", "enum": ["unspecified", "smaller", "larger"]}
                }
            },
            "action": {
                "type": "object",
                "properties": {
                    "addLabelIds": {"type": "array", "items": {"type": "string"}},
                    "removeLabelIds": {"type": "array", "items": {"type": "string"}},
                    "forward": {"type": "string"}
                }
            }
        },
        "required": ["criteria", "action"]
    })
}

fn get_filter_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "filterId": {
                "type": "string",
                "description": "ID of the filter"
            }
        },
        "required": ["filterId"]
    })
}

fn delete_filter_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "filterId": {
                "type": "string",
                "description": "ID of the filter to delete"
            }
        },
        "required": ["filterId"]
    })
}

fn create_filter_from_template_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "template": {
                "type": "string",
                "enum": ["fromSender", "withSubject", "withAttachments", "largeEmails", "containingText", "mailingList"],
                "description": "Pre-defined filter template"
            },
            "parameters": {
                "type": "object",
                "description": "Nested parameters object (optional - can also use flat parameters)",
                "properties": {
                    "senderEmail": {"type": "string"},
                    "subjectText": {"type": "string"},
                    "searchText": {"type": "string"},
                    "listIdentifier": {"type": "string"},
                    "sizeInBytes": {"type": "number"},
                    "labelIds": {"type": "array", "items": {"type": "string"}},
                    "archive": {"type": "boolean"},
                    "markAsRead": {"type": "boolean"},
                    "markImportant": {"type": "boolean"}
                }
            },
            "senderEmail": {"type": "string", "description": "Email address for fromSender template"},
            "subjectText": {"type": "string", "description": "Subject text for withSubject template"},
            "searchText": {"type": "string", "description": "Search text for containingText template"},
            "listIdentifier": {"type": "string", "description": "List ID for mailingList template"},
            "sizeInBytes": {"type": "number", "description": "Size threshold for largeEmails template"},
            "labelIds": {"type": "array", "items": {"type": "string"}, "description": "Labels to apply"},
            "labelId": {"type": "string", "description": "Single label to apply (alternative to labelIds)"},
            "archive": {"type": "boolean", "description": "Whether to archive matching emails"},
            "markAsRead": {"type": "boolean", "description": "Whether to mark matching emails as read"},
            "markImportant": {"type": "boolean", "description": "Whether to mark matching emails as important"}
        },
        "required": ["template"]
    })
}

fn download_attachment_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "messageId": {
                "type": "string",
                "description": "ID of the email containing the attachment"
            },
            "attachmentId": {
                "type": "string",
                "description": "ID of the attachment"
            },
            "filename": {
                "type": "string",
                "description": "Filename to save as"
            },
            "savePath": {
                "type": "string",
                "description": "Directory to save to"
            }
        },
        "required": ["messageId", "attachmentId"]
    })
}

