//! Tools `ricochet mcp` runs itself, against a project directory on the local disk.
//!
//! Each runs the CLI's own `deploy` as a child process, so that deploy's stdout and prompts never
//! reach the MCP channel, which owns this process's stdout.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use rmcp::handler::server::common::schema_for_type;
use rmcp::model::{CallToolResult, ContentBlock, Tool, ToolAnnotations};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;

/// How long listing a bundle may take, since it reads only the local disk.
///
/// A deploy has no limit, since an upload takes as long as the bundle and the link need, and every
/// request it makes is bounded by the HTTP client's own timeout.
const PREVIEW_TIMEOUT: Duration = Duration::from_secs(120);

/// A tool that acts on a local project directory.
#[derive(Clone, Copy)]
pub(crate) enum LocalTool {
    PreviewBundle,
    Deploy,
}

/// The arguments every local tool takes.
#[derive(Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct LocalToolArgs {
    /// The absolute path of the directory holding `_ricochet.toml`.
    path: PathBuf,
}

/// The `event` of one line of `deploy -F json`, naming only the two that end the stream.
#[derive(Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum DeployEnd {
    Done,
    Error,
    #[serde(other)]
    Progress,
}

impl LocalTool {
    pub(crate) const ALL: [Self; 2] = [Self::PreviewBundle, Self::Deploy];

    fn name(self) -> &'static str {
        match self {
            Self::PreviewBundle => "preview_bundle",
            Self::Deploy => "deploy",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.name() == name)
    }

    pub(crate) fn definition(self) -> Tool {
        let (description, annotations) = match self {
            Self::PreviewBundle => (
                "List the files a deploy of a local project directory would bundle and the ones it would leave out, without contacting a server. `path` is the absolute path of the directory holding `_ricochet.toml`.",
                ToolAnnotations::new().read_only(true),
            ),
            Self::Deploy => (
                "Deploy a local project directory to the ricochet server and wait until the server accepts the bundle. `path` is the absolute path of the directory holding `_ricochet.toml`.",
                ToolAnnotations::new()
                    .read_only(false)
                    .destructive(true)
                    .open_world(true),
            ),
        };
        Tool::new(self.name(), description, schema_for_type::<LocalToolArgs>())
            .with_annotations(annotations)
    }

    /// Run the tool on `args.path`, reporting any failure as a tool error the agent can read.
    pub(crate) async fn call(
        self,
        args: LocalToolArgs,
        server_ref: Option<&str>,
    ) -> CallToolResult {
        match self.run(&args.path, server_ref).await {
            Ok(Outcome::Success(text)) => CallToolResult::success(vec![ContentBlock::text(text)]),
            Ok(Outcome::Failure(text)) => CallToolResult::error(vec![ContentBlock::text(text)]),
            Err(error) => CallToolResult::error(vec![ContentBlock::text(format!("{error:#}"))]),
        }
    }

    async fn run(self, path: &Path, server_ref: Option<&str>) -> anyhow::Result<Outcome> {
        // A relative path would resolve against wherever the editor started this server, not the project.
        if !path.is_absolute() {
            anyhow::bail!(
                "`path` must be an absolute path, but {} is relative",
                path.display()
            );
        }
        let metadata = tokio::fs::metadata(path)
            .await
            .with_context(|| format!("cannot read {}", path.display()))?;
        if !metadata.is_dir() {
            anyhow::bail!("{} is not a directory", path.display());
        }

        let exe = std::env::current_exe().context("cannot locate the ricochet binary")?;
        let mut command = tokio::process::Command::new(exe);
        if let Some(server_ref) = server_ref {
            command.args(["--server", server_ref]);
        }
        command.arg("deploy").arg(path).args(["-F", "json"]);
        if let Self::PreviewBundle = self {
            command.arg("--dry-run");
        }
        command
            .stdin(Stdio::null())
            .env("RICOCHET_NON_INTERACTIVE", "1")
            .env("RICOCHET_NO_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1")
            .kill_on_drop(true);

        let output = match self {
            Self::PreviewBundle => tokio::time::timeout(PREVIEW_TIMEOUT, command.output())
                .await
                .with_context(|| {
                    format!(
                        "listing the bundle of {} took longer than {PREVIEW_TIMEOUT:?}",
                        path.display()
                    )
                })?,
            Self::Deploy => command.output().await,
        }
        .context("cannot run ricochet deploy")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let failure = || {
            let stderr = stderr.trim();
            let reason = stderr.strip_prefix("Error: ").unwrap_or(stderr);
            Outcome::Failure(match (output.status.success(), reason.is_empty()) {
                (true, true) => "ricochet deploy exited without a result".to_owned(),
                (true, false) => format!("ricochet deploy exited without a result: {reason}"),
                (false, true) => format!("ricochet deploy failed: {}", output.status),
                (false, false) => format!("ricochet deploy failed: {reason}"),
            })
        };

        Ok(match self {
            Self::PreviewBundle if output.status.success() => Outcome::Success(stdout.into_owned()),
            Self::PreviewBundle => failure(),
            Self::Deploy => stdout
                .lines()
                .rev()
                .find_map(|line| match serde_json::from_str(line) {
                    Ok(DeployEnd::Done) => Some(Outcome::Success(line.to_owned())),
                    Ok(DeployEnd::Error) => Some(Outcome::Failure(line.to_owned())),
                    Ok(DeployEnd::Progress) | Err(_) => None,
                })
                .unwrap_or_else(failure),
        })
    }
}

/// What a finished child process reported, as the text of the tool's answer.
enum Outcome {
    Success(String),
    Failure(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_is_found_by_its_name() {
        for tool in LocalTool::ALL {
            assert!(matches!(
                (tool, LocalTool::from_name(tool.name())),
                (LocalTool::PreviewBundle, Some(LocalTool::PreviewBundle))
                    | (LocalTool::Deploy, Some(LocalTool::Deploy))
            ));
        }
        assert!(LocalTool::from_name("list_items").is_none());
    }

    #[test]
    fn only_deploy_is_marked_as_changing_the_server() {
        let preview = LocalTool::PreviewBundle.definition().annotations;
        assert_eq!(preview.and_then(|a| a.read_only_hint), Some(true));

        let deploy = LocalTool::Deploy
            .definition()
            .annotations
            .expect("deploy is annotated");
        assert_eq!(deploy.read_only_hint, Some(false));
        assert_eq!(deploy.destructive_hint, Some(true));
        assert_eq!(deploy.open_world_hint, Some(true));
    }

    #[test]
    fn the_schema_requires_a_path() {
        let schema = LocalTool::Deploy.definition().input_schema;
        assert_eq!(schema.get("required"), Some(&serde_json::json!(["path"])));
    }
}
