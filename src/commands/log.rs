use crate::log_stream::{LogLine, Stream};
use crate::{OutputFormat, client::RicochetClient, config::Config};
use anyhow::{Result, bail};
use colored::Colorize;
use console::Term;
use jiff::tz::TimeZone;
use std::collections::VecDeque;
use std::str::FromStr;

/// The most lines kept for the scrollback replay, about what a terminal keeps itself.
const SCROLLBACK_LINES: usize = 10_000;

/// How many of the latest log lines stay on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lines {
    Count(usize),
    All,
}

impl FromStr for Lines {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        if value.eq_ignore_ascii_case("all") {
            return Ok(Self::All);
        }
        match value.parse::<usize>() {
            Ok(0) | Err(_) => bail!("expected a positive number of lines or `all`, got `{value}`"),
            Ok(count) => Ok(Self::Count(count)),
        }
    }
}

/// How following a log finished.
#[derive(Debug, PartialEq)]
pub(crate) enum LogEnd {
    /// The server closed the log.
    Closed,
    /// The viewer pressed Ctrl-C.
    Interrupted,
    /// The reader of stdout, such as `head`, closed the pipe.
    ReaderGone,
}

/// Print a log as it is written, until the server closes it.
pub async fn follow_log(
    config: &Config,
    server_ref: Option<&str>,
    id: &str,
    lines: Option<Lines>,
    format: OutputFormat,
) -> Result<()> {
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    let term = Term::buffered_stdout();
    if print_log(&client, server_ref, id, lines, format, &term).await? == LogEnd::Interrupted {
        // Ctrl-C is caught only to restore the scrollback, so exit as an uncaught interrupt would.
        std::process::exit(130);
    }
    Ok(())
}

/// Write a log to `term` as it is written, until the server closes it or the viewer stops watching.
///
/// `term` is flushed after every batch of lines, so it can be buffered.
pub(crate) async fn print_log(
    client: &RicochetClient,
    server_ref: Option<&str>,
    id: &str,
    lines: Option<Lines>,
    format: OutputFormat,
    term: &Term,
) -> Result<LogEnd> {
    // Once Ctrl-C is caught it no longer ends the process, so every wait below watches for it.
    let interrupted = tokio::signal::ctrl_c();
    tokio::pin!(interrupted);
    let mut stream = tokio::select! {
        stream = client.stream_log(id) => stream?,
        _ = &mut interrupted => return Ok(LogEnd::Interrupted),
    };
    let timezone = TimeZone::system();

    // A rolling window only makes sense where lines can be redrawn in place.
    // Each redraw caps the window at the terminal height.
    let window = match (format, lines) {
        (OutputFormat::Table, Some(Lines::Count(count))) if term.is_term() => Some(count),
        (OutputFormat::Table, None) if term.is_term() => Some(usize::MAX),
        _ => None,
    };

    let Some(window) = window else {
        loop {
            let line = tokio::select! {
                line = stream.next_line() => line?,
                _ = &mut interrupted => return Ok(LogEnd::Interrupted),
            };
            let Some(line) = line else {
                return Ok(LogEnd::Closed);
            };
            let record = format.render_record(&line, || Ok(render_line(&line, &timezone)))?;
            if let Err(error) = term.write_line(&record).and_then(|()| term.flush()) {
                // A reader such as `head` that has seen enough closes the pipe, which ends the follow.
                return match error.kind() {
                    std::io::ErrorKind::BrokenPipe => Ok(LogEnd::ReaderGone),
                    _ => Err(error.into()),
                };
            }
        }
    };

    let mut history = VecDeque::new();
    let mut omitted = 0;
    let mut drawn = 0;
    let outcome = loop {
        let line = tokio::select! {
            line = stream.next_line() => line,
            _ = &mut interrupted => break Ok(LogEnd::Interrupted),
        };
        let line = match line {
            Ok(Some(line)) => line,
            Ok(None) => break Ok(LogEnd::Closed),
            Err(error) => break Err(error),
        };

        // Lines that arrived together are drawn once, so replaying a finished log stays fast.
        let buffered = std::iter::from_fn(|| stream.take_buffered_line());
        for mut line in std::iter::once(line).chain(buffered) {
            line.line = screen_text(&line.line);
            if history.len() == SCROLLBACK_LINES {
                history.pop_front();
                omitted += 1;
            }
            history.push_back(render_line(&line, &timezone));
        }

        // The cursor cannot reach rows that scrolled off the screen, so the window never outgrows it.
        // Rows are cut to the terminal width so none wraps and throws off the redraw.
        let (height, width) = term.size();
        let rows = window
            .min(usize::from(height).saturating_sub(1).max(1))
            .min(history.len());
        term.move_cursor_up(drawn)?;
        term.clear_to_end_of_screen()?;
        for row in history.iter().skip(history.len() - rows) {
            term.write_line(&console::truncate_str(row, usize::from(width), "…"))?;
        }
        term.flush()?;
        drawn = rows;
    };

    // Swap the window for every line so the whole log stays in the terminal's scrollback.
    term.move_cursor_up(drawn)?;
    term.clear_to_end_of_screen()?;
    if omitted > 0 {
        let server_flag = server_ref
            .map(|server| format!(" --server {server}"))
            .unwrap_or_default();
        let notice = format!(
            "{omitted} earlier lines are not shown. Run `ricochet log {id} --lines all{server_flag}` to see every line."
        );
        term.write_line(&notice.dimmed().to_string())?;
    }
    for row in &history {
        term.write_line(row)?;
    }
    term.flush()?;
    outcome
}

/// Text as a terminal shows it on one row: what follows the last carriage return, with tabs expanded.
fn screen_text(text: &str) -> String {
    let text = text
        .trim_end_matches('\r')
        .rsplit('\r')
        .next()
        .unwrap_or_default();
    let mut row = String::with_capacity(text.len());
    for character in text.chars() {
        if character == '\t' {
            let column = console::measure_text_width(&row);
            row.push_str(&" ".repeat(8 - column % 8));
        } else {
            row.push(character);
        }
    }
    row
}

/// Render a line as `HH:MM:SS stream text`, with the time in the viewer's zone.
fn render_line(line: &LogLine, timezone: &TimeZone) -> String {
    let (Some(timestamp), Some(stream)) = (&line.timestamp, &line.stream) else {
        return line.line.clone();
    };
    let time = timestamp
        .to_zoned(timezone.clone())
        .strftime("%H:%M:%S")
        .to_string();
    let label = match stream {
        Stream::Stdout => "stdout".cyan(),
        Stream::Stderr => "stderr".yellow(),
        Stream::Other(label) => label.as_str().magenta(),
    };
    format!("{} {label} {}", time.dimmed(), line.line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_accepts_a_count_or_all() {
        assert_eq!("20".parse::<Lines>().ok(), Some(Lines::Count(20)));
        assert_eq!("ALL".parse::<Lines>().ok(), Some(Lines::All));
    }

    #[test]
    fn screen_text_keeps_the_last_overwrite_and_expands_tabs() {
        assert_eq!(screen_text("10%\r55%\r100%"), "100%");
        assert_eq!(screen_text("done\r"), "done");
        assert_eq!(screen_text("a\tbc\td"), "a       bc      d");
    }

    #[test]
    fn lines_rejects_zero_and_words() {
        for value in ["0", "inf", "-3"] {
            assert!(value.parse::<Lines>().is_err(), "{value}");
        }
    }
}
