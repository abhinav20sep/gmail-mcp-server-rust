//! Filter management for Gmail
//!
//! Provides comprehensive filter management functionality.

use crate::error::{GmailApiError, GmailMcpError, Result};
use crate::gmail::types::{Filter, FilterAction, FilterCriteria, FilterList, SizeComparison};

/// Filter manager for Gmail operations
pub struct FilterManager<'a> {
    client: &'a reqwest::Client,
    access_token: &'a str,
}

impl<'a> FilterManager<'a> {
    /// Create a new filter manager
    pub fn new(client: &'a reqwest::Client, access_token: &'a str) -> Self {
        Self {
            client,
            access_token,
        }
    }

    /// Base URL for filters API
    fn base_url() -> String {
        format!(
            "{}/users/me/settings/filters",
            crate::config::gmail::API_BASE_URL
        )
    }

    /// Create a new Gmail filter
    pub async fn create(&self, criteria: FilterCriteria, action: FilterAction) -> Result<Filter> {
        let filter = Filter {
            id: None,
            criteria,
            action,
        };

        let response = self
            .client
            .post(Self::base_url())
            .bearer_auth(self.access_token)
            .json(&filter)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            if status.as_u16() == 400 {
                return Err(GmailMcpError::Gmail(GmailApiError::InvalidFilterCriteria {
                    message: text,
                }));
            }

            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to create filter ({}): {}", status, text),
            }))
        }
    }

    /// List all Gmail filters
    pub async fn list(&self) -> Result<FilterListResult> {
        let response = self
            .client
            .get(Self::base_url())
            .bearer_auth(self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            // Get response text first to handle empty responses
            let text = response.text().await.unwrap_or_default();
            
            // Handle empty response or empty object (no filters exist)
            if text.is_empty() || text.trim() == "{}" {
                return Ok(FilterListResult {
                    filters: vec![],
                    count: 0,
                });
            }
            
            // Parse the response
            let filter_list: FilterList = serde_json::from_str(&text).map_err(|e| {
                GmailMcpError::Gmail(GmailApiError::RequestFailed {
                    message: format!("Failed to parse filter list: {}", e),
                })
            })?;
            let filters = filter_list.filter;

            Ok(FilterListResult {
                filters: filters.clone(),
                count: filters.len(),
            })
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to list filters ({}): {}", status, text),
            }))
        }
    }

    /// Get a specific filter by ID
    pub async fn get(&self, filter_id: &str) -> Result<Filter> {
        let url = format!("{}/{}", Self::base_url(), filter_id);

        let response = self
            .client
            .get(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else if response.status().as_u16() == 404 {
            Err(GmailMcpError::Gmail(GmailApiError::FilterNotFound {
                filter_id: filter_id.to_string(),
            }))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to get filter ({}): {}", status, text),
            }))
        }
    }

    /// Delete a Gmail filter
    pub async fn delete(&self, filter_id: &str) -> Result<()> {
        let url = format!("{}/{}", Self::base_url(), filter_id);

        let response = self
            .client
            .delete(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else if response.status().as_u16() == 404 {
            Err(GmailMcpError::Gmail(GmailApiError::FilterNotFound {
                filter_id: filter_id.to_string(),
            }))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to delete filter ({}): {}", status, text),
            }))
        }
    }
}

/// Result of listing filters
#[derive(Debug, Clone)]
pub struct FilterListResult {
    /// All filters
    pub filters: Vec<Filter>,

    /// Filter count
    pub count: usize,
}

/// Pre-defined filter templates for common scenarios
pub struct FilterTemplates;

impl FilterTemplates {
    /// Filter emails from a specific sender
    pub fn from_sender(
        sender_email: &str,
        label_ids: Option<Vec<String>>,
        archive: bool,
    ) -> (FilterCriteria, FilterAction) {
        let criteria = FilterCriteria {
            from: Some(sender_email.to_string()),
            ..Default::default()
        };

        let action = FilterAction {
            add_label_ids: label_ids,
            remove_label_ids: if archive {
                Some(vec!["INBOX".to_string()])
            } else {
                None
            },
            ..Default::default()
        };

        (criteria, action)
    }

