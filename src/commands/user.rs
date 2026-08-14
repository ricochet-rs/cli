use crate::{OutputFormat, client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};
use ricochet_core::config::git::{GitCredential, GitProtocol};

fn format_credentials(
    server_url: &str,
    credentials: &[GitCredential],
    format: OutputFormat,
) -> Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(credentials)?),
        OutputFormat::Yaml => Ok(serde_yaml::to_string(credentials)?),
        OutputFormat::Table => {
            let mut output = server_url.italic().dimmed().to_string();

            if credentials.is_empty() {
                output.push_str(&format!("\n{}", "No credentials found.".yellow()));
                return Ok(output);
            }

            let mut table = Table::new();
            table.load_style(UTF8_FULL);
            table.set_header(vec!["ID", "Name", "Type", "User ID"]);
            for credential in credentials {
                table.add_row(vec![
                    credential.id.as_str(),
                    credential.name.as_str(),
                    &credential.protocol.to_string(),
                    credential.user_id.as_str(),
                ]);
            }

            output.push_str(&format!("\n{table}\n\n{} credential(s)", credentials.len()));
            Ok(output)
        }
    }
}

pub async fn list_credentials(
    config: &Config,
    server_ref: Option<&str>,
    user_id: Option<&str>,
    protocol: Option<GitProtocol>,
    format: OutputFormat,
) -> Result<()> {
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let credentials = client.list_credentials(user_id, protocol).await?;

    println!(
        "{}",
        format_credentials(server_config.url.as_str(), &credentials, format)?
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credential(protocol: GitProtocol) -> GitCredential {
        GitCredential {
            id: "credential-id".to_string(),
            user_id: "user-id".to_string(),
            name: "deploy-key".to_string(),
            protocol,
        }
    }

    #[test]
    fn formats_json_output() -> Result<()> {
        let output = format_credentials(
            "https://example.com",
            &[credential(GitProtocol::Ssh)],
            OutputFormat::Json,
        )?;
        let parsed: Vec<GitCredential> = serde_json::from_str(&output)?;

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "credential-id");
        assert_eq!(parsed[0].protocol, GitProtocol::Ssh);
        Ok(())
    }

    #[test]
    fn formats_yaml_output() -> Result<()> {
        let output = format_credentials(
            "https://example.com",
            &[credential(GitProtocol::Https)],
            OutputFormat::Yaml,
        )?;
        let parsed: Vec<GitCredential> = serde_yaml::from_str(&output)?;

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].protocol, GitProtocol::Https);
        Ok(())
    }

    #[test]
    fn formats_table_output() -> Result<()> {
        let output = format_credentials(
            "https://example.com",
            &[credential(GitProtocol::Ssh)],
            OutputFormat::Table,
        )?;

        assert!(output.contains("https://example.com"));
        assert!(output.contains("credential-id"));
        assert!(output.contains("deploy-key"));
        assert!(output.contains("ssh"));
        assert!(output.contains("user-id"));
        assert!(output.contains("1 credential(s)"));
        Ok(())
    }

    #[test]
    fn formats_empty_table_output() -> Result<()> {
        let output = format_credentials("https://example.com", &[], OutputFormat::Table)?;

        assert!(output.contains("No credentials found."));
        Ok(())
    }
}
