//! Tools `ricochet mcp` runs itself, against a project directory on the local disk.
//!
//! Each runs the CLI's own `deploy` as a child process, so that deploy's stdout and prompts never
//! reach the MCP channel, which owns this process's stdout.

use std::{path::PathBuf, time::Duration};

use rmcp::schemars::JsonSchema;
use serde::Deserialize;

use crate::config::ServerConfig;

/// How long listing a bundle may take, since it reads only the local disk.
///
/// A deploy has no limit, since an upload takes as long as the bundle and the link need, and every
/// request it makes is bounded by the HTTP client's own timeout.
pub(crate) const PREVIEW_TIMEOUT: Duration = Duration::from_secs(120);

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
    pub(crate) path: PathBuf,
}

/// The server a local tool deploys to.
pub(crate) enum DeployTarget {
    /// The server `ricochet mcp` resolved at startup, pinned by URL and key.
    Server(ServerConfig),
    /// No server resolved at startup, so the child resolves `--server`, if any, itself and reports why it fails.
    Unresolved(Option<String>),
}

impl LocalTool {
    pub(crate) const ALL: [Self; 2] = [Self::PreviewBundle, Self::Deploy];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::PreviewBundle => "preview_bundle",
            Self::Deploy => "deploy",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.name() == name)
    }
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
}