    /// Filter emails with specific subject
    pub fn with_subject(
        subject_text: &str,
        label_ids: Option<Vec<String>>,
        mark_as_read: bool,
    ) -> (FilterCriteria, FilterAction) {
        let criteria = FilterCriteria {
            subject: Some(subject_text.to_string()),
            ..Default::default()
        };

        let action = FilterAction {
            add_label_ids: label_ids,
            remove_label_ids: if mark_as_read {
                Some(vec!["UNREAD".to_string()])
            } else {
                None
            },
            ..Default::default()
        };

        (criteria, action)
    }

    /// Filter emails with attachments
    pub fn with_attachments(label_ids: Option<Vec<String>>) -> (FilterCriteria, FilterAction) {
        let criteria = FilterCriteria {
            has_attachment: Some(true),
            ..Default::default()
        };

        let action = FilterAction {
            add_label_ids: label_ids,
            ..Default::default()
        };

        (criteria, action)
    }

    /// Filter large emails
    pub fn large_emails(
        size_in_bytes: i64,
        label_ids: Option<Vec<String>>,
    ) -> (FilterCriteria, FilterAction) {
        let criteria = FilterCriteria {
            size: Some(size_in_bytes),
            size_comparison: Some(SizeComparison::Larger),
            ..Default::default()
        };

        let action = FilterAction {
            add_label_ids: label_ids,
            ..Default::default()
        };

        (criteria, action)
    }

    /// Filter emails containing specific text
    pub fn containing_text(
        search_text: &str,
        label_ids: Option<Vec<String>>,
        mark_important: bool,
    ) -> (FilterCriteria, FilterAction) {
        let criteria = FilterCriteria {
            query: Some(format!("\"{}\"", search_text)),
            ..Default::default()
        };

        let mut add_labels = label_ids.unwrap_or_default();
        if mark_important {
            add_labels.push("IMPORTANT".to_string());
        }

        let action = FilterAction {
            add_label_ids: if add_labels.is_empty() {
                None
            } else {
                Some(add_labels)
            },
            ..Default::default()
        };

        (criteria, action)
    }

    /// Filter mailing list emails
    pub fn mailing_list(
        list_identifier: &str,
        label_ids: Option<Vec<String>>,
        archive: bool,
    ) -> (FilterCriteria, FilterAction) {
        let criteria = FilterCriteria {
            query: Some(format!(
                "list:{} OR subject:[{}]",
                list_identifier, list_identifier
            )),
            ..Default::default()
        };

        let action = FilterAction {
            add_label_ids: label_ids,
            remove_label_ids: if archive {
                Some(vec!["INBOX".to_string()])
            } else {
                None
            },
            ..Default::default()
        };

        (criteria, action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_template_from_sender() {
        let (criteria, action) =
            FilterTemplates::from_sender("test@example.com", Some(vec!["Label_1".to_string()]), true);

        assert_eq!(criteria.from, Some("test@example.com".to_string()));
        assert_eq!(
            action.add_label_ids,
            Some(vec!["Label_1".to_string()])
        );
        assert_eq!(action.remove_label_ids, Some(vec!["INBOX".to_string()]));
    }

    #[test]
    fn test_filter_template_large_emails() {
        let (criteria, _action) = FilterTemplates::large_emails(1024 * 1024, None);

        assert_eq!(criteria.size, Some(1024 * 1024));
        assert_eq!(criteria.size_comparison, Some(SizeComparison::Larger));
    }

    #[test]
    fn test_filter_template_with_attachments() {
        let (criteria, _action) = FilterTemplates::with_attachments(None);

        assert_eq!(criteria.has_attachment, Some(true));
    }
}

