use anyhow::{Result, bail};
use std::collections::HashMap;
use std::path::Path;

/// Parse `.env` / `.Renviron` style `KEY=VALUE` lines into a map.
pub fn parse_dotenv(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let mut key = key.trim();
        if let Some(stripped) = key.strip_prefix("export ") {
            key = stripped.trim();
        }
        if !is_valid_key(key) {
            continue;
        }
        let value = strip_quotes(value.trim());
        map.insert(key.to_string(), value.to_string());
    }
    map
}

fn is_valid_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn strip_quotes(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && (bytes[0] == b'"' || bytes[0] == b'\'')
        && bytes[bytes.len() - 1] == bytes[0]
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

/// Resolve `--env` entries into concrete key/value pairs.
///
/// `KEY=VALUE` is taken literally. `KEY` alone is looked up in `.env`, then
/// `.Renviron` (both in `dir`), then the process environment; an unresolved
/// key is an error.
pub fn resolve_env_vars(entries: &[String], dir: &Path) -> Result<HashMap<String, String>> {
    let mut dotfiles: Option<HashMap<String, String>> = None;
    let mut result = HashMap::new();

    for entry in entries {
        if let Some((key, value)) = entry.split_once('=') {
            result.insert(key.to_string(), value.to_string());
        } else {
            let key = entry.as_str();
            let files = dotfiles.get_or_insert_with(|| load_dotfiles(dir));
            let value = files.get(key).cloned().or_else(|| std::env::var(key).ok());
            match value {
                Some(v) => {
                    result.insert(key.to_string(), v);
                }
                None => bail!(
                    "env var `{key}` not found in .env, .Renviron, or the environment; pass it explicitly as `--env {key}=value`"
                ),
            }
        }
    }
    Ok(result)
}

/// Load `.Renviron` then `.env` so that `.env` values take precedence.
/// Missing files are treated as empty.
fn load_dotfiles(dir: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for name in [".Renviron", ".env"] {
        if let Ok(content) = std::fs::read_to_string(dir.join(name)) {
            for (k, v) in parse_dotenv(&content) {
                map.insert(k, v);
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parses_basic_pairs_comments_and_quotes() {
        let content = "\
# a comment

FOO=bar
export BAZ = qux
QUOTED=\"hello world\"
SINGLE='single'
NOT A KEY=ignored
123BAD=nope
";
        let map = parse_dotenv(content);
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(map.get("BAZ"), Some(&"qux".to_string()));
        assert_eq!(map.get("QUOTED"), Some(&"hello world".to_string()));
        assert_eq!(map.get("SINGLE"), Some(&"single".to_string()));
        assert!(!map.contains_key("123BAD"));
        assert_eq!(map.len(), 4);
    }

    #[test]
    fn key_value_entry_used_literally() {
        let dir = TempDir::new().unwrap();
        let entries = vec![
            "A=1".to_string(),
            "B=with=equals".to_string(),
            "C=".to_string(),
        ];
        let map = resolve_env_vars(&entries, dir.path()).unwrap();
        assert_eq!(map.get("A"), Some(&"1".to_string()));
        assert_eq!(map.get("B"), Some(&"with=equals".to_string()));
        assert_eq!(map.get("C"), Some(&"".to_string()));
    }

    #[test]
    fn key_only_resolves_env_then_renviron_then_process() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".env"),
            "FROM_ENV=env_value\nSHARED=env_wins\n",
        )
        .unwrap();
        fs::write(
            dir.path().join(".Renviron"),
            "FROM_RENV=renv_value\nSHARED=renv_loses\n",
        )
        .unwrap();
        // SAFETY: single-threaded test; key is unique to this test.
        unsafe { std::env::set_var("RICO_TEST_PROC_ONLY", "proc_value") };

        let entries = vec![
            "FROM_ENV".to_string(),
            "FROM_RENV".to_string(),
            "SHARED".to_string(),
            "RICO_TEST_PROC_ONLY".to_string(),
        ];
        let map = resolve_env_vars(&entries, dir.path()).unwrap();
        assert_eq!(map.get("FROM_ENV"), Some(&"env_value".to_string()));
        assert_eq!(map.get("FROM_RENV"), Some(&"renv_value".to_string()));
        assert_eq!(map.get("SHARED"), Some(&"env_wins".to_string()));
        assert_eq!(
            map.get("RICO_TEST_PROC_ONLY"),
            Some(&"proc_value".to_string())
        );

        unsafe { std::env::remove_var("RICO_TEST_PROC_ONLY") };
    }

    #[test]
    fn unresolved_key_errors_with_hint() {
        let dir = TempDir::new().unwrap();
        let entries = vec!["DEFINITELY_MISSING_KEY_XYZ".to_string()];
        let err = resolve_env_vars(&entries, dir.path())
            .unwrap_err()
            .to_string();
        assert!(err.contains("DEFINITELY_MISSING_KEY_XYZ"));
        assert!(err.contains("--env DEFINITELY_MISSING_KEY_XYZ=value"));
    }

    #[test]
    fn later_flag_overrides_earlier() {
        let dir = TempDir::new().unwrap();
        let entries = vec!["K=first".to_string(), "K=second".to_string()];
        let map = resolve_env_vars(&entries, dir.path()).unwrap();
        assert_eq!(map.get("K"), Some(&"second".to_string()));
    }
}
