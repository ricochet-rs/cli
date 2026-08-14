use super::auth_ui;
use crate::{client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;
use dialoguer::Password;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use unicode_icons::icons::symbols;
use url::Url;

const HOSTED_TRIAL_SERVER: &str = "https://try.ricochet.rs";
const HOSTED_TRIAL_PROFILE: &str = "try";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoginServerSource {
    Configured,
    HostedTrialFallback,
}

#[derive(Debug)]
struct AuthState {
    received_callback: bool,
    session_cookie: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct CreateApiKeyRequest {
    pub(crate) name: String,
    pub(crate) expires_in_hours: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) expires_at: Option<String>, // Alternative: ISO 8601 timestamp
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ApiKeyResponse {
    pub(crate) key: String,
    #[allow(dead_code)]
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) expires_at: Option<String>,
}

pub async fn login(
    config: &mut Config,
    server_ref: Option<&str>,
    api_key: Option<String>,
) -> Result<()> {
    println!("🔐 Authenticating against Ricochet server\n");

    // Resolve which server to authenticate with
    let (server_url, server_name, server_source) = resolve_login_server(config, server_ref)?;

    println!("Server: {}", server_url.as_str().bright_cyan());
    if let Some(ref name) = server_name {
        println!("Profile: {}", name.bright_cyan());
    }
    if server_source == LoginServerSource::HostedTrialFallback {
        println!("{}", "Using the hosted trial default.".dimmed());
        println!(
            "{}",
            "Register your own server with `ricochet server add <NAME> <URL>`.".dimmed()
        );
    }
    println!();

    // Check if already authenticated with this server
    let server_config = config.resolve_server(server_ref).ok();
    if let Some(ref sc) = server_config
        && let Some(ref existing_key) = sc.api_key
    {
        let client = RicochetClient::new_with_key(server_url.to_string(), existing_key.clone())?;
        if client.validate_key().await.unwrap_or(false) {
            println!(
                "{} Already authenticated",
                symbols::check_mark().to_string().green().bold()
            );
            println!(
                "{}",
                "Note: Ensure your API key hasn't expired (CLI keys expire after 8 hours)".dimmed()
            );
            return Ok(());
        } else {
            println!(
                "{} Existing credentials are invalid or expired, proceeding with login...",
                "⚠".yellow()
            );
        }
    }

    // If an API key was provided directly, try to validate it
    if let Some(key) = api_key {
        return validate_and_save_key(
            config,
            server_url.clone(),
            key,
            server_name.clone(),
            server_source,
        )
        .await;
    }

    // In headless environments, prompt for manual API key entry instead of OAuth
    if is_headless() {
        println!(
            "{} Headless environment detected (no display server). Using manual key entry.",
            "ℹ".bright_cyan()
        );
        println!("Create an API key in your server's web UI and paste it below.\n");

        let key = Password::new()
            .with_prompt("Enter API key (starts with 'rico_')")
            .interact()?;

        return validate_and_save_key(config, server_url, key, server_name, server_source).await;
    }

    // Use OAuth with local callback server
    oauth_login_with_callback(config, server_url, server_name, server_source).await?;

    Ok(())
}

/// Resolve the server URL and name for login
fn resolve_login_server(
    config: &Config,
    server_ref: Option<&str>,
) -> Result<(Url, Option<String>, LoginServerSource)> {
    if let Some(ref_str) = server_ref {
        // Check if it's a named server
        if let Some(server_config) = config.servers.get(ref_str) {
            return Ok((
                server_config.url.clone(),
                Some(ref_str.to_string()),
                LoginServerSource::Configured,
            ));
        }

        // Check if it's a URL
        if ref_str.starts_with("http://") || ref_str.starts_with("https://") {
            let url = crate::config::parse_server_url(ref_str)?;
            // Check if we have a server with this URL
            for (name, server_config) in &config.servers {
                if server_config.url == url {
                    return Ok((url, Some(name.clone()), LoginServerSource::Configured));
                }
            }
            // New server by URL - will be added during login
            return Ok((url, None, LoginServerSource::Configured));
        }

        // It's a new server name - prompt for URL? For now, use default URL
        anyhow::bail!(
            "Server '{}' not found. Add it first with: ricochet server add {} <url>",
            ref_str,
            ref_str
        );
    }

    if config_is_untouched_default(config) {
        return Ok((
            crate::config::parse_server_url(HOSTED_TRIAL_SERVER)?,
            Some(HOSTED_TRIAL_PROFILE.to_string()),
            LoginServerSource::HostedTrialFallback,
        ));
    }

    // Use default server
    let server_config = config.resolve_server(None)?;
    let server_name = config.default_server().map(|s| s.to_string());
    Ok((
        server_config.url,
        server_name,
        LoginServerSource::Configured,
    ))
}

fn config_is_untouched_default(config: &Config) -> bool {
    config.default_server() == Some("default")
        && config.servers.len() == 1
        && config.servers.get("default").is_some_and(|server| {
            server.url.as_str() == "http://localhost:3000/" && server.api_key.is_none()
        })
}

async fn oauth_login_with_callback(
    config: &mut Config,
    server: url::Url,
    server_name: Option<String>,
    server_source: LoginServerSource,
) -> Result<()> {
    use axum::{Router, extract::Query, response::Html, routing::get};
    use std::collections::HashMap;
    use tokio::net::TcpListener;

    println!("\n{}", "Starting OAuth authentication...".yellow());

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let callback_url = format!("http://localhost:{}/callback", port);

    // Create shared state for the callback
    let state = Arc::new(Mutex::new(AuthState {
        received_callback: false,
        session_cookie: None,
        error: None,
    }));
    let state_clone = state.clone();

    // Create the callback handler
    // Also handle /callback&api_key=... (server bug workaround)
    let app = Router::new().route(
        "/callback",
        get(move |Query(params): Query<HashMap<String, String>>| {
            let state = state_clone.clone();
            async move {
                let mut auth_state = state.lock().await;

                // Debug: log what we received (abbreviate API key if present)
                if !params.is_empty() {
                    let mut debug_params = params.clone();
                    if let Some(api_key) = debug_params.get_mut("api_key")
                        && api_key.len() > 12
                    {
                        *api_key = format!("{}...", &api_key[..12]);
                    }
                    println!("Received callback with params: {:?}", debug_params);
                }

                if let Some(error) = params.get("error") {
                    auth_state.error = Some(error.clone());
                    auth_state.received_callback = true;
                    Html(auth_ui::create_error_page(error))
                } else if let Some(api_key) = params.get("api_key") {
                    // Server directly provides an API key - best case!
                    auth_state.session_cookie = Some(api_key.clone());
                    auth_state.received_callback = true;
                    Html(auth_ui::create_success_page())
                } else if let Some(session) = params.get("session") {
                    // Server provides a session token
                    auth_state.session_cookie = Some(session.clone());
                    auth_state.received_callback = true;
                    Html(auth_ui::create_session_page())
                } else {
                    // Log all params for debugging
                    println!("Callback params: {:?}", params);
                    auth_state.received_callback = true;
                    // Use a simple complete page from auth_ui
                    Html(auth_ui::create_session_page())
                }
            }
        }),
    );

    // Start the local server
    let server_handle = tokio::spawn(async move { axum::serve(listener, app).await });

    // Build OAuth URL with our callback
    let mut oauth_url = server.clone();
    oauth_url.set_path("/oauth/authorize");
    oauth_url
        .query_pairs_mut()
        .append_pair("redirect_uri", &callback_url)
        .append_pair("response_type", "code")
        .append_pair("client_id", "cli");

    println!("\nOpening browser for authentication...");
    println!("If browser doesn't open, visit:");
    println!("  {}", oauth_url.to_string().bright_cyan().underline());

    // Open browser
    if webbrowser::open(oauth_url.as_str()).is_err() {
        println!("\n{}", "Could not open browser automatically".dimmed());
    }

    // Wait for callback (with timeout)
    println!("\nWaiting for authentication...");
    let timeout = tokio::time::Duration::from_secs(300); // 5 minutes
    let start = tokio::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            server_handle.abort();
            anyhow::bail!("Authentication timeout. Please try again.");
        }

        let auth_state = state.lock().await;
        if auth_state.received_callback {
            if let Some(error) = &auth_state.error {
                server_handle.abort();
                anyhow::bail!("Authentication failed: {}", error);
            }
            break;
        }
        drop(auth_state);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Shutdown the server
    server_handle.abort();

    // Check what we got back
    let auth_state = state.lock().await;
    if let Some(token) = &auth_state.session_cookie {
        // Check if it's an API key (starts with rico_) or session token
        if token.starts_with("rico_") {
            // Direct API key - just save it!
            println!(
                "\n{} Received API key directly from server!",
                symbols::check_mark().to_string().green().bold()
            );
            validate_and_save_key(
                config,
                server.clone(),
                token.clone(),
                server_name.clone(),
                server_source,
            )
            .await?;
        } else {
            // Session token - use it to create API key
            create_api_key_with_session(
                config,
                server.clone(),
                token.clone(),
                server_name.clone(),
                server_source,
            )
            .await?;
        }
    } else {
        // Fall back to manual entry
        println!(
            "\n{}",
            "OAuth callback received but no session token provided".yellow()
        );
        println!("Please create an API key manually in the browser");

        let mut keys_url = server.clone();
        keys_url.set_path("/credentials");
        if webbrowser::open(keys_url.as_str()).is_err() {
            println!("Open: {}", keys_url.as_str().bright_cyan().underline());
        }

        let key = Password::new()
            .with_prompt("Enter API key (starts with 'rico_')")
            .interact()?;

        validate_and_save_key(config, server, key, server_name, server_source).await?;
    }

    Ok(())
}

