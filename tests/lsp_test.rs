//! Drives `ricochet lsp` over stdio the way an editor does.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

const TIMEOUT: Duration = Duration::from_secs(20);

/// A `_ricochet.toml` whose entrypoint extension does not match its content type.
const INVALID_ITEM: &str = r#"[content]
name = "example"
entrypoint = "app.py"
access_type = "private"
content_type = "r"

[language]
name = "r"
packages = "renv.lock"
"#;

struct LspSession {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Value>,
    _home: TempDir,
}

impl LspSession {
    fn start() -> Self {
        Self::start_in(TempDir::new().expect("temp home"))
    }

    /// Start the server with `home` as its HOME and XDG config root.
    fn start_in(home: TempDir) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ricochet"))
            .arg("lsp")
            .env("HOME", home.path())
            .env("XDG_CONFIG_HOME", home.path().join(".config"))
            .env("RICOCHET_NO_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn ricochet lsp");
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let (sender, messages) = channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Some(message) = read_message(&mut reader) {
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            stdin,
            messages,
            _home: home,
        }
    }

    fn send(&mut self, message: &Value) {
        let body = message.to_string();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).expect("write message");
        self.stdin.flush().expect("flush message");
    }

    fn recv(&self) -> Value {
        match self.messages.recv_timeout(TIMEOUT) {
            Ok(message) => message,
            Err(RecvTimeoutError::Timeout) => {
                panic!("no message from the server within {TIMEOUT:?}")
            }
            Err(RecvTimeoutError::Disconnected) => panic!("the server closed stdout"),
        }
    }

    fn initialize(&mut self) {
        self.send(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"processId": null, "rootUri": null, "capabilities": {}}
        }));
        let response = self.recv();
        assert_eq!(response["id"], 1, "{response}");
        assert_eq!(
            response["result"]["capabilities"]["textDocumentSync"], 1,
            "{response}"
        );
        self.send(&json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));
    }

    fn open(&mut self, uri: &str, text: &str) {
        self.send(&json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "toml", "version": 1, "text": text
            }}
        }));
    }

    /// Send `shutdown` and return every message received up to and including its response.
    fn shutdown(&mut self) -> Vec<Value> {
        self.send(&json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null}));
        let mut received = Vec::new();
        loop {
            let message = self.recv();
            let done = message["id"] == 2;
            received.push(message);
            if done {
                return received;
            }
        }
    }

    fn exit(mut self) -> std::process::ExitStatus {
        self.send(&json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
        let deadline = std::time::Instant::now() + TIMEOUT;
        loop {
            if let Some(status) = self.child.try_wait().expect("poll child") {
                return status;
            }
            if std::time::Instant::now() > deadline {
                let _ = self.child.kill();
                panic!("the server did not exit within {TIMEOUT:?}");
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for LspSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn read_message(reader: &mut impl BufRead) -> Option<Value> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0; length?];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn file_uri(dir: &TempDir, name: &str) -> String {
    url::Url::from_file_path(dir.path().join(name))
        .expect("absolute path")
        .to_string()
}

#[test]
fn lsp_publishes_diagnostics_for_ricochet_toml() {
    let dir = TempDir::new().expect("temp dir");
    let uri = file_uri(&dir, "_ricochet.toml");
    let mut session = LspSession::start();
    session.initialize();
    session.open(&uri, INVALID_ITEM);

    let published = session.recv();
    assert_eq!(
        published["method"], "textDocument/publishDiagnostics",
        "{published}"
    );
    assert_eq!(published["params"]["uri"], uri, "{published}");
    let diagnostics = published["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    assert_eq!(diagnostics.len(), 1, "{published}");
    assert_eq!(diagnostics[0]["source"], "ricochet", "{published}");

    let received = session.shutdown();
    assert_eq!(received.len(), 1, "unexpected extra messages: {received:?}");
    assert!(session.exit().success());
}

#[test]
fn lsp_ignores_other_toml_files() {
    let dir = TempDir::new().expect("temp dir");
    let mut session = LspSession::start();
    session.initialize();
    session.open(&file_uri(&dir, "Cargo.toml"), INVALID_ITEM);

    // The server handles messages in order, so anything it would publish for the
    // open arrives before the response to the shutdown that follows it.
    let received = session.shutdown();
    let published: Vec<_> = received
        .iter()
        .filter(|message| message["method"] == "textDocument/publishDiagnostics")
        .collect();
    assert!(published.is_empty(), "{published:?}");
    assert!(session.exit().success());
}

#[test]
fn lsp_starts_with_a_broken_config_file() {
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join(".config").join("ricochet");
    std::fs::create_dir_all(&config_dir).expect("create config dir");
    std::fs::write(config_dir.join("config.toml"), "garbage = [").expect("write config");

    let mut session = LspSession::start_in(home);
    session.initialize();
    let received = session.shutdown();
    assert_eq!(received.len(), 1, "unexpected extra messages: {received:?}");
    assert!(session.exit().success());
}
