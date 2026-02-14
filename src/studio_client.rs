//! HTTP client for Skills Studio ingest API (app.supervaize.com).

use crate::error::{RelayError, Result};
use reqwest::Client;
use std::time::Duration;

pub struct StudioClient {
    base_url: String,
    client: Client,
}

impl StudioClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        Self { base_url, client }
    }

    /// Upload a skill zip to ingest/upload. Returns skill id from response.
    pub async fn upload_skill(&self, token: &str, zip_bytes: Vec<u8>) -> Result<String> {
        let url = format!("{}/ingest/upload", self.base_url.trim_end_matches('/'));
        let part = reqwest::multipart::Part::bytes(zip_bytes)
            .file_name("skill.zip")
            .mime_str("application/zip")
            .map_err(|e| RelayError::Studio(format!("Multipart error: {}", e)))?;
        let form = reqwest::multipart::Form::new().part("file", part);
        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| RelayError::Network(e))?;

        if response.status().as_u16() == 401 {
            return Err(RelayError::Studio(
                "Unauthorized - studio token may be expired or invalid".to_string(),
            ));
        }
        if response.status().as_u16() == 403 {
            return Err(RelayError::Studio(
                "Forbidden - insufficient permissions for ingest".to_string(),
            ));
        }
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(RelayError::Studio(format!(
                "Ingest failed (status {}): {}",
                status, text
            )));
        }
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RelayError::Studio(format!("Invalid response: {}", e)))?;
        let skill_id = json
            .get("id")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| {
                RelayError::Studio("Response missing skill id".to_string())
            })?;
        Ok(skill_id)
    }
}
