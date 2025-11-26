//! Label management for Gmail
//!
//! Provides comprehensive label management functionality.

use crate::error::{GmailApiError, GmailMcpError, Result};
use crate::gmail::types::{CreateLabelRequest, Label, LabelList, UpdateLabelRequest};

/// Label manager for Gmail operations
pub struct LabelManager<'a> {
    client: &'a reqwest::Client,
    access_token: &'a str,
}

impl<'a> LabelManager<'a> {
    /// Create a new label manager
    pub fn new(client: &'a reqwest::Client, access_token: &'a str) -> Self {
        Self {
            client,
            access_token,
        }
    }

    /// Base URL for labels API
    fn base_url() -> String {
        format!("{}/users/me/labels", crate::config::gmail::API_BASE_URL)
    }

    /// Create a new Gmail label
    pub async fn create(
        &self,
        name: &str,
        message_list_visibility: Option<&str>,
        label_list_visibility: Option<&str>,
    ) -> Result<Label> {
        let request = CreateLabelRequest {
            name: name.to_string(),
            message_list_visibility: message_list_visibility.map(|s| s.to_string()),
            label_list_visibility: label_list_visibility.map(|s| s.to_string()),
        };

        let response = self
            .client
            .post(Self::base_url())
            .bearer_auth(self.access_token)
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            if text.contains("already exists") {
                return Err(GmailMcpError::Gmail(GmailApiError::LabelAlreadyExists {
                    name: name.to_string(),
                }));
            }

            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to create label ({}): {}", status, text),
            }))
        }
    }

    /// Update an existing Gmail label
    pub async fn update(&self, label_id: &str, updates: UpdateLabelRequest) -> Result<Label> {
        let url = format!("{}/{}", Self::base_url(), label_id);

        // First verify the label exists
        self.get(label_id).await?;

        let response = self
            .client
            .put(&url)
            .bearer_auth(self.access_token)
            .json(&updates)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            if status.as_u16() == 404 {
                return Err(GmailMcpError::Gmail(GmailApiError::LabelNotFound {
                    label_id: label_id.to_string(),
                }));
            }

            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to update label ({}): {}", status, text),
            }))
        }
    }

    /// Delete a Gmail label
    pub async fn delete(&self, label_id: &str) -> Result<()> {
        // First verify the label exists and is not a system label
        let label = self.get(label_id).await?;

        if label.label_type.as_deref() == Some("system") {
            return Err(GmailMcpError::Gmail(GmailApiError::CannotDeleteSystemLabel {
                label_id: label_id.to_string(),
            }));
        }

        let url = format!("{}/{}", Self::base_url(), label_id);

        let response = self
            .client
            .delete(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            if status.as_u16() == 404 {
                return Err(GmailMcpError::Gmail(GmailApiError::LabelNotFound {
                    label_id: label_id.to_string(),
                }));
            }

            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to delete label ({}): {}", status, text),
            }))
        }
    }

    /// Get a specific label by ID
    pub async fn get(&self, label_id: &str) -> Result<Label> {
        let url = format!("{}/{}", Self::base_url(), label_id);

        let response = self
            .client
            .get(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else if response.status().as_u16() == 404 {
            Err(GmailMcpError::Gmail(GmailApiError::LabelNotFound {
                label_id: label_id.to_string(),
            }))
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to get label ({}): {}", status, text),
            }))
        }
    }

    /// List all Gmail labels
    pub async fn list(&self) -> Result<LabelListResult> {
        let response = self
            .client
            .get(Self::base_url())
            .bearer_auth(self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            let label_list: LabelList = response.json().await?;
            let labels = label_list.labels;

            let system_labels: Vec<Label> = labels
                .iter()
                .filter(|l| l.label_type.as_deref() == Some("system"))
                .cloned()
                .collect();

            let user_labels: Vec<Label> = labels
                .iter()
                .filter(|l| l.label_type.as_deref() == Some("user"))
                .cloned()
                .collect();

            Ok(LabelListResult {
                all: labels,
                system: system_labels.clone(),
                user: user_labels.clone(),
                count: LabelCount {
                    total: system_labels.len() + user_labels.len(),
                    system: system_labels.len(),
                    user: user_labels.len(),
                },
            })
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(GmailMcpError::Gmail(GmailApiError::RequestFailed {
                message: format!("Failed to list labels ({}): {}", status, text),
            }))
        }
    }

    /// Find a label by name (case-insensitive)
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Label>> {
        let result = self.list().await?;
        let name_lower = name.to_lowercase();

        Ok(result
            .all
            .into_iter()
            .find(|l| l.name.to_lowercase() == name_lower))
    }

    /// Get or create a label by name
    pub async fn get_or_create(
        &self,
        name: &str,
        message_list_visibility: Option<&str>,
        label_list_visibility: Option<&str>,
    ) -> Result<Label> {
        // First try to find existing label
        if let Some(label) = self.find_by_name(name).await? {
            return Ok(label);
        }

        // If not found, create new one
        self.create(name, message_list_visibility, label_list_visibility)
            .await
    }
}

/// Result of listing labels
#[derive(Debug, Clone)]
pub struct LabelListResult {
    /// All labels
    pub all: Vec<Label>,

    /// System labels only
    pub system: Vec<Label>,

    /// User labels only
    pub user: Vec<Label>,

    /// Label counts
    pub count: LabelCount,
}

/// Label count statistics
#[derive(Debug, Clone)]
pub struct LabelCount {
    /// Total label count
    pub total: usize,

    /// System label count
    pub system: usize,

    /// User label count
    pub user: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_list_result() {
        let result = LabelListResult {
            all: vec![],
            system: vec![],
            user: vec![],
            count: LabelCount {
                total: 0,
                system: 0,
                user: 0,
            },
        };
        assert_eq!(result.count.total, 0);
    }
}

