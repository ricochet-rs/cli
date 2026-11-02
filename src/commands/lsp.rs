//! A language server that checks `_ricochet.toml` against the rules the server applies on deploy.

use std::ops::Range;
use std::path::{Path, PathBuf};

use lsp_server::{Connection, ErrorCode, Message, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
    PublishDiagnostics,
};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, Position, PublishDiagnosticsParams, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};
use ricochet_core::content::{ContentError, ContentItem};
use serde::de::DeserializeOwned;
use toml::de::DeTable;

const RICOCHET_TOML: &str = "_ricochet.toml";

/// Serve diagnostics for `_ricochet.toml` over stdio until the client sends `exit`.
pub fn serve_lsp() -> anyhow::Result<()> {
    let (connection, io_threads) = Connection::stdio();
    connection.initialize(serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..ServerCapabilities::default()
    })?)?;

    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    break;
                }
                let message = format!("unhandled method {}", request.method);
                connection.sender.send(Message::Response(Response::new_err(
                    request.id,
                    ErrorCode::MethodNotFound as i32,
                    message,
                )))?;
            }
            Message::Notification(notification) => {
                handle_notification(&connection, notification)?;
            }
            Message::Response(_) => {}
        }
    }

    drop(connection);
    io_threads.join()?;
    Ok(())
}

/// Publish diagnostics for a document event, ignoring a notification whose params do not parse.
fn handle_notification(
    connection: &Connection,
    notification: lsp_server::Notification,
) -> anyhow::Result<()> {
    match notification.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let Some(params) = parse_params::<DidOpenTextDocumentParams>(notification) else {
                return Ok(());
            };
            let document = params.text_document;
            if let Some(dir) = ricochet_toml_dir(&document.uri) {
                publish(connection, document.uri, diagnose(&document.text, &dir))?;
            }
        }
        DidChangeTextDocument::METHOD => {
            let Some(params) = parse_params::<DidChangeTextDocumentParams>(notification) else {
                return Ok(());
            };
            let uri = params.text_document.uri;
            if let (Some(dir), Some(change)) =
                (ricochet_toml_dir(&uri), params.content_changes.last())
            {
                let diagnostics = diagnose(&change.text, &dir);
                publish(connection, uri, diagnostics)?;
            }
        }
        DidCloseTextDocument::METHOD => {
            let Some(params) = parse_params::<DidCloseTextDocumentParams>(notification) else {
                return Ok(());
            };
            let uri = params.text_document.uri;
            if ricochet_toml_dir(&uri).is_some() {
                publish(connection, uri, Vec::new())?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// The notification's params, or `None` with a warning when the client sent params that do not parse.
fn parse_params<P: DeserializeOwned>(notification: lsp_server::Notification) -> Option<P> {
    serde_json::from_value(notification.params)
        .inspect_err(|error| {
            tracing::warn!(method = %notification.method, %error, "ignoring malformed notification");
        })
        .ok()
}

/// The directory holding the document when it is a local `_ricochet.toml`.
fn ricochet_toml_dir(uri: &Uri) -> Option<PathBuf> {
    let path = url::Url::parse(uri.as_str()).ok()?.to_file_path().ok()?;
    if path.file_name()? != RICOCHET_TOML {
        return None;
    }
    path.parent().map(Path::to_path_buf)
}

fn publish(connection: &Connection, uri: Uri, diagnostics: Vec<Diagnostic>) -> anyhow::Result<()> {
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    };
    connection
        .sender
        .send(Message::Notification(lsp_server::Notification::new(
            PublishDiagnostics::METHOD.to_owned(),
            params,
        )))?;
    Ok(())
}

/// The document key a `ContentError` is about, so its diagnostic lands on that line.
#[derive(Clone, Copy)]
enum ContentErrorKey {
    LanguageVersion,
    LanguageName,
    LanguagePackages,
    Entrypoint,
    ServeTable,
    ScheduleTable,
    StaticTable,
    ContentId,
}

impl From<&ContentError> for ContentErrorKey {
    fn from(error: &ContentError) -> Self {
        match error {
            ContentError::InvalidVersion(_) => Self::LanguageVersion,
            ContentError::InvalidLangauge(_) => Self::LanguageName,
            ContentError::PackagesRequired | ContentError::MismatchedPackages => {
                Self::LanguagePackages
            }
            ContentError::InvalidEntrypoint(_) => Self::Entrypoint,
            ContentError::NotServable => Self::ServeTable,
            ContentError::NotSchedualable => Self::ScheduleTable,
            ContentError::NotStatic => Self::StaticTable,
            ContentError::MissingContentId => Self::ContentId,
        }
    }
}

impl ContentErrorKey {
    fn path(self) -> &'static [&'static str] {
        match self {
            Self::LanguageVersion => &["language", "version"],
            Self::LanguageName => &["language", "name"],
            Self::LanguagePackages => &["language", "packages"],
            Self::Entrypoint => &["content", "entrypoint"],
            Self::ServeTable => &["serve"],
            Self::ScheduleTable => &["schedule"],
            Self::StaticTable => &["static"],
            Self::ContentId => &["content", "id"],
        }
    }

    /// The byte span of this key in `text`, or `None` when the document does not contain it.
    fn locate(self, text: &str) -> Option<Range<usize>> {
        let document = DeTable::parse(text).ok()?;
        let (last, parents) = self.path().split_last()?;
        let mut table = document.get_ref();
        for segment in parents {
            let (_, value) = table.iter().find(|(key, _)| key.get_ref() == segment)?;
            table = value.get_ref().as_table()?;
        }
        let (key, _) = table.iter().find(|(key, _)| key.get_ref() == last)?;
        Some(key.span())
    }
}

