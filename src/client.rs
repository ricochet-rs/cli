use crate::config::{Config, parse_server_url};
use anyhow::{Context, Result};
use reqwest::{Client, Response, StatusCode};
use ricochet_core::content::ContentItem;
use serde::de::DeserializeOwned;
use std::fs::read_to_string;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use tokio::io::{AsyncRead, ReadBuf};
use url::Url;

// Progress tracking wrapper for AsyncRead
struct ProgressReader<R> {
    reader: R,
    progress_bar: indicatif::ProgressBar,
    bytes_read: u64,
}

impl<R: AsyncRead + Unpin> AsyncRead for ProgressReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.reader).poll_read(cx, buf);
        let after = buf.filled().len();
        let bytes_read = (after - before) as u64;

        self.bytes_read += bytes_read;
        self.progress_bar.set_position(self.bytes_read);

        result
    }
}

pub struct RicochetClient {
    pub(crate) client: Client,
    pub(crate) base_url: Url,
    pub(crate) api_key: String,
}

impl RicochetClient {
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        let base_url = config.server_url()?;

        Ok(Self {
            client,
            base_url,
            api_key: config.api_key()?,
        })
    }

    pub fn new_with_key(server: String, api_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        let base_url = parse_server_url(&server)?;

        Ok(Self {
            client,
            base_url,
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

    fn mask_api_key(key: &str) -> String {
        if key.is_empty() {
            "No API key provided".to_string()
        } else if key.len() > 12 {
            format!("{}...{}", &key[..8], &key[key.len().saturating_sub(4)..])
        } else {
            "***".to_string()
        }
    }

    pub async fn validate_key(&self) -> Result<bool> {
        let mut url = self.base_url.clone();
        url.set_path("/api/v0/check_key");
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        Ok(response.status() == StatusCode::OK)
    }

    pub async fn list_items(&self) -> Result<Vec<serde_json::Value>> {
        let mut url = self.base_url.clone();
        url.set_path("/api/v0/user/items");
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        match Self::handle_response(response).await {
            Ok(result) => Ok(result),
            Err(e) => {
                // Check if this is an authentication error
                if e.to_string().contains("403") && e.to_string().contains("Invalid API key") {
                    let masked_key = Self::mask_api_key(&self.api_key);
                    anyhow::bail!("Authentication failed. API key used: {}", masked_key)
                } else {
                    Err(e)
                }
            }
        }
    }

    pub async fn deploy(
        &self,
        path: &Path,
        content_id: Option<String>,
        toml_path: &Path,
        pb: &indicatif::ProgressBar,
        debug: bool,
    ) -> Result<serde_json::Value> {
        let mut url = self.base_url.clone();
        url.set_path("/api/v0/content/upload");

        if debug {
            eprintln!("Debug: API URL: {url:?}");
            eprintln!("Debug: Base URL: {}", self.base_url);
        }

        let content_item = ContentItem::from_toml(&read_to_string(toml_path)?)?;
        let include = content_item.content.include;
        let exclude = content_item.content.exclude;

        // Create a tar bundle from the directory
        pb.set_message("Creating bundle...");
        let tar_path = std::env::temp_dir().join(format!("ricochet-{}.tar.gz", ulid::Ulid::new()));
        crate::utils::create_bundle(path, &tar_path, include, exclude, debug)?;

        // Get file size for progress tracking
        let file_size = tokio::fs::metadata(&tar_path).await?.len();

        // Change to progress bar with bytes
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{spinner:.green} {msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%)")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_length(file_size);
        pb.set_position(0);
        pb.set_message("Uploading to server");

        let bundle_file = tokio::fs::File::open(&tar_path).await?;
        let progress_reader = ProgressReader {
            reader: bundle_file,
            progress_bar: pb.clone(),
            bytes_read: 0,
        };
        let bundle_body =
            reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(progress_reader));

        let mut form = reqwest::multipart::Form::new().part(
            "bundle",
            reqwest::multipart::Part::stream(bundle_body)
                .file_name("bundle.tar.gz")
                .mime_str("application/x-tar")?,
        );

        if let Some(id) = content_id {
            // Updating existing content
            form = form.text("id", id);
        } else {
            // Creating new content - include the config file
            let toml_file = tokio::fs::File::open(toml_path).await?;
            let toml_body =
                reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(toml_file));
            form = form.part(
                "config",
                reqwest::multipart::Part::stream(toml_body)
                    .file_name("_ricochet.toml")
                    .mime_str("application/toml")?,
            );
        }

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        match Self::handle_response(response).await {
            Ok(result) => Ok(result),
            Err(e) => {
                // Check if this is an authentication error
                if e.to_string().contains("403") && e.to_string().contains("Invalid API key") {
                    let masked_key = Self::mask_api_key(&self.api_key);
                    anyhow::bail!("Authentication failed. API key used: {}", masked_key)
                } else {
                    Err(e)
                }
            }
        }
    }

    pub async fn get_status(&self, id: &str) -> Result<serde_json::Value> {
        // Get deployments for the item
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/deployments", id));
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn invoke(&self, id: &str, params: Option<String>) -> Result<serde_json::Value> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/invoke", id));

        let body = if let Some(params) = params {
            serde_json::from_str(&params)?
        } else {
            serde_json::json!({})
        };

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn stop_invocation(&self, id: &str, invocation_id: &str) -> Result<()> {
        let mut url = self.base_url.clone();
        url.set_path(&format!(
            "/api/v0/content/{}/invocations/{}/stop",
            id, invocation_id
        ));

        let response = self
            .client
            .post(url)
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
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/instances/{}/stop", id, pid));

        let response = self
            .client
            .post(url)
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
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}", id));

        let response = self
            .client
            .delete(url)
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
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/schedule", id));

        let body = serde_json::json!({
            "schedule": schedule
        });

        let response = self
            .client
            .patch(url)
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
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/settings", id));

        let body: serde_json::Value = serde_json::from_str(settings)?;

        let response = self
            .client
            .patch(url)
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
