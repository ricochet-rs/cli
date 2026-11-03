//! A language server implmeneted for the `_ricochet.toml` format.

use lsp_server::{Connection, ErrorCode, IoThreads, Message, Response};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, Position, PublishDiagnosticsParams, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
        PublishDiagnostics,
    },
};
use ricochet_core::content::{ContentError, ContentItem};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use toml::de::DeTable;

const RICOCHET_TOML: &str = "_ricochet.toml";

pub struct RicochetLsp {
    connection: Connection,
    io_threads: IoThreads,
}

impl Default for RicochetLsp {
    fn default() -> Self {
        let (connection, io_threads) = Connection::stdio();
        Self {
            connection,
            io_threads,
        }
    }
}

impl RicochetLsp {
    /// Starts the _ricochet.toml LSP and renders diagnostics to stdout
    pub fn spawn(self) -> anyhow::Result<()> {
        self.connection
            .initialize(serde_json::to_value(ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..ServerCapabilities::default()
            })?)?;

        for message in &self.connection.receiver {
            match message {
                Message::Request(request) => {
                    if self.connection.handle_shutdown(&request)? {
                        break;
                    }
                    let message = format!("unhandled method {}", request.method);
                    self.connection
                        .sender
                        .send(Message::Response(Response::new_err(
                            request.id,
                            ErrorCode::MethodNotFound as i32,
                            message,
                        )))?;
                }
                Message::Notification(notification) => {
                    self.handle_notification(notification)?;
                }
                Message::Response(_) => {}
            }
        }

