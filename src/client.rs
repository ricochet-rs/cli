use crate::config::{ServerConfig, parse_server_url};
use anyhow::{Context, Result};
use colored::Colorize;
use reqwest::{Client, Response, StatusCode};
use ricochet_core::{
    config::git::{GitCredential, GitProtocol, GitRepo},
    content::{ContentItem, OwnershipScope},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use std::{
    fs::read_to_string,
    path::Path,
    pin::Pin,
    task::{Context as TaskContext, Poll},
};
use tokio::io::{AsyncRead, ReadBuf};
use url::Url;

/// Where a local deploy reports its progress.
#[derive(Clone)]
pub(crate) enum DeployProgress {
    /// A spinner, then a byte bar, drawn on stderr for a person.
    Bar(indicatif::ProgressBar),
    /// Newline-delimited JSON events on stdout for a program.
    Events,
}

/// A request the server failed, keeping its cause for [`DeployFailure`] to classify.
#[derive(Debug)]
pub(crate) enum ApiError {
    /// The server answered with a failure status.
    Status { status: StatusCode, message: String },
    /// The server did not accept the API key.
    Credentials { message: String },
    /// The server could not be reached.
    Unreachable { message: String },
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Status { message, .. }
            | Self::Credentials { message }
            | Self::Unreachable { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for ApiError {}

/// Why a deploy failed, so a program can react without parsing the message.
#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum DeployFailure {
    /// The local project cannot be deployed as configured.
    Project,
    /// The server did not accept the API key.
    Auth,
    /// The server could not be reached.
    Network,
    /// The server refused the deployment request.
    Rejected { status: u16 },
    /// The server failed while handling the request.
    Server {
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<u16>,
    },
}

impl DeployFailure {
    fn classify(error: &anyhow::Error) -> Self {
        for cause in error.chain() {
            if let Some(api) = cause.downcast_ref::<ApiError>() {
                return match api {
                    ApiError::Credentials { .. } => Self::Auth,
                    ApiError::Unreachable { .. } => Self::Network,
                    ApiError::Status { status, .. }
                        if *status == StatusCode::UNAUTHORIZED
                            || *status == StatusCode::FORBIDDEN =>
                    {
                        Self::Auth
                    }
                    ApiError::Status { status, .. } if status.is_server_error() => Self::Server {
                        status: Some(status.as_u16()),
                    },
                    ApiError::Status { status, .. } => Self::Rejected {
                        status: status.as_u16(),
                    },
                };
            }
            if let Some(request) = cause.downcast_ref::<reqwest::Error>() {
                return if request.is_decode() {
                    Self::Server { status: None }
                } else {
                    Self::Network
                };
            }
        }
        Self::Project
    }
}

/// One line of `deploy -F json` output.
#[derive(Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum DeployEvent {
    Bundling {
        files: usize,
    },
    Uploading {
        bytes_sent: u64,
        bytes_total: u64,
    },
    Error {
        #[serde(flatten)]
        failure: DeployFailure,
        message: String,
    },
}

impl DeployEvent {
    fn emit(&self) -> serde_json::Result<()> {
        println!("{}", serde_json::to_string(self)?);
        Ok(())
    }
}

impl DeployProgress {
    fn bundled(&self, files: usize) -> Result<()> {
        if let Self::Events = self {
            DeployEvent::Bundling { files }.emit()?;
        }
        Ok(())
    }

    fn upload_started(&self, bytes_total: u64) -> Result<()> {
        match self {
            Self::Bar(pb) => {
                pb.set_style(
                    indicatif::ProgressStyle::default_bar()
                        .template("{spinner:.green} {msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%)")?
                        .progress_chars("#>-"),
                );
                pb.set_length(bytes_total);
                pb.set_position(0);
                pb.set_message("Uploading to server");
            }
            Self::Events => DeployEvent::Uploading {
                bytes_sent: 0,
                bytes_total,
            }
            .emit()?,
        }
        Ok(())
    }

    /// Report an upload advancing from `before` to `after` bytes, one event per percent for a program.
    fn uploaded(&self, before: u64, after: u64, bytes_total: u64) -> serde_json::Result<()> {
        match self {
            Self::Bar(pb) => pb.set_position(after),
            Self::Events => {
                let percent = |bytes: u64| bytes * 100 / bytes_total.max(1);
                if percent(after) > percent(before) {
                    DeployEvent::Uploading {
                        bytes_sent: after,
                        bytes_total,
                    }
                    .emit()?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn finish(&self) {
        if let Self::Bar(pb) = self {
            pb.finish_and_clear();
        }
    }

    /// Write the final `done` event, carrying every field of the server's `response`.
    pub(crate) fn emit_done(response: &serde_json::Value) {
        let mut done = serde_json::Map::from_iter([("event".into(), "done".into())]);
        if let Some(fields) = response.as_object() {
            done.extend(fields.clone());
        }
        println!("{}", serde_json::Value::Object(done));
    }

    /// Write the final `error` event, classifying `error` for a program.
    pub(crate) fn emit_error(error: &anyhow::Error) -> Result<()> {
        DeployEvent::Error {
            failure: DeployFailure::classify(error),
            message: console::strip_ansi_codes(&error.to_string()).into_owned(),
        }
        .emit()?;
        Ok(())
    }
}

/// Reports the bytes read from the bundle as they are uploaded.
struct ProgressReader<R> {
    reader: R,
    progress: DeployProgress,
    bytes_read: u64,
    bytes_total: u64,
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

        let bytes_before = self.bytes_read;
        self.bytes_read += (after - before) as u64;
        self.progress
            .uploaded(bytes_before, self.bytes_read, self.bytes_total)
            .map_err(std::io::Error::other)?;

        result
    }
}

pub struct RicochetClient {
    pub(crate) client: Client,
    pub(crate) base_url: Url,
    pub(crate) api_key: String,
}

impl RicochetClient {
    pub fn new(server_config: &ServerConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        let api_key = server_config
            .api_key
            .clone()
            .context("No API key configured. Use 'ricochet login' to authenticate")?;

        Ok(Self {
            client,
            base_url: server_config.url.clone(),
            api_key,
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
            Err(ApiError::Status {
                status,
                message: format!("Request failed with status {status}: {error_text}"),
            }
            .into())
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

    /// Fetch the server's RSA public key (PKCS#1 PEM) used to encrypt env vars.
    pub async fn get_public_key(&self) -> Result<rsa::RsaPublicKey> {
        let mut url = self.base_url.clone();
        url.set_path("/api/v0/public-key");
        let response = self.client.get(url).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(ApiError::Status {
                status,
                message: format!("Failed to fetch public key (status {status}): {body}"),
            }
            .into());
        }
        crate::crypto::parse_public_key_pem(&body)
    }

    /// Check if a key is expired and report if so
    /// Use this as a pre-flight check for all API calls where appropriate
    pub async fn preflight_key_check(&self) -> Result<()> {
        let server_url = self.base_url.as_str().trim_end_matches('/');
        let login_cmd = format!("ricochet login -S {server_url}").bright_cyan();
        match self.validate_key().await {
            Ok(true) => Ok(()),
            Ok(false) => Err(ApiError::Credentials {
                message: format!(
                    "Credentials are invalid or expired for server {server_url}.\nRun {login_cmd} to authenticate."
                ),
            }
            .into()),
            Err(e) => Err(ApiError::Unreachable {
                message: format!(
                    "Failed to validate credentials for {server_url}:\n{} {}\nRun {login_cmd} to authenticate.",
                    "⚠".bright_yellow(),
                    e.to_string().dimmed()
                ),
            }
            .into()),
        }
    }

    pub async fn list_items(&self, scope: OwnershipScope) -> Result<Vec<serde_json::Value>> {
        let mut url = self.base_url.clone();
        url.set_path("/api/v0/user/items");
        match scope {
            // Omitting the parameter keeps the request identical to the one
            // servers without the instance-wide listing already answer.
            OwnershipScope::Owned => {}
            OwnershipScope::All => url.set_query(Some("scope=all")),
        }

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        if response.status() == StatusCode::FORBIDDEN {
            let body = response.text().await.unwrap_or_default();
            return Err(self.forbidden_listing_error(scope, &body));
        }

        Self::handle_response(response).await
    }

    /// Explain a 403 from the item listing in terms of what the caller asked for.
    fn forbidden_listing_error(&self, scope: OwnershipScope, body: &str) -> anyhow::Error {
        if body.contains("Invalid API key") {
            return anyhow::anyhow!(
                "Authentication failed. API key used: {}",
                Self::mask_api_key(&self.api_key)
            );
        }

        match scope {
            OwnershipScope::All => anyhow::anyhow!(
                "Listing every item on the instance requires an instance admin API key.\nServer response: {body}"
            ),
            OwnershipScope::Owned => {
                anyhow::anyhow!("Request failed with status 403 Forbidden: {body}")
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn deploy(
        &self,
        path: &Path,
        content_id: Option<String>,
        toml_path: &Path,
        extra_root_files: &[(std::path::PathBuf, String)],
        env_vars: Option<crate::crypto::RsaEncryptedEnvVars>,
        progress: &DeployProgress,
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
        if let DeployProgress::Bar(pb) = progress {
            pb.set_message("Creating bundle...");
        }
        let tar_path =
            std::env::temp_dir().join(format!("ricochet-{}.tar.gz", ulid::Ulid::generate()));
        let files = crate::utils::create_bundle(
            path,
            &tar_path,
            include,
            exclude,
            extra_root_files,
            debug,
        )?;
        progress.bundled(files)?;

        let bytes_total = tokio::fs::metadata(&tar_path).await?.len();
        progress.upload_started(bytes_total)?;

        let bundle_file = tokio::fs::File::open(&tar_path).await?;
        let progress_reader = ProgressReader {
            reader: bundle_file,
            progress: progress.clone(),
            bytes_read: 0,
            bytes_total,
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
        }
        // always include the config file
        let toml_file = tokio::fs::File::open(toml_path).await?;
        let toml_body = reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(toml_file));
        form = form.part(
            "config",
            reqwest::multipart::Part::stream(toml_body)
                .file_name("_ricochet.toml")
                .mime_str("application/toml")?,
        );

        if let Some(envs) = env_vars {
            form = form.text("env_vars", serde_json::to_string(&envs)?);
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
                    Err(ApiError::Credentials {
                        message: format!("Authentication failed. API key used: {masked_key}"),
                    }
                    .into())
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Create a Git-backed content item and start its first deployment.
    /// `config` is the raw `_ricochet.toml` contents; omit it to read the configuration from the repository.
    pub async fn deploy_git(
        &self,
        repo: &GitRepo,
        config: Option<String>,
        credential_id: Option<String>,
    ) -> Result<serde_json::Value> {
        let mut url = self.base_url.clone();
        url.set_path("/api/v0/deploy/git");

        let mut form = reqwest::multipart::Form::new().text("repo", serde_json::to_string(repo)?);

        if let Some(cfg) = config {
            form = form.text("config", cfg);
        }
        if let Some(cred) = credential_id {
            form = form.text("credential_id", cred);
        }

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        Self::handle_response(response).await
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

    pub async fn invoke(
        &self,
        id: &str,
        params: Option<String>,
    ) -> Result<crate::item::invoke::Invoked> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{id}/invoke"));

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

    /// Read one run of a task, finished or not.
    pub async fn get_invocation(
        &self,
        id: &str,
        invocation_id: &str,
    ) -> Result<crate::task::invocation::Invocation> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{id}/invocations/{invocation_id}"));

        // Short enough that a stalled check cannot outlast the retry window of `wait`.
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        // The server records a run before returning its ID, so a fresh run is never missing.
        if response.status() == StatusCode::NOT_FOUND {
            anyhow::bail!(
                "Task run {invocation_id} of {id} was not found. If the ID is correct, update the Ricochet server to follow task runs from the CLI."
            );
        }
        // Raised as a `reqwest::Error` so `wait` can retry a passing server fault.
        if response.status().is_server_error() {
            response.error_for_status_ref()?;
        }

        Self::handle_response(response).await
    }

    /// Schedule a task to run on a cron schedule
    pub async fn schedule(&self, id: &str, schedule: &str) -> Result<serde_json::Value> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{id}/schedule"));

        let resp = self
            .client
            .patch(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&json!({"schedule": schedule}))
            .send()
            .await?;

        Self::handle_response(resp).await
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

    pub async fn list_instances(&self, id: &str) -> Result<serde_json::Value> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/instances", id));

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        Self::handle_response(response).await
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

    pub async fn update_settings(&self, id: &str, settings: &serde_json::Value) -> Result<()> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/settings", id));

        let response = self
            .client
            .patch(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .json(settings)
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

    pub async fn list_deployments(
        &self,
        content_ulid: &str,
    ) -> Result<Vec<crate::item::deployment::DeploymentRow>> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/deployments", content_ulid));
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;
        Self::handle_response(response).await
    }

    pub async fn get_deployment(
        &self,
        deployment_ulid: &str,
    ) -> Result<crate::item::deployment::DeploymentRow> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/deployments/{}", deployment_ulid));
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;
        Self::handle_response(response).await
    }

    /// List the names of an item's environment variables.
    pub async fn get_env_vars(&self, id: &str) -> Result<Vec<String>> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/env-vars", id));
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;
        Self::handle_response(response).await
    }

    /// Delete an environment variable by name. Returns the item's remaining
    /// environment variable names.
    pub async fn delete_env_var(&self, id: &str, name: &str) -> Result<Vec<String>> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/env-vars/{}", id, name));
        let response = self
            .client
            .delete(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;
        Self::handle_response(response).await
    }

    /// Upsert environment variables, leaving others untouched. Returns the
    /// item's remaining environment variable names.
    pub async fn upsert_env_vars(
        &self,
        id: &str,
        encrypted: &crate::crypto::RsaEncryptedEnvVars,
    ) -> Result<Vec<String>> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/env-vars", id));
        let response = self
            .client
            .patch(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .json(encrypted)
            .send()
            .await?;
        Self::handle_response(response).await
    }

    /// Replace all environment variables. Returns the item's remaining
    /// environment variable names.
    pub async fn replace_env_vars(
        &self,
        id: &str,
        encrypted: &crate::crypto::RsaEncryptedEnvVars,
    ) -> Result<Vec<String>> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/env-vars", id));
        let response = self
            .client
            .put(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .json(encrypted)
            .send()
            .await?;
        Self::handle_response(response).await
    }

    /// List Git credentials owned by the current user, or by `user_id` if the caller is an admin.
    /// Credential values are never returned.
    pub async fn list_credentials(
        &self,
        user_id: Option<&str>,
        protocol: Option<GitProtocol>,
    ) -> Result<Vec<GitCredential>> {
        let mut url = self.base_url.clone();
        url.set_path("/api/v0/user/credentials");
        {
            let mut pairs = url.query_pairs_mut();
            if let Some(uid) = user_id {
                pairs.append_pair("user_id", uid);
            }
            if let Some(p) = protocol {
                pairs.append_pair("type", &p.to_string());
            }
        }

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn get_ricochet_toml(&self, id: &str) -> Result<String> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v0/content/{}/toml", id));

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to fetch ricochet.toml: {}", error_text)
        }

        let toml_content = response.text().await?;
        Ok(toml_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(code: u16) -> anyhow::Error {
        ApiError::Status {
            status: StatusCode::from_u16(code).expect("a valid status code"),
            message: format!("status {code}"),
        }
        .into()
    }

    #[test]
    fn failure_statuses_classify_by_who_must_act() {
        assert_eq!(DeployFailure::classify(&status(401)), DeployFailure::Auth);
        assert_eq!(DeployFailure::classify(&status(403)), DeployFailure::Auth);
        assert_eq!(
            DeployFailure::classify(&status(422)),
            DeployFailure::Rejected { status: 422 }
        );
        assert_eq!(
            DeployFailure::classify(&status(503)),
            DeployFailure::Server { status: Some(503) }
        );
    }

    #[test]
    fn credential_and_connection_failures_classify_without_a_status() {
        let credentials = ApiError::Credentials {
            message: "expired".into(),
        };
        let unreachable = ApiError::Unreachable {
            message: "refused".into(),
        };
        assert_eq!(
            DeployFailure::classify(&credentials.into()),
            DeployFailure::Auth
        );
        assert_eq!(
            DeployFailure::classify(&unreachable.into()),
            DeployFailure::Network
        );
    }

    #[test]
    fn a_failure_found_through_context_keeps_its_class() {
        let wrapped = status(422).context("Deployment failed");
        assert_eq!(
            DeployFailure::classify(&wrapped),
            DeployFailure::Rejected { status: 422 }
        );
    }

    #[test]
    fn a_failure_without_a_server_cause_is_the_project() {
        let local = anyhow::anyhow!("Required package file `renv.lock` not found.");
        assert_eq!(DeployFailure::classify(&local), DeployFailure::Project);
    }

    #[test]
    fn an_error_event_carries_its_kind_beside_the_message() -> Result<()> {
        let event = DeployEvent::Error {
            failure: DeployFailure::Rejected { status: 422 },
            message: "bad bundle".into(),
        };
        assert_eq!(
            serde_json::to_value(&event)?,
            json!({"event": "error", "kind": "rejected", "status": 422, "message": "bad bundle"})
        );
        Ok(())
    }
}
