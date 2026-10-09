use crate::log_stream::{LogLine, Stream};
use crate::{OutputFormat, client::RicochetClient, config::Config};
use anyhow::{Result, bail};
use colored::Colorize;
use console::Term;
use jiff::tz::TimeZone;
use std::collections::VecDeque;
use std::str::FromStr;

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
    let mut stream = client.stream_log(id).await?;
    let timezone = TimeZone::system();

    let term = Term::stdout();
    // A rolling window only makes sense where lines can be redrawn in place.
    let window = match (format, lines) {
        (OutputFormat::Table, Some(Lines::Count(count))) if term.is_term() => Some(count),
        (OutputFormat::Table, None) if term.is_term() => {
            Some(usize::from(term.size().0).saturating_sub(1).max(1))
        }
        _ => None,
    };

    let Some(window) = window else {
        while let Some(line) = stream.next_line().await? {
            match format {
                OutputFormat::Table => println!("{}", render_line(&line, &timezone)),
                OutputFormat::Json => println!("{}", serde_json::to_string(&line)?),
                OutputFormat::Yaml => {
                    println!("---\n{}", serde_yaml::to_string(&line)?.trim_end())
                }
            }
        }
        return Ok(());
    };

    let mut history = Vec::new();
    let mut visible = VecDeque::with_capacity(window);
    let interrupted = tokio::signal::ctrl_c();
    tokio::pin!(interrupted);
    loop {
        let line = tokio::select! {
            line = stream.next_line() => line?,
            _ = &mut interrupted => None,
        };
        let Some(line) = line else { break };

        let drawn = visible.len();
        let rendered = render_line(&line, &timezone);
        if visible.len() == window {
            visible.pop_front();
        }
        visible.push_back(rendered.clone());
        history.push(rendered);

        // Lines are cut to the terminal width so none wraps and throws off the redraw.
        let width = usize::from(term.size().1);
        term.move_cursor_up(drawn)?;
        for row in &visible {
            term.clear_line()?;
            term.write_line(&console::truncate_str(row, width, "…"))?;
        }
    }

    // Swap the window for every line so the whole log stays in the terminal's scrollback.
    term.move_cursor_up(visible.len())?;
    term.clear_to_end_of_screen()?;
    for row in &history {
        term.write_line(row)?;
    }
    Ok(())
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
    fn lines_rejects_zero_and_words() {
        for value in ["0", "inf", "-3"] {
            assert!(value.parse::<Lines>().is_err(), "{value}");
        }
    }
}
