//! Gmail API client
//!
//! High-level client for Gmail API operations.

use crate::config::gmail::{API_BASE_URL, USER_ID};
use crate::error::{GmailApiError, GmailMcpError, Result};
use crate::gmail::auth::Authenticator;
use crate::gmail::filters::{FilterListResult, FilterManager};
use crate::gmail::labels::{LabelListResult, LabelManager};
use crate::gmail::types::*;
use crate::gmail::utils::{
    create_email_message, encode_raw_message, extract_attachments, extract_email_content,
    find_header, EmailParams,
};

use std::sync::Arc;

/// Gmail API client
pub struct GmailClient {
    /// HTTP client
    http_client: reqwest::Client,

    /// OAuth authenticator
    authenticator: Arc<Authenticator>,
}

impl GmailClient {
    /// Create a new Gmail client
    pub fn new(authenticator: Arc<Authenticator>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            authenticator,
        }
    }

    /// Get a valid access token
    async fn access_token(&self) -> Result<String> {
        self.authenticator.get_access_token().await
    }

    /// Base URL for messages
    fn messages_url() -> String {
        format!("{}/users/{}/messages", API_BASE_URL, USER_ID)
    }

    /// Base URL for drafts
    fn drafts_url() -> String {
        format!("{}/users/{}/drafts", API_BASE_URL, USER_ID)
    }

    // ==================== Message Operations ====================

    /// Send an email
    pub async fn send_email(&self, params: EmailParams) -> Result<Message> {
        let token = self.access_token().await?;

        // For now, we only support simple emails without attachments
        // Attachment support would require multipart MIME handling
        let raw_message = create_email_message(&params)?;
        let encoded = encode_raw_message(&raw_message);

        let request = SendMessageRequest {
            raw: encoded,
            thread_id: params.thread_id,
        };

        let url = format!("{}/send", Self::messages_url());

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to send email ({}): {}", status, text),
            }))
        }
    }

    /// Create a draft
    pub async fn create_draft(&self, params: EmailParams) -> Result<Draft> {
        let token = self.access_token().await?;

        let raw_message = create_email_message(&params)?;
        let encoded = encode_raw_message(&raw_message);

        let request = CreateDraftRequest {
            message: SendMessageRequest {
                raw: encoded,
                thread_id: params.thread_id,
            },
        };

        let response = self
            .http_client
            .post(Self::drafts_url())
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to create draft ({}): {}", status, text),
            }))
        }
    }

    /// Get a message by ID
    pub async fn get_message(&self, message_id: &str) -> Result<Message> {
        let token = self.access_token().await?;
        let url = format!("{}/{}?format=full", Self::messages_url(), message_id);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else if response.status().as_u16() == 404 {
            Err(GmailMcpError::Gmail(GmailApiError::MessageNotFound {
                message_id: message_id.to_string(),
            }))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to get message ({}): {}", status, text),
            }))
        }
    }

    /// Get a message with parsed content
    pub async fn read_message(&self, message_id: &str) -> Result<ReadMessageResult> {
        let message = self.get_message(message_id).await?;

        let payload = message.payload.as_ref();
        let snippet = message.snippet.clone();

        let subject = payload
            .and_then(|p| find_header(p, "subject"))
            .unwrap_or("")
            .to_string();

        let from = payload
            .and_then(|p| find_header(p, "from"))
            .unwrap_or("")
            .to_string();

        let to = payload
            .and_then(|p| find_header(p, "to"))
            .unwrap_or("")
            .to_string();

        let date = payload
            .and_then(|p| find_header(p, "date"))
            .unwrap_or("")
            .to_string();

        let content = payload
            .map(extract_email_content)
            .unwrap_or_default();

        let attachments = payload
            .map(extract_attachments)
            .unwrap_or_default();

        // Check if body extraction failed (for logging)
        let extraction_failed = content.text.is_empty() && content.html.is_empty();
        
        // Determine body content with fallback to snippet
        let is_html_only = content.text.is_empty() && !content.html.is_empty();
        let (body, html_body) = if !content.text.is_empty() {
            let html = if content.html.is_empty() { None } else { Some(content.html) };
            (content.text, html)
        } else if !content.html.is_empty() {
            (content.html.clone(), Some(content.html))
        } else {
            // Fallback to snippet if body extraction failed
            (snippet.unwrap_or_default(), None)
        };

        // Log if we had to fall back to snippet
        if extraction_failed {
            tracing::debug!(
                "Email {} body extraction returned empty, using snippet fallback",
                message_id
            );
        }

        Ok(ReadMessageResult {
            id: message.id,
            thread_id: message.thread_id.unwrap_or_default(),
            subject,
            from,
            to,
            date,
            body,
            html_body,
            is_html_only,
            attachments,
        })
    }

    /// Search for messages
    pub async fn search_messages(
        &self,
        query: &str,
        max_results: Option<u32>,
    ) -> Result<Vec<SearchMessageResult>> {
        let token = self.access_token().await?;
        let max = max_results.unwrap_or(10);

        let url = format!("{}?q={}&maxResults={}", Self::messages_url(), urlencoding::encode(query), max);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to search messages ({}): {}", status, text),
            }));
        }

        let message_list: MessageList = response.json().await?;

        // Fetch metadata for each message
        let mut results = Vec::new();
        for msg_ref in message_list.messages {
            let url = format!(
                "{}/{}?format=metadata&metadataHeaders=Subject&metadataHeaders=From&metadataHeaders=Date",
                Self::messages_url(),
                msg_ref.id
            );

            let response = self
                .http_client
                .get(&url)
                .bearer_auth(&token)
                .send()
                .await?;

            if response.status().is_success() {
                let message: Message = response.json().await?;
                let payload = message.payload.as_ref();

                results.push(SearchMessageResult {
                    id: message.id,
                    thread_id: msg_ref.thread_id,
                    subject: payload
                        .and_then(|p| find_header(p, "subject"))
                        .unwrap_or("")
                        .to_string(),
                    from: payload
                        .and_then(|p| find_header(p, "from"))
                        .unwrap_or("")
                        .to_string(),
                    date: payload
                        .and_then(|p| find_header(p, "date"))
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }

        Ok(results)
    }

    /// Modify message labels
    pub async fn modify_message(
        &self,
        message_id: &str,
        add_label_ids: Option<Vec<String>>,
        remove_label_ids: Option<Vec<String>>,
    ) -> Result<Message> {
        let token = self.access_token().await?;
        let url = format!("{}/{}/modify", Self::messages_url(), message_id);

        let request = ModifyMessageRequest {
            add_label_ids,
            remove_label_ids,
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else if response.status().as_u16() == 404 {
            Err(GmailMcpError::Gmail(GmailApiError::MessageNotFound {
                message_id: message_id.to_string(),
            }))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to modify message ({}): {}", status, text),
            }))
        }
    }

    /// Delete a message by moving it to trash
    /// 
    /// Note: This moves the message to trash rather than permanently deleting it.
    /// The gmail.modify scope doesn't allow permanent deletion, so we use the
    /// safer trash approach which works with standard OAuth scopes.
    pub async fn delete_message(&self, message_id: &str) -> Result<()> {
        // Use Gmail's trash endpoint which works with gmail.modify scope
        let token = self.access_token().await?;
        let url = format!("{}/{}/trash", Self::messages_url(), message_id);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Length", "0")
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else if response.status().as_u16() == 404 {
            Err(GmailMcpError::Gmail(GmailApiError::MessageNotFound {
                message_id: message_id.to_string(),
            }))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to trash message ({}): {}", status, text),
            }))
        }
    }

    /// Download an attachment
    pub async fn get_attachment(
        &self,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<AttachmentData> {
        let token = self.access_token().await?;
        let url = format!(
            "{}/{}/attachments/{}",
            Self::messages_url(),
            message_id,
            attachment_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else if response.status().as_u16() == 404 {
            Err(GmailMcpError::Gmail(GmailApiError::AttachmentNotFound {
                attachment_id: attachment_id.to_string(),
            }))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to get attachment ({}): {}", status, text),
            }))
        }
    }

    // ==================== Batch Operations ====================

    /// Batch modify messages
    pub async fn batch_modify_messages(
        &self,
        message_ids: &[String],
        add_label_ids: Option<Vec<String>>,
        remove_label_ids: Option<Vec<String>>,
        batch_size: usize,
    ) -> Result<BatchOperationResult> {
        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for chunk in message_ids.chunks(batch_size) {
            for message_id in chunk {
                match self
                    .modify_message(message_id, add_label_ids.clone(), remove_label_ids.clone())
                    .await
                {
                    Ok(_) => successes.push(message_id.clone()),
                    Err(e) => failures.push((message_id.clone(), e.to_string())),
                }
            }
        }

        Ok(BatchOperationResult {
            success_count: successes.len(),
            failure_count: failures.len(),
            failures,
        })
    }

    /// Batch delete messages
    pub async fn batch_delete_messages(
        &self,
        message_ids: &[String],
        batch_size: usize,
    ) -> Result<BatchOperationResult> {
        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for chunk in message_ids.chunks(batch_size) {
            for message_id in chunk {
                match self.delete_message(message_id).await {
                    Ok(_) => successes.push(message_id.clone()),
                    Err(e) => failures.push((message_id.clone(), e.to_string())),
                }
            }
        }

        Ok(BatchOperationResult {
            success_count: successes.len(),
            failure_count: failures.len(),
            failures,
        })
    }

    // ==================== Label Operations ====================

    /// List all labels
    pub async fn list_labels(&self) -> Result<LabelListResult> {
        let token = self.access_token().await?;
        let manager = LabelManager::new(&self.http_client, &token);
        manager.list().await
    }

    /// Create a label
    pub async fn create_label(
        &self,
        name: &str,
        message_list_visibility: Option<&str>,
        label_list_visibility: Option<&str>,
    ) -> Result<Label> {
        let token = self.access_token().await?;
        let manager = LabelManager::new(&self.http_client, &token);
        manager
            .create(name, message_list_visibility, label_list_visibility)
            .await
    }

    /// Update a label
    pub async fn update_label(&self, label_id: &str, updates: UpdateLabelRequest) -> Result<Label> {
        let token = self.access_token().await?;
        let manager = LabelManager::new(&self.http_client, &token);
        manager.update(label_id, updates).await
    }

    /// Delete a label
    pub async fn delete_label(&self, label_id: &str) -> Result<()> {
        let token = self.access_token().await?;
        let manager = LabelManager::new(&self.http_client, &token);
        manager.delete(label_id).await
    }

    /// Get or create a label
    pub async fn get_or_create_label(
        &self,
        name: &str,
        message_list_visibility: Option<&str>,
        label_list_visibility: Option<&str>,
    ) -> Result<Label> {
        let token = self.access_token().await?;
        let manager = LabelManager::new(&self.http_client, &token);
        manager
            .get_or_create(name, message_list_visibility, label_list_visibility)
            .await
    }

    // ==================== Filter Operations ====================

    /// List all filters
    pub async fn list_filters(&self) -> Result<FilterListResult> {
        let token = self.access_token().await?;
        let manager = FilterManager::new(&self.http_client, &token);
        manager.list().await
    }

    /// Get a specific filter
    pub async fn get_filter(&self, filter_id: &str) -> Result<Filter> {
        let token = self.access_token().await?;
        let manager = FilterManager::new(&self.http_client, &token);
        manager.get(filter_id).await
    }

    /// Create a filter
    pub async fn create_filter(
        &self,
        criteria: FilterCriteria,
        action: FilterAction,
    ) -> Result<Filter> {
        let token = self.access_token().await?;
        let manager = FilterManager::new(&self.http_client, &token);
        manager.create(criteria, action).await
    }

    /// Delete a filter
    pub async fn delete_filter(&self, filter_id: &str) -> Result<()> {
        let token = self.access_token().await?;
        let manager = FilterManager::new(&self.http_client, &token);
        manager.delete(filter_id).await
    }
}

/// Result of reading a message
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used for API completeness
pub struct ReadMessageResult {
    pub id: String,
    pub thread_id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub date: String,
    pub body: String,
    pub html_body: Option<String>,
    pub is_html_only: bool,
    pub attachments: Vec<EmailAttachment>,
}

/// Result of searching messages
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used for API completeness
pub struct SearchMessageResult {
    pub id: String,
    pub thread_id: String,
    pub subject: String,
    pub from: String,
    pub date: String,
}

/// Result of a batch operation
#[derive(Debug, Clone)]
pub struct BatchOperationResult {
    pub success_count: usize,
    pub failure_count: usize,
    pub failures: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    // Integration tests would go here
}

