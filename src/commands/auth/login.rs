use super::auth_ui;
use crate::{client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;
use dialoguer::{Confirm, Input, Password};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use unicode_icons::icons::symbols;

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

pub async fn login(config: &mut Config, api_key: Option<String>) -> Result<()> {
    println!("🔐 Authenticating against ricochet server\n");

    // First check if API key is set via environment variable
    if let Ok(env_key) = std::env::var("RICOCHET_API_KEY")
        && let Ok(server) = std::env::var("RICOCHET_SERVER")
    {
        println!("Using API key from environment variable");

        // Validate the key
        let client = RicochetClient::new_with_key(server.clone(), env_key.clone())?;
        match client.validate_key().await {
            Ok(true) => {
                println!(
                    "{} Already authenticated via environment variables",
                    symbols::check_mark().to_string().green().bold()
                );
                // Note: We can't check expiration without server returning it
                println!(
                    "{}",
                    "Note: Ensure your API key hasn't expired (CLI keys expire after 8 hours)"
                        .dimmed()
                );
                return Ok(());
            }
            Ok(false) | Err(_) => {
                println!(
                    "{} Environment API key is invalid or expired, proceeding with login...",
                    "⚠".yellow()
                );
            }
        }
    }

    // Get server URL
    let server = if let Ok(server) = std::env::var("RICOCHET_SERVER") {
        println!(
            "{}",
            format!(
                "Using server info from $RICOCHET_SERVER env var: {}",
                server
            )
            .dimmed()
        );
        server
    } else if let Some(server) = config.server.clone() {
        let use_existing = Confirm::new()
            .with_prompt(format!("Use server: {}?", server))
            .default(true)
            .interact()?;

        if use_existing {
            server
        } else {
            Input::new()
                .with_prompt("Server URL")
                .default("http://localhost:3000".to_string())
                .interact_text()?
        }
    } else {
        Input::new()
            .with_prompt("Server URL")
            .default("http://localhost:3000".to_string())
            .interact_text()?
    };

    // If an API key was provided directly, use it
    if let Some(key) = api_key {
        println!("\n{}", "Validating provided API key...".dimmed());
        let client = RicochetClient::new_with_key(server.clone(), key.clone())?;

        match client.validate_key().await {
            Ok(true) => {
                config.server = Some(server);
                config.api_key = Some(key);
                config.save()?;

                println!(
                    "\n{} Successfully authenticated!",
                    symbols::check_mark().to_string().green().bold()
                );
                println!(
                    "Configuration saved to: {}",
                    Config::config_path()?.display()
                );
                return Ok(());
            }
            Ok(false) => {
                anyhow::bail!("Invalid API key")
            }
            Err(e) => {
                anyhow::bail!("Failed to validate credentials: {}", e)
            }
        }
    }

    // Always use OAuth with local callback server
    oauth_login_with_callback(config, server).await?;

    Ok(())
}

async fn oauth_login_with_callback(config: &mut Config, server: String) -> Result<()> {
    use axum::{Router, extract::Query, response::Html, routing::get};
    use std::collections::HashMap;
    use tokio::net::TcpListener;

    println!("\n{}", "Starting OAuth authentication...".yellow());

    // First, try to check if there's an existing valid API key
    if let Some(existing_key) = &config.api_key {
        println!("{}", "Checking existing credentials...".dimmed());
        let client = RicochetClient::new_with_key(server.clone(), existing_key.clone())?;
        if client.validate_key().await.unwrap_or(false) {
            println!(
                "{} Already authenticated with valid API key",
                symbols::check_mark().to_string().green().bold()
            );
            return Ok(());
        }
        println!(
            "{}",
            "Existing API key is invalid, proceeding with OAuth...".yellow()
        );
    }

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
    // Note: Server should handle the redirect_uri properly and use ? for first param
    let oauth_url = format!(
        "{}/oauth/authorize?redirect_uri={}&response_type=code&client_id=cli",
        server,
        urlencoding::encode(&callback_url)
    );

    println!("\nOpening browser for authentication...");
    println!("If browser doesn't open, visit:");
    println!("  {}", oauth_url.bright_cyan().underline());

    // Open browser
    if webbrowser::open(&oauth_url).is_err() {
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
            validate_and_save_key(config, server.clone(), token.clone()).await?;
        } else {
            // Session token - use it to create API key
            create_api_key_with_session(config, server.clone(), token.clone()).await?;
        }
    } else {
        // Fall back to manual entry
        println!(
            "\n{}",
            "OAuth callback received but no session token provided".yellow()
        );
        println!("Please create an API key manually in the browser");

        let dashboard_url = format!("{}/dashboard/api-keys", server);
        if webbrowser::open(&dashboard_url).is_err() {
            println!("Open: {}", dashboard_url.bright_cyan().underline());
        }

        let key = Password::new()
            .with_prompt("Enter API key (starts with 'rico_')")
            .interact()?;

        validate_and_save_key(config, server, key).await?;
    }

    Ok(())
}

async fn create_api_key_with_session(
    config: &mut Config,
    server: String,
    session_token: String,
) -> Result<()> {
    println!("\n{}", "Creating API key using session...".dimmed());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let api_key_url = format!("{}/api/v0/api-keys", server);

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
        .post(&api_key_url)
        .header("Cookie", format!("tower.session={}", session_token))
        .json(&key_request)
        .send()
        .await?;

    if response.status().is_success() {
        let api_key_data: ApiKeyResponse = response.json().await?;

        // Save the API key
        config.server = Some(server.clone());
        config.api_key = Some(api_key_data.key.clone());
        config.save()?;

        println!(
            "\n{} Successfully created and saved API key!",
            symbols::check_mark().to_string().green().bold()
        );

        // Display expiration info
        if let Some(expires_at) = &api_key_data.expires_at {
            if let Ok(expiry_time) = chrono::DateTime::parse_from_rfc3339(expires_at) {
                let now = chrono::Utc::now();
                let duration = expiry_time.signed_duration_since(now);
                let hours = duration.num_hours();
                let minutes = duration.num_minutes() % 60;

                println!(
                    "API key expires in: {} hours {} minutes",
                    hours.to_string().bright_yellow(),
                    minutes.to_string().bright_yellow()
                );
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

async fn validate_and_save_key(config: &mut Config, server: String, key: String) -> Result<()> {
    println!("\n{}", "Validating credentials...".dimmed());
    let client = RicochetClient::new_with_key(server.clone(), key.clone())?;

    match client.validate_key().await {
        Ok(true) => {
            config.server = Some(server);
            config.api_key = Some(key.clone());
            config.save()?;

            println!(
                "\n{} Successfully authenticated!",
                symbols::check_mark().to_string().green().bold()
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

pub fn logout(config: &mut Config) -> Result<()> {
    if config.api_key.is_none() {
        println!("{}", "Not currently logged in".yellow());
        return Ok(());
    }

    config.api_key = None;
    config.save()?;

    println!(
        "{} Logged out successfully",
        symbols::check_mark().to_string().green().bold()
    );
    Ok(())
}