        // explicit drop is required for the i/o threads to join
        // don't delete this
        drop(self.connection);
        self.io_threads.join()?;
        Ok(())
    }

    /// React to an LSP notification
    fn handle_notification(&self, notification: lsp_server::Notification) -> anyhow::Result<()> {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let Some(params) = Self::parse_params::<DidOpenTextDocumentParams>(notification)
                else {
                    return Ok(());
                };
                let document = params.text_document;
                if let Some(dir) = Self::ricochet_toml_dir(&document.uri) {
                    self.publish(document.uri, Self::diagnose(&document.text, &dir))?;
                }
            }
            DidChangeTextDocument::METHOD => {
                let Some(params) = Self::parse_params::<DidChangeTextDocumentParams>(notification)
                else {
                    return Ok(());
                };
                let uri = params.text_document.uri;
                if let (Some(dir), Some(change)) =
                    (Self::ricochet_toml_dir(&uri), params.content_changes.last())
                {
                    let diagnostic = Self::diagnose(&change.text, &dir);
                    self.publish(uri, diagnostic)?;
                }
            }
            DidCloseTextDocument::METHOD => {
                let Some(params) = Self::parse_params::<DidCloseTextDocumentParams>(notification)
                else {
                    return Ok(());
                };
                let uri = params.text_document.uri;
                if Self::ricochet_toml_dir(&uri).is_some() {
                    self.publish(uri, None)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Parses a notification's params, returning `None` if they are malformed
    fn parse_params<P: DeserializeOwned>(notification: lsp_server::Notification) -> Option<P> {
        serde_json::from_value(notification.params)
            .inspect_err(|error| {
                tracing::warn!(method = %notification.method, %error, "ignoring malformed notification");
            })
            .ok()
    }

    /// Returns the directory of a `_ricochet.toml`, or `None` for any other file
    fn ricochet_toml_dir(uri: &Uri) -> Option<PathBuf> {
        let path = url::Url::parse(uri.as_str()).ok()?.to_file_path().ok()?;
        if path.file_name()? != RICOCHET_TOML {
            return None;
        }
        path.parent().map(Path::to_path_buf)
    }

    fn publish(&self, uri: Uri, diagnostic: Option<Diagnostic>) -> anyhow::Result<()> {
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics: diagnostic.into_iter().collect(),
            version: None,
        };
        self.connection
            .sender
            .send(Message::Notification(lsp_server::Notification::new(
                PublishDiagnostics::METHOD.to_owned(),
                params,
            )))?;
        Ok(())
    }

    /// Validates a `_ricochet.toml` and returns its first error as a diagnostic, or `None` if the file is valid
    fn diagnose(text: &str, document_dir: &Path) -> Option<Diagnostic> {
        let (span, message) = match toml::from_str::<ContentItem>(text) {
            Err(error) => (error.span().unwrap_or(0..0), error.message().to_owned()),
            Ok(mut item) => {
                item.content.entrypoint = document_dir.join(&item.content.entrypoint);
                // if the content item successful deserializes, we need to confirm the config is valid
                // if not, we need to provide guidance on _where_ the error actually comes from
                let Err(error) = item.validate_config() else {
                    return None;
                };
                // provide guidance on which toml keys are actually causing the error here.
                let key_path: &[&str] = match &error {
                    ContentError::InvalidVersion(_) => &["language", "version"],
                    ContentError::InvalidLangauge(_) => &["language", "name"],
                    ContentError::PackagesRequired | ContentError::MismatchedPackages => {
                        &["language", "packages"]
                    }
                    ContentError::InvalidEntrypoint(_) => &["content", "entrypoint"],
                    ContentError::NotServable => &["serve"],
                    ContentError::NotSchedualable => &["schedule"],
                    ContentError::NotStatic => &["static"],
                    ContentError::MissingContentId => &["content", "id"],
                };
                let span = DeTable::parse(text)
                    .ok()
                    .and_then(|document| {
                        let (last, parents) = key_path.split_last()?;
                        let mut table = document.get_ref();
                        for segment in parents {
                            let (_, value) =
                                table.iter().find(|(key, _)| key.get_ref() == segment)?;
                            table = value.get_ref().as_table()?;
                        }
                        let (key, _) = table.iter().find(|(key, _)| key.get_ref() == last)?;
                        Some(key.span())
                    })
                    .unwrap_or(0..0);
                (span, error.to_string())
            }
        };
        Some(Diagnostic {
            range: lsp_types::Range::new(
                Self::line_and_column(text, span.start),
                Self::line_and_column(text, span.end),
            ),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("ricochet".to_owned()),
            message,
            ..Diagnostic::default()
        })
    }

    /// Converts a byte offset to an LSP position
    fn line_and_column(text: &str, offset: usize) -> Position {
        let before = text.get(..offset).unwrap_or(text);
        let line_start = before.rfind('\n').map_or(0, |index| index + 1);
        let line = before.matches('\n').count();
        let character = before[line_start..].encode_utf16().count();
        Position::new(
            u32::try_from(line).unwrap_or(u32::MAX),
            u32::try_from(character).unwrap_or(u32::MAX),
        )
    }
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

    fn line_of(text: &str, needle: &str) -> u32 {
        let index = text
            .lines()
            .position(|line| line.starts_with(needle))
            .expect("needle is in the text");
        u32::try_from(index).expect("line fits in u32")
    }

    #[test]
    fn diagnose_reports_parse_error_at_its_span() {
        let diagnostic =
            RicochetLsp::diagnose("[content\n", Path::new(".")).expect("a parse error");
        assert_eq!(diagnostic.range.start.line, 0);
    }

    #[test]
    fn diagnose_reports_unknown_enum_value_at_value() {
        let text = item("shinyy", "app.R");
        let diagnostic =
            RicochetLsp::diagnose(&text, Path::new(".")).expect("an unknown content type");
        assert_eq!(diagnostic.range.start.line, line_of(&text, "content_type"));
        assert!(diagnostic.message.contains("shinyy"), "{diagnostic:?}");
    }

    #[test]
    fn diagnose_maps_validate_config_error_to_key() {
        let text = item("r", "app.py");
        let diagnostic =
            RicochetLsp::diagnose(&text, Path::new(".")).expect("a wrong entrypoint extension");
        assert_eq!(diagnostic.range.start.line, line_of(&text, "entrypoint"));
        assert!(
            diagnostic
                .message
                .contains("R content type requires a file ending in `.R`."),
            "{diagnostic:?}"
        );
    }

    #[test]
    fn diagnose_points_serve_on_task_at_serve_table() {
        let text = format!("{}{SERVE}", item("quarto-r", "report.qmd"));
        let diagnostic =
            RicochetLsp::diagnose(&text, Path::new(".")).expect("a serve table on a task");
        assert_eq!(diagnostic.range.start.line, line_of(&text, "[serve]"));
        assert!(
            diagnostic.message.contains("cannot be served"),
            "{diagnostic:?}"
        );
    }

    #[test]
    fn diagnose_accepts_shiny_directory_entrypoint() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir(dir.path().join("app")).expect("app dir");
        let text = item("shiny", "app");
        assert!(RicochetLsp::diagnose(&text, dir.path()).is_none());
    }

    #[test]
    fn diagnose_returns_none_for_valid_file() {
        let text = format!("{}{SERVE}", item("shiny", "app.R"));
        assert!(RicochetLsp::diagnose(&text, Path::new(".")).is_none());
    }
}