/// Check a `_ricochet.toml` whose directory is `document_dir`, as `ricochet deploy` would.
fn diagnose(text: &str, document_dir: &Path) -> Vec<Diagnostic> {
    let (span, message) = match toml::from_str::<ContentItem>(text) {
        Err(error) => (error.span().unwrap_or(0..0), error.message().to_owned()),
        Ok(mut item) => {
            item.content.entrypoint = document_dir.join(&item.content.entrypoint);
            let Err(error) = item.validate_config() else {
                return Vec::new();
            };
            let span = ContentErrorKey::from(&error).locate(text).unwrap_or(0..0);
            (span, error.to_string())
        }
    };
    vec![Diagnostic {
        range: lsp_types::Range::new(position_at(text, span.start), position_at(text, span.end)),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("ricochet".to_owned()),
        message,
        ..Diagnostic::default()
    }]
}

/// The LSP position of a byte offset, counting columns in UTF-16 code units.
fn position_at(text: &str, offset: usize) -> Position {
    let before = text.get(..offset).unwrap_or(text);
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let line = before.matches('\n').count();
    let character = before[line_start..].encode_utf16().count();
    Position::new(
        u32::try_from(line).unwrap_or(u32::MAX),
        u32::try_from(character).unwrap_or(u32::MAX),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVE: &str = r#"
[serve]
min_instances = 0
max_instances = 5
spawn_threshold = 80
max_connections = 10
"#;

    fn item(content_type: &str, entrypoint: &str) -> String {
        format!(
            r#"[content]
name = "example"
entrypoint = "{entrypoint}"
access_type = "private"
content_type = "{content_type}"

[language]
name = "r"
packages = "renv.lock"
"#
        )
    }

    fn lines_and_messages(diagnostics: &[Diagnostic]) -> Vec<(u32, String)> {
        diagnostics
            .iter()
            .map(|d| (d.range.start.line, d.message.clone()))
            .collect()
    }

    fn line_of(text: &str, needle: &str) -> u32 {
        let index = text
            .lines()
            .position(|line| line.starts_with(needle))
            .expect("needle is in the text");
        u32::try_from(index).expect("line fits in u32")
    }

    #[test]
    fn diagnose_reports_parse_error_at_its_span() {
        let found = lines_and_messages(&diagnose("[content\n", Path::new(".")));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].0, 0);
    }

    #[test]
    fn diagnose_reports_unknown_enum_value_at_value() {
        let text = item("shinyy", "app.R");
        let found = lines_and_messages(&diagnose(&text, Path::new(".")));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].0, line_of(&text, "content_type"));
        assert!(found[0].1.contains("shinyy"), "{found:?}");
    }

    #[test]
    fn diagnose_maps_validate_config_error_to_key() {
        let text = item("r", "app.py");
        let found = lines_and_messages(&diagnose(&text, Path::new(".")));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].0, line_of(&text, "entrypoint"));
        assert!(
            found[0]
                .1
                .contains("R content type requires a file ending in `.R`."),
            "{found:?}"
        );
    }

    #[test]
    fn diagnose_points_serve_on_task_at_serve_table() {
        let text = format!("{}{SERVE}", item("quarto-r", "report.qmd"));
        let found = lines_and_messages(&diagnose(&text, Path::new(".")));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].0, line_of(&text, "[serve]"));
        assert!(found[0].1.contains("cannot be served"), "{found:?}");
    }

    #[test]
    fn diagnose_accepts_shiny_directory_entrypoint() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir(dir.path().join("app")).expect("app dir");
        let text = item("shiny", "app");
        assert!(diagnose(&text, dir.path()).is_empty());
    }

    #[test]
    fn diagnose_returns_empty_for_valid_file() {
        let text = format!("{}{SERVE}", item("shiny", "app.R"));
        assert!(diagnose(&text, Path::new(".")).is_empty());
    }

    #[test]
    fn handle_notification_ignores_malformed_params() {
        let (server, client) = Connection::memory();
        let notification = lsp_server::Notification::new(
            DidOpenTextDocument::METHOD.to_owned(),
            serde_json::json!({"textDocument": {"uri": "not a uri", "text": "[content\n"}}),
        );
        handle_notification(&server, notification).expect("malformed params are not an error");
        assert!(client.receiver.try_recv().is_err());
    }

    #[test]
    fn handle_notification_publishes_for_ricochet_toml() {
        let (server, client) = Connection::memory();
        let dir = tempfile::tempdir().expect("temp dir");
        let uri = url::Url::from_file_path(dir.path().join(RICOCHET_TOML)).expect("file url");
        let notification = lsp_server::Notification::new(
            DidOpenTextDocument::METHOD.to_owned(),
            serde_json::json!({"textDocument": {
                "uri": uri.as_str(), "languageId": "toml", "version": 1, "text": "[content\n"
            }}),
        );
        handle_notification(&server, notification).expect("valid params");
        let Ok(Message::Notification(published)) = client.receiver.try_recv() else {
            panic!("expected a published diagnostic");
        };
        assert_eq!(published.method, PublishDiagnostics::METHOD);
    }
}
