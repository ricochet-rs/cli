//! Output rules shared by every command.
//!
//! `-F json` and `-F yaml` reserve stdout for the serialised payload, so status
//! lines, hints, spinners and links belong on stderr.

use anyhow::Result;
use serde::Serialize;

#[derive(clap::ValueEnum, Clone, Debug, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
}

impl OutputFormat {
    /// Render `payload` for a machine reader, or call `table` for a person.
    pub fn render<T: Serialize>(
        self,
        payload: &T,
        table: impl FnOnce() -> Result<String>,
    ) -> Result<String> {
        match self {
            Self::Json => Ok(serde_json::to_string_pretty(payload)?),
            Self::Yaml => Ok(serde_yaml::to_string(payload)?.trim_end().to_string()),
            Self::Table => table(),
        }
    }

    /// Write the rendered payload to stdout, the only stream a caller parses.
    pub fn print<T: Serialize>(
        self,
        payload: &T,
        table: impl FnOnce() -> Result<String>,
    ) -> Result<()> {
        println!("{}", self.render(payload, table)?);
        Ok(())
    }

    /// Render one record of a stream: a JSON line, a YAML document, or `table` for a person.
    pub fn render_record<T: Serialize>(
        self,
        record: &T,
        table: impl FnOnce() -> Result<String>,
    ) -> Result<String> {
        match self {
            Self::Json => Ok(serde_json::to_string(record)?),
            Self::Yaml => Ok(format!(
                "---\n{}",
                serde_yaml::to_string(record)?.trim_end()
            )),
            Self::Table => table(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_carries_the_payload_alone() -> Result<()> {
        let rendered =
            OutputFormat::Json.render(&json!({"id": "01J"}), || Ok("human text".to_string()))?;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&rendered)?["id"],
            "01J"
        );
        Ok(())
    }

    #[test]
    fn yaml_carries_the_payload_alone() -> Result<()> {
        let rendered =
            OutputFormat::Yaml.render(&json!({"id": "01J"}), || Ok("human text".to_string()))?;
        assert_eq!(rendered, "id: 01J");
        Ok(())
    }

    #[test]
    fn table_defers_to_the_human_rendering() -> Result<()> {
        let rendered =
            OutputFormat::Table.render(&json!({"id": "01J"}), || Ok("human text".to_string()))?;
        assert_eq!(rendered, "human text");
        Ok(())
    }

    #[test]
    fn a_json_record_fits_on_one_line() -> Result<()> {
        let rendered = OutputFormat::Json
            .render_record(&json!({"line": "a", "stream": "stdout"}), || {
                Ok("human text".to_string())
            })?;
        assert!(!rendered.contains('\n'));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&rendered)?["line"],
            "a"
        );
        Ok(())
    }

    #[test]
    fn a_yaml_record_is_its_own_document() -> Result<()> {
        let rendered = OutputFormat::Yaml
            .render_record(&json!({"line": "a"}), || Ok("human text".to_string()))?;
        assert_eq!(rendered, "---\nline: a");
        Ok(())
    }
}
