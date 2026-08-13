pub mod deployment;
pub mod env_vars;
pub mod invoke;
pub mod schedule;
pub mod settings;
pub mod toml;

use anyhow::{Context, Result};
use colored::Colorize;
use ricochet_core::content::ContentItem;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

/// Load the local `_ricochet.toml`, returning its content ID and parsed item.
pub fn load_local(path: Option<&Path>) -> Result<(String, ContentItem)> {
    let toml_path = path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("_ricochet.toml"));
    if toml_path.is_dir() {
        anyhow::bail!(
            "{} {} is a directory. Point --path at the `_ricochet.toml` file itself.",
            "⚠".yellow(),
            toml_path.display()
        );
    }
    if !toml_path.exists() {
        anyhow::bail!(
            "{} No `_ricochet.toml` found at {}. Provide one with --path.",
            "⚠".yellow(),
            toml_path.display()
        );
    }
    let toml = read_to_string(&toml_path).context("reading local _ricochet.toml")?;
    let item = ContentItem::from_toml(&toml).context("parsing local _ricochet.toml")?;
    let Some(id) = item.content.id.clone() else {
        anyhow::bail!("Local _ricochet.toml has no item ID. Deploy the item first.");
    };
    Ok((id, item))
}

/// Take the content ID as given, or read it from the local `_ricochet.toml`.
pub fn resolve_id(id: Option<&str>, path: Option<&Path>) -> Result<String> {
    match id {
        Some(id) => Ok(id.to_string()),
        None => load_local(path)
            .map(|(id, _)| id)
            .context("provide either an item ID or a path to a `_ricochet.toml` file"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const LOCAL_TOML: &str = r#"[content]
id = "01KE52BY41EQ7NE89K7Z5MMZ84"
name = "local-app"
entrypoint = "app.R"
access_type = "external"
content_type = "shiny"

[language]
name = "r"
packages = "renv.lock"
"#;

    #[test]
    fn reads_the_id_from_the_given_path() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("_ricochet.toml");
        fs::write(&toml_path, LOCAL_TOML).unwrap();

        let (id, item) = load_local(Some(&toml_path)).unwrap();
        assert_eq!(id, "01KE52BY41EQ7NE89K7Z5MMZ84");
        assert_eq!(item.content.name, "local-app");
        assert_eq!(resolve_id(None, Some(&toml_path)).unwrap(), id);
    }

    #[test]
    fn directory_path_says_so() {
        let dir = TempDir::new().unwrap();
        let err = load_local(Some(dir.path())).unwrap_err().to_string();
        assert!(err.contains("is a directory"), "{err}");
    }

    #[test]
    fn missing_file_names_the_path() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("_ricochet.toml");
        let err = load_local(Some(&toml_path)).unwrap_err().to_string();
        assert!(err.contains(&toml_path.display().to_string()), "{err}");
    }

    #[test]
    fn explicit_id_never_touches_the_filesystem() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nope.toml");
        assert_eq!(resolve_id(Some("ABC"), Some(&missing)).unwrap(), "ABC");
    }
}
