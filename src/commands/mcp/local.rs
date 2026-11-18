use std::{path::PathBuf, time::Duration};

use rmcp::schemars::JsonSchema;
use serde::Deserialize;

use crate::config::ServerConfig;

pub(crate) const PREVIEW_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Copy)]
pub(crate) enum LocalTool {
    PreviewBundle,
    Deploy,
}

#[derive(Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct LocalToolArgs {
    /// Absolute path containing the `_ricochet.toml`.
    pub(crate) path: PathBuf,
}

pub(crate) enum DeployTarget {
    Server(ServerConfig),
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