async fn create_api_key_with_session(
    config: &mut Config,
    server: Url,
    session_token: String,
    server_name: Option<String>,
    server_source: LoginServerSource,
) -> Result<()> {
    println!("\n{}", "Creating API key using session...".dimmed());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut api_key_url = server.clone();
    api_key_url.set_path("/api/v0/api-keys");

    // Calculate expiration time (8 hours from now)
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(8);

    let key_request = CreateApiKeyRequest {
        name: format!(
            "ricochet-cli-{}",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ),
        expires_in_hours: 8,
        expires_at: Some(expires_at.to_rfc3339()), // ISO 8601 format
    };

    let response = client
        .post(api_key_url.as_str())
        .header("Cookie", format!("tower.session={}", session_token))
        .json(&key_request)
        .send()
        .await?;

    if response.status().is_success() {
        let api_key_data: ApiKeyResponse = response.json().await?;

        let name = store_login_credentials(
            config,
            &server,
            api_key_data.key.clone(),
            server_name,
            server_source,
        )?;
        config.save()?;

        println!(
            "\n{} Successfully created and saved API key!",
            symbols::check_mark().to_string().green().bold()
        );
        println!(
            "Server: {} ({})",
            name.bright_cyan(),
            server.as_str().dimmed()
        );

        // Display expiration info
        if let Some(expires_at) = &api_key_data.expires_at {
            if let Ok(expiry_time) = chrono::DateTime::parse_from_rfc3339(expires_at) {
                let now = chrono::Utc::now();
                let duration = expiry_time.signed_duration_since(now);

                if duration.num_seconds() > 0 {
                    let hours = duration.num_hours();
                    let minutes = (duration.num_minutes() % 60).abs();

                    println!(
                        "API key expires in: {} hours {} minutes",
                        hours.to_string().bright_yellow(),
                        minutes.to_string().bright_yellow()
                    );
                }
                println!(
                    "Expires at: {}",
                    expiry_time.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
        } else {
            println!("API key expires in: 8 hours");
        }

        println!(
            "Configuration saved to: {}",
            Config::config_path()?.display()
        );

        // Show the key prefix for verification
        let key_prefix = if api_key_data.key.len() > 12 {
            &api_key_data.key[..12]
        } else {
            &api_key_data.key
        };
        println!("API key: {}...", key_prefix.dimmed());
        Ok(())
    } else {
        anyhow::bail!("Failed to create API key. Session may be invalid or expired.")
    }
}

/// Determine the server name to use for saving credentials
fn determine_server_name(config: &Config, server_url: &Url, server_name: Option<String>) -> String {
    if let Some(name) = server_name {
        return name;
    }

    // Check if we have an existing server with this URL
    for (name, server_config) in &config.servers {
        if server_config.url == *server_url {
            return name.clone();
        }
    }

    // Generate a name based on the hostname
    if let Some(host) = server_url.host_str() {
        if host == "localhost" || host == "127.0.0.1" {
            return "local".to_string();
        }
        // Use first part of hostname
        let parts: Vec<&str> = host.split('.').collect();
        if !parts.is_empty() && parts[0] != "www" {
            return parts[0].to_string();
        }
    }

    "default".to_string()
}

fn store_login_credentials(
    config: &mut Config,
    server: &Url,
    api_key: String,
    server_name: Option<String>,
    server_source: LoginServerSource,
) -> Result<String> {
    let name = determine_server_name(config, server, server_name);
    config.add_server(name.clone(), server.clone(), Some(api_key));
    if server_source == LoginServerSource::HostedTrialFallback {
        config.set_default_server(&name)?;
    }
    Ok(name)
}

/// Detect whether we're running in a headless environment where a browser cannot be opened.
///
/// On Linux, checks for `DISPLAY` (X11) and `WAYLAND_DISPLAY`. If neither is set, the
/// environment is headless. Also checks for `SSH_CLIENT`/`SSH_TTY` as a strong signal.
/// On macOS and Windows, always returns false since a display server is assumed.
fn is_headless() -> bool {
    if cfg!(target_os = "linux") {
        let has_display = std::env::var("DISPLAY").is_ok();
        let has_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
        let is_ssh = std::env::var("SSH_CLIENT").is_ok() || std::env::var("SSH_TTY").is_ok();

        if is_ssh && !has_display && !has_wayland {
            return true;
        }

        !has_display && !has_wayland
    } else {
        false
    }
}

async fn validate_and_save_key(
    config: &mut Config,
    server: url::Url,
    key: String,
    server_name: Option<String>,
    server_source: LoginServerSource,
) -> Result<()> {
    println!("\n{}", "Validating credentials...".dimmed());
    let client = RicochetClient::new_with_key(server.to_string(), key.clone())?;

    match client.validate_key().await {
        Ok(true) => {
            let name =
                store_login_credentials(config, &server, key.clone(), server_name, server_source)?;
            config.save()?;

            println!(
                "\n{} Successfully authenticated!",
                symbols::check_mark().to_string().green().bold()
            );
            println!(
                "Server: {} ({})",
                name.bright_cyan(),
                server.as_str().dimmed()
            );
            println!(
                "Configuration saved to: {}",
                Config::config_path()?.display()
            );

            // Show the key prefix for verification
            let key_prefix = if key.len() > 12 { &key[..12] } else { &key };
            println!("API key: {}...", key_prefix.dimmed());
            Ok(())
        }
        Ok(false) => {
            anyhow::bail!("Invalid API key. Please check and try again.")
        }
        Err(e) => {
            anyhow::bail!("Failed to validate credentials: {}", e)
        }
    }
}

pub fn logout(config: &mut Config, server_ref: Option<&str>) -> Result<()> {
    // Resolve which server to logout from
    let server_name = if let Some(ref_str) = server_ref {
        // Check if it's a named server
        if config.servers.contains_key(ref_str) {
            ref_str.to_string()
        } else if ref_str.starts_with("http://") || ref_str.starts_with("https://") {
            // It's a URL - find matching server
            let url = crate::config::parse_server_url(ref_str)?;
            let mut found_name = None;
            for (name, server_config) in &config.servers {
                if server_config.url == url {
                    found_name = Some(name.clone());
                    break;
                }
            }
            found_name.ok_or_else(|| anyhow::anyhow!("No server found with URL: {}", ref_str))?
        } else {
            anyhow::bail!("Server '{}' not found", ref_str);
        }
    } else {
        // Use default server
        config
            .default_server()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No default server configured"))?
    };

    // Check if the server has an API key
    let server_config = config
        .servers
        .get_mut(&server_name)
        .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", server_name))?;

    if server_config.api_key.is_none() {
        println!(
            "{} Not logged in to server '{}'",
            "⚠".yellow(),
            server_name.bright_cyan()
        );
        return Ok(());
    }

    // Clear the API key
    server_config.api_key = None;
    config.save()?;

    println!(
        "{} Logged out from server '{}'",
        symbols::check_mark().to_string().green().bold(),
        server_name.bright_cyan()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use std::collections::HashMap;

    #[test]
    fn bare_login_uses_hosted_trial_for_untouched_config() {
        let config = Config::default();

        let (url, profile, source) = resolve_login_server(&config, None).expect("server resolves");

        assert_eq!(url.as_str(), "https://try.ricochet.rs/");
        assert_eq!(profile.as_deref(), Some("try"));
        assert_eq!(source, LoginServerSource::HostedTrialFallback);
    }

    #[test]
    fn explicit_server_preserves_untouched_local_default() {
        let config = Config::default();

        let (url, profile, source) =
            resolve_login_server(&config, Some("default")).expect("server resolves");

        assert_eq!(url.as_str(), "http://localhost:3000/");
        assert_eq!(profile.as_deref(), Some("default"));
        assert_eq!(source, LoginServerSource::Configured);
    }

    #[test]
    fn bare_login_preserves_configured_default() {
        let mut servers = HashMap::new();
        servers.insert(
            "production".to_string(),
            ServerConfig {
                url: Url::parse("https://ricochet.example.com").expect("valid URL"),
                api_key: None,
            },
        );
        let config = Config {
            server: None,
            api_key: None,
            servers,
            default_server: Some("production".to_string()),
            default_format: Some("table".to_string()),
            skip_update_check: None,
        };

        let (url, profile, source) = resolve_login_server(&config, None).expect("server resolves");

        assert_eq!(url.as_str(), "https://ricochet.example.com/");
        assert_eq!(profile.as_deref(), Some("production"));
        assert_eq!(source, LoginServerSource::Configured);
    }

    #[test]
    fn hosted_trial_credentials_make_try_profile_default() {
        let mut config = Config::default();
        let url = Url::parse(HOSTED_TRIAL_SERVER).expect("valid URL");

        let name = store_login_credentials(
            &mut config,
            &url,
            "rico_test".to_string(),
            Some(HOSTED_TRIAL_PROFILE.to_string()),
            LoginServerSource::HostedTrialFallback,
        )
        .expect("credentials stored");

        assert_eq!(name, "try");
        assert_eq!(config.default_server(), Some("try"));
        assert_eq!(
            config
                .servers
                .get("try")
                .and_then(|server| server.api_key.as_deref()),
            Some("rico_test")
        );
    }
}
