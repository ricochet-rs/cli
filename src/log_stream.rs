//! Follow a server-side log as it is written.

use anyhow::Result;
use jiff::Timestamp;
use reqwest::Response;
use serde::Serialize;

/// The output stream a log line was written to.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Stream {
    Stdout,
    Stderr,
    /// A label the server writes for other sources, such as a container name.
    #[serde(untagged)]
    Other(String),
}

/// One line of a log, split from its `[timestamp stream]` prefix when it has one.
#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct LogLine {
    pub(crate) line: String,
    pub(crate) timestamp: Option<Timestamp>,
    pub(crate) stream: Option<Stream>,
}

impl LogLine {
    /// Decode a line's bytes without its line ending.
    fn from_bytes(bytes: &[u8]) -> Self {
        let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
        Self::parse(String::from_utf8_lossy(bytes).into_owned())
    }

    fn parse(raw: String) -> Self {
        let prefixed = raw
            .strip_prefix('[')
            .and_then(|rest| rest.split_once(']'))
            .and_then(|(prefix, text)| {
                let (timestamp, stream) = prefix.split_once(' ')?;
                Some((timestamp.parse::<Timestamp>().ok()?, stream, text))
            });

        let Some((timestamp, stream, text)) = prefixed else {
            return Self {
                line: raw,
                timestamp: None,
                stream: None,
            };
        };
        let stream = match stream {
            "stdout" => Stream::Stdout,
            "stderr" => Stream::Stderr,
            other => Stream::Other(other.to_string()),
        };
        Self {
            line: text.strip_prefix(' ').unwrap_or(text).to_string(),
            timestamp: Some(timestamp),
            stream: Some(stream),
        }
    }
}

/// A log streamed from the server a line at a time, ending when the server closes it.
pub(crate) struct LogStream {
    response: Response,
    pending: Vec<u8>,
}

impl LogStream {
    pub(crate) fn new(response: Response) -> Self {
        Self {
            response,
            pending: Vec::new(),
        }
    }

    /// The next line, or `None` once the log has ended.
    pub(crate) async fn next_line(&mut self) -> Result<Option<LogLine>> {
        loop {
            if let Some(line) = self.take_buffered_line() {
                return Ok(Some(line));
            }
            match self.response.chunk().await? {
                Some(chunk) => self.pending.extend_from_slice(&chunk),
                None if self.pending.is_empty() => return Ok(None),
                None => {
                    let line = std::mem::take(&mut self.pending);
                    return Ok(Some(LogLine::from_bytes(&line)));
                }
            }
        }
    }

    /// Take the next complete line already received, without waiting for the server.
    pub(crate) fn take_buffered_line(&mut self) -> Option<LogLine> {
        let end = self.pending.iter().position(|byte| *byte == b'\n')?;
        let line: Vec<u8> = self.pending.drain(..=end).collect();
        Some(LogLine::from_bytes(&line[..end]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prefixed_line_splits_into_timestamp_stream_and_text() {
        let line = LogLine::parse("[2026-10-09T13:48:06.643970Z stderr] Warning: upgrade".into());
        assert_eq!(
            line,
            LogLine {
                line: "Warning: upgrade".into(),
                timestamp: "2026-10-09T13:48:06.643970Z".parse().ok(),
                stream: Some(Stream::Stderr),
            }
        );
    }

    #[test]
    fn an_empty_prefixed_line_keeps_its_timestamp() {
        let line = LogLine::parse("[2026-10-09T13:48:06.645257Z stdout]".into());
        assert_eq!(line.line, "");
        assert_eq!(line.stream, Some(Stream::Stdout));
    }

    #[test]
    fn an_unknown_stream_label_is_kept() {
        let line = LogLine::parse("[2026-10-09T13:48:06Z init:restore] pulling".into());
        assert_eq!(line.stream, Some(Stream::Other("init:restore".into())));
    }

    #[test]
    fn a_crlf_line_drops_its_carriage_return() {
        let line = LogLine::from_bytes(b"[2026-10-09T13:48:06Z stdout] windows line\r");
        assert_eq!(line.line, "windows line");
    }

    #[test]
    fn a_line_without_a_timestamp_prefix_is_text_alone() {
        for raw in [
            "Execution environment: host",
            "[not a timestamp stdout] text",
        ] {
            let line = LogLine::parse(raw.into());
            assert_eq!(line.line, raw);
            assert_eq!(line.timestamp, None);
            assert_eq!(line.stream, None);
        }
    }
}
