use crate::config::Config;
use anyhow::{Context, Result};
use reqwest::{Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::path::Path;

pub struct RicochetClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl RicochetClient {
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        Ok(Self {
            client,
            base_url: config.server_url()?,
            api_key: config.api_key()?,
        })
    }

    pub fn new_with_key(server: String, api_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        Ok(Self {
            client,
            base_url: server,
            api_key,
        })
    }

    async fn handle_response<T: DeserializeOwned>(response: Response) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            response
                .json::<T>()
                .await
                .context("Failed to parse response")
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Request failed with status {}: {}", status, error_text)
        }
    }

    pub async fn validate_key(&self) -> Result<bool> {
        let url = format!("{}/api/v0/check_key", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        Ok(response.status() == StatusCode::OK)
    }

    pub async fn list_items(&self) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/api/v0/user/items", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn deploy(
        &self,
        path: &Path,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/api/v0/content/upload", self.base_url);

        // Check if path is a directory or a bundle
        let file = if path.is_dir() {
            // Create a tar bundle from the directory
            let tar_path =
                std::env::temp_dir().join(format!("ricochet-{}.tar.gz", ulid::Ulid::new()));
            crate::utils::create_bundle(path, &tar_path)?;
            tokio::fs::File::open(&tar_path).await?
        } else {
            tokio::fs::File::open(path).await?
        };

        let file_body = reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(file));

        let mut form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::stream(file_body)
                .file_name(path.file_name().unwrap().to_string_lossy().to_string()),
        );

        if let Some(name) = name {
            form = form.text("name", name);
        }
        if let Some(description) = description {
            form = form.text("description", description);
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn get_status(&self, id: &str) -> Result<serde_json::Value> {
        // Get deployments for the item
        let url = format!("{}/api/v0/content/{}/deployments", self.base_url, id);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn invoke(&self, id: &str, params: Option<String>) -> Result<serde_json::Value> {
        let url = format!("{}/api/v0/content/{}/invoke", self.base_url, id);

        let body = if let Some(params) = params {
            serde_json::from_str(&params)?
        } else {
            serde_json::json!({})
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn stop_invocation(&self, id: &str, invocation_id: &str) -> Result<()> {
        let url = format!(
            "{}/api/v0/content/{}/invocations/{}/stop",
            self.base_url, id, invocation_id
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to stop invocation: {}", error_text)
        }

        Ok(())
    }

    pub async fn stop_instance(&self, id: &str, pid: &str) -> Result<()> {
        let url = format!(
            "{}/api/v0/content/{}/instances/{}/stop",
            self.base_url, id, pid
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to stop instance: {}", error_text)
        }

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/v0/admin/delete/{}", self.base_url, id);

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to delete item: {}", error_text)
        }

        Ok(())
    }

    pub async fn update_schedule(&self, id: &str, schedule: Option<String>) -> Result<()> {
        let url = format!("{}/api/v0/content/{}/schedule", self.base_url, id);

        let body = serde_json::json!({
            "schedule": schedule
        });

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to update schedule: {}", error_text)
        }

        Ok(())
    }

    pub async fn update_settings(&self, id: &str, settings: &str) -> Result<()> {
        let url = format!("{}/api/v0/content/{}/settings", self.base_url, id);

        let body: serde_json::Value = serde_json::from_str(settings)?;

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to update settings: {}", error_text)
        }

        Ok(())
    }
}
