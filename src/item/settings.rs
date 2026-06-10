use anyhow::Result;
use ricochet_core::content::ContentItem;
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, PartialEq)]
pub struct FieldChange {
    pub field: String,
    pub from: String,
    pub to: String,
}

fn opt_str(o: &Option<String>) -> Value {
    match o {
        Some(s) => Value::String(s.clone()),
        None => Value::Null,
    }
}

fn opt_path(o: &Option<std::path::PathBuf>) -> Value {
    match o {
        Some(p) => Value::String(p.to_string_lossy().into_owned()),
        None => Value::Null,
    }
}

fn render(v: &Value) -> String {
    match v {
        Value::Null => "(unset)".to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

struct PatchBuilder {
    changes: Vec<FieldChange>,
    patch: Map<String, Value>,
}

impl PatchBuilder {
    fn new() -> Self {
        Self {
            changes: Vec::new(),
            patch: Map::new(),
        }
    }

    /// Diff a single field within a group; always considered.
    fn field(&mut self, group: &str, name: &str, remote: Value, local: Value) {
        if remote != local {
            self.changes.push(FieldChange {
                field: format!("{group}.{name}"),
                from: render(&remote),
                to: render(&local),
            });
            self.patch
                .entry(group.to_string())
                .or_insert_with(|| Value::Object(Map::new()))
                .as_object_mut()
                .expect("group entry is always an object")
                .insert(name.to_string(), local);
        }
    }

    /// Diff a single optional field; skipped entirely when unset locally so we
    /// never push a `null` the user did not ask for (the endpoint ignores nulls).
    fn field_opt(&mut self, group: &str, name: &str, remote: Value, local: Value) {
        if local.is_null() {
            return;
        }
        self.field(group, name, remote, local);
    }

    /// Diff a whole group value (used for `packages`).
    fn whole(&mut self, group: &str, remote: Value, local: Value) {
        if remote != local {
            self.changes.push(FieldChange {
                field: group.to_string(),
                from: "(changed)".to_string(),
                to: "(changed)".to_string(),
            });
            self.patch.insert(group.to_string(), local);
        }
    }

    fn finish(self) -> (Vec<FieldChange>, Value) {
        (self.changes, Value::Object(self.patch))
    }
}

/// Compute the patchable difference between the remote and local content items.
/// Returns a human-readable change list and the JSON body for the settings PATCH.
pub fn compute_patch(
    remote: &ContentItem,
    local: &ContentItem,
) -> Result<(Vec<FieldChange>, Value)> {
    let mut b = PatchBuilder::new();

    // content — required scalars are always compared
    b.field(
        "content",
        "name",
        Value::String(remote.content.name.clone()),
        Value::String(local.content.name.clone()),
    );
    b.field(
        "content",
        "entrypoint",
        Value::String(remote.content.entrypoint.to_string_lossy().into_owned()),
        Value::String(local.content.entrypoint.to_string_lossy().into_owned()),
    );
    b.field(
        "content",
        "access_type",
        serde_json::to_value(&remote.content.access_type)?,
        serde_json::to_value(&local.content.access_type)?,
    );
    b.field(
        "content",
        "content_type",
        serde_json::to_value(&remote.content.content_type)?,
        serde_json::to_value(&local.content.content_type)?,
    );

    // content — optional fields only when set locally
    if local.content.slug.is_some() {
        b.field(
            "content",
            "slug",
            opt_str(&remote.content.slug),
            opt_str(&local.content.slug),
        );
    }
    if local.content.summary.is_some() {
        b.field(
            "content",
            "summary",
            opt_str(&remote.content.summary),
            opt_str(&local.content.summary),
        );
    }
    if local.content.thumbnail.is_some() {
        b.field(
            "content",
            "thumbnail",
            opt_path(&remote.content.thumbnail),
            opt_path(&local.content.thumbnail),
        );
    }
    if local.content.tags.is_some() {
        b.field(
            "content",
            "tags",
            serde_json::to_value(&remote.content.tags)?,
            serde_json::to_value(&local.content.tags)?,
        );
    }
    if local.content.exec_env.is_some() {
        b.field(
            "content",
            "exec_env",
            opt_str(&remote.content.exec_env),
            opt_str(&local.content.exec_env),
        );
    }

    // serve — only when the local TOML declares a [serve] block
    if let Some(local_serve) = &local.serve {
        let remote_serve = remote.serve.clone().unwrap_or_default();
        b.field(
            "serve",
            "min_instances",
            json!(remote_serve.min_instances),
            json!(local_serve.min_instances),
        );
        b.field(
            "serve",
            "max_instances",
            json!(remote_serve.max_instances),
            json!(local_serve.max_instances),
        );
        b.field(
            "serve",
            "spawn_threshold",
            json!(remote_serve.spawn_threshold),
            json!(local_serve.spawn_threshold),
        );
        b.field(
            "serve",
            "max_connections",
            json!(remote_serve.max_connections),
            json!(local_serve.max_connections),
        );
        b.field_opt(
            "serve",
            "max_connection_age",
            json!(remote_serve.max_connection_age),
            json!(local_serve.max_connection_age),
        );
        b.field_opt(
            "serve",
            "inactive_timeout",
            json!(remote_serve.inactive_timeout),
            json!(local_serve.inactive_timeout),
        );
        b.field_opt(
            "serve",
            "connection_timeout",
            json!(remote_serve.connection_timeout),
            json!(local_serve.connection_timeout),
        );

        // deployment — maps to [serve.k8s]; only when declared locally
        if let Some(local_k8s) = &local_serve.k8s {
            let remote_k8s = remote_serve.k8s.clone().unwrap_or_default();
            b.field(
                "deployment",
                "strategy",
                serde_json::to_value(remote_k8s.strategy)?,
                serde_json::to_value(local_k8s.strategy)?,
            );
            b.field_opt(
                "deployment",
                "max_surge",
                opt_str(&remote_k8s.max_surge),
                opt_str(&local_k8s.max_surge),
            );
            b.field_opt(
                "deployment",
                "max_unavailable",
                opt_str(&remote_k8s.max_unavailable),
                opt_str(&local_k8s.max_unavailable),
            );
            b.field_opt(
                "deployment",
                "config",
                opt_str(&remote_k8s.config),
                opt_str(&local_k8s.config),
            );
        }
    }

    // resources — only when declared locally
    if let Some(local_res) = &local.resources {
        let remote_res = remote.resources.clone().unwrap_or_default();
        b.field_opt(
            "resources",
            "cpu_request",
            opt_str(&remote_res.cpu_request),
            opt_str(&local_res.cpu_request),
        );
        b.field_opt(
            "resources",
            "cpu_limit",
            opt_str(&remote_res.cpu_limit),
            opt_str(&local_res.cpu_limit),
        );
        b.field_opt(
            "resources",
            "memory_request",
            opt_str(&remote_res.memory_request),
            opt_str(&local_res.memory_request),
        );
        b.field_opt(
            "resources",
            "memory_limit",
            opt_str(&remote_res.memory_limit),
            opt_str(&local_res.memory_limit),
        );
    }

    // static — only when declared locally
    if let Some(local_static) = &local.static_ {
        let remote_static = remote.static_.clone().unwrap_or_default();
        b.field_opt(
            "static",
            "index",
            opt_str(&remote_static.index),
            opt_str(&local_static.index),
        );
        b.field_opt(
            "static",
            "output_dir",
            opt_str(&remote_static.output_dir),
            opt_str(&local_static.output_dir),
        );
    }

    // packages — only when declared locally; sent as a whole value
    if let Some(local_pkgs) = &local.repositories {
        b.whole(
            "packages",
            serde_json::to_value(&remote.repositories)?,
            serde_json::to_value(local_pkgs)?,
        );
    }

    Ok(b.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r#"
[content]
id = "01KE52BY41EQ7NE89K7Z5MMZ84"
name = "example-app"
entrypoint = "app.R"
access_type = "private"
content_type = "shiny"

[language]
name = "r"
packages = "renv.lock"

[serve]
min_instances = 0
max_instances = 5
spawn_threshold = 80
max_connections = 10
"#;

    const BASE_NO_SERVE: &str = r#"
[content]
id = "01KE52BY41EQ7NE89K7Z5MMZ84"
name = "example-app"
entrypoint = "app.R"
access_type = "private"
content_type = "shiny"

[language]
name = "r"
packages = "renv.lock"
"#;

    fn parse(toml: &str) -> ContentItem {
        ContentItem::from_toml(toml).unwrap()
    }

    #[test]
    fn no_changes_when_identical() {
        let item = parse(BASE);
        let (changes, patch) = compute_patch(&item, &item).unwrap();
        assert!(changes.is_empty());
        assert_eq!(patch, json!({}));
    }

    #[test]
    fn detects_serve_change() {
        let remote = parse(BASE);
        let local = parse(&BASE.replace("min_instances = 0", "min_instances = 2"));
        let (changes, patch) = compute_patch(&remote, &local).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "serve.min_instances");
        assert_eq!(patch, json!({"serve": {"min_instances": 2}}));
    }

    #[test]
    fn detects_access_type_change() {
        let remote = parse(BASE);
        let local = parse(&BASE.replace("access_type = \"private\"", "access_type = \"external\""));
        let (_changes, patch) = compute_patch(&remote, &local).unwrap();
        assert_eq!(patch, json!({"content": {"access_type": "external"}}));
    }

    #[test]
    fn absent_local_serve_is_not_reset() {
        let remote = parse(BASE);
        let local = parse(BASE_NO_SERVE);
        let (changes, patch) = compute_patch(&remote, &local).unwrap();
        assert!(patch.get("serve").is_none());
        assert!(!changes.iter().any(|c| c.field.starts_with("serve")));
    }

    #[test]
    fn detects_changes_across_sections() {
        let remote = parse(BASE);
        let modified = BASE
            .replace("access_type = \"private\"", "access_type = \"external\"")
            .replace("min_instances = 0", "min_instances = 3");
        let local = parse(&modified);
        let (changes, patch) = compute_patch(&remote, &local).unwrap();
        assert_eq!(
            patch,
            json!({
                "content": {"access_type": "external"},
                "serve": {"min_instances": 3}
            })
        );
        assert_eq!(changes.len(), 2);
    }
}
