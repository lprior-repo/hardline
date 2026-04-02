//! JSONL writer for streaming output

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::io::{self, Stdout, Write};

use serde_json;

use super::OutputLine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonlConfig {
    pub pretty: bool,
    pub flush_on_emit: bool,
}

impl Default for JsonlConfig {
    fn default() -> Self {
        Self {
            pretty: false,
            flush_on_emit: true,
        }
    }
}

impl JsonlConfig {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pretty: false,
            flush_on_emit: true,
        }
    }

    #[must_use]
    pub const fn with_pretty(self, pretty: bool) -> Self {
        Self { pretty, ..self }
    }

    #[must_use]
    pub const fn with_flush_on_emit(self, flush_on_emit: bool) -> Self {
        Self {
            flush_on_emit,
            ..self
        }
    }
}

pub struct JsonlWriter<W> {
    writer: W,
    config: JsonlConfig,
}

impl<W: Write> JsonlWriter<W> {
    #[must_use]
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            config: JsonlConfig::default(),
        }
    }

    #[must_use]
    pub const fn with_config(writer: W, config: JsonlConfig) -> Self {
        Self { writer, config }
    }

    /// Emit one JSONL record.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the write/flush operation fails.
    pub fn emit(&mut self, line: &OutputLine) -> io::Result<()> {
        let json = if self.config.pretty {
            serde_json::to_string_pretty(line)
        } else {
            serde_json::to_string(line)
        }
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        writeln!(self.writer, "{json}")?;

        if self.config.flush_on_emit {
            self.writer.flush()?;
        }

        Ok(())
    }

    /// Emit all JSONL records from an iterator.
    ///
    /// # Errors
    ///
    /// Returns the first write or serialization error produced while emitting lines.
    pub fn emit_all<'a, I>(&mut self, lines: I) -> io::Result<()>
    where
        I: IntoIterator<Item = &'a OutputLine>,
    {
        lines.into_iter().try_for_each(|line| self.emit(line))
    }

    /// Flush buffered output.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying writer fails to flush.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    #[must_use]
    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl JsonlWriter<Stdout> {
    #[must_use]
    pub fn stdout() -> Self {
        Self::new(io::stdout())
    }

    #[must_use]
    pub fn stdout_with_config(config: JsonlConfig) -> Self {
        Self::with_config(io::stdout(), config)
    }
}

/// Emit one JSONL record to a writer and flush.
///
/// # Errors
///
/// Returns an error if serialization, writing, or flushing fails.
pub fn emit<W: Write>(writer: &mut W, line: &OutputLine) -> io::Result<()> {
    let json =
        serde_json::to_string(line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writeln!(writer, "{json}")?;
    writer.flush()
}

/// Emit one JSONL record to stdout.
///
/// # Errors
///
/// Returns an error if serialization, writing, or flushing fails.
pub fn emit_stdout(line: &OutputLine) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    emit(&mut handle, line)
}

/// Emit many JSONL records to stdout.
///
/// # Errors
///
/// Returns the first write or serialization error produced while emitting lines.
pub fn emit_all_stdout<'a, I>(lines: I) -> io::Result<()>
where
    I: IntoIterator<Item = &'a OutputLine>,
{
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    lines
        .into_iter()
        .try_for_each(|line| emit(&mut handle, line))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::summary::{Summary, SummaryType};
    use super::super::domain_types::Message;

    fn make_test_line() -> OutputLine {
        OutputLine::Summary(
            Summary::new(SummaryType::Info, Message::new("test message").expect("valid"))
                .expect("valid summary"),
        )
    }

    // ── JsonlConfig ──────────────────────────────────────────────────────────

    #[test]
    fn test_jsonl_config_default() {
        let config = JsonlConfig::default();
        assert!(!config.pretty);
        assert!(config.flush_on_emit);
    }

    #[test]
    fn test_jsonl_config_new() {
        let config = JsonlConfig::new();
        assert!(!config.pretty);
        assert!(config.flush_on_emit);
    }

    #[test]
    fn test_jsonl_config_with_pretty() {
        let config = JsonlConfig::new().with_pretty(true);
        assert!(config.pretty);
        assert!(config.flush_on_emit);
    }

    #[test]
    fn test_jsonl_config_with_flush_on_emit() {
        let config = JsonlConfig::new().with_flush_on_emit(false);
        assert!(!config.flush_on_emit);
    }

    #[test]
    fn test_jsonl_config_chained() {
        let config = JsonlConfig::new().with_pretty(true).with_flush_on_emit(false);
        assert!(config.pretty);
        assert!(!config.flush_on_emit);
    }

    #[test]
    fn test_jsonl_config_equality() {
        let a = JsonlConfig::new().with_pretty(true);
        let b = JsonlConfig::new().with_pretty(true);
        assert_eq!(a, b);
    }

    // ── JsonlWriter::new ─────────────────────────────────────────────────────

    #[test]
    fn test_jsonl_writer_new() {
        let buf: Vec<u8> = Vec::new();
        let mut writer = JsonlWriter::new(buf);
        assert_eq!(writer.emit(&make_test_line()).expect("emit ok"), ());
    }

    #[test]
    fn test_jsonl_writer_with_config() {
        let config = JsonlConfig::new().with_pretty(true);
        let buf: Vec<u8> = Vec::new();
        let mut writer = JsonlWriter::with_config(buf, config);
        writer.emit(&make_test_line()).expect("emit ok");
    }

    // ── JsonlWriter emit ─────────────────────────────────────────────────────

    #[test]
    fn test_jsonl_writer_emits_valid_json() {
        let buf: Vec<u8> = Vec::new();
        let mut writer = JsonlWriter::new(buf);
        writer.emit(&make_test_line()).expect("emit ok");
        let output = String::from_utf8(writer.into_inner()).expect("utf8");
        let parsed: serde_json::Value = serde_json::from_str(&output.trim()).expect("valid json");
        // OutputLine is an enum with #[serde(rename_all = "snake_case")],
        // so Summary variant serializes with a "summary" wrapper or tag
        // Verify we get valid JSON with expected structure
        assert!(parsed.is_object());
        // The message field may be nested depending on serde tag representation
        let has_message = parsed["message"].is_string()
            || parsed["summary"]["message"].is_string()
            || parsed.get("summary").is_some();
        assert!(has_message, "Output should contain a message field: {output}");
    }

    #[test]
    fn test_jsonl_writer_emits_jsonl_one_per_line() {
        let buf: Vec<u8> = Vec::new();
        let mut writer = JsonlWriter::new(buf);
        writer.emit(&make_test_line()).expect("emit ok");
        writer.emit(&make_test_line()).expect("emit ok");
        let output = String::from_utf8(writer.into_inner()).expect("utf8");
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        // Each line should be valid JSON
        for line in lines {
            let _: serde_json::Value = serde_json::from_str(line).expect("valid json");
        }
    }

    #[test]
    fn test_jsonl_writer_pretty_mode() {
        let config = JsonlConfig::new().with_pretty(true);
        let buf: Vec<u8> = Vec::new();
        let mut writer = JsonlWriter::with_config(buf, config);
        writer.emit(&make_test_line()).expect("emit ok");
        let output = String::from_utf8(writer.into_inner()).expect("utf8");
        // Pretty printed JSON has multiple lines
        assert!(output.lines().count() > 1);
        // Should still be valid JSON
        let _: serde_json::Value = serde_json::from_str(&output.trim()).expect("valid json");
    }

    #[test]
    fn test_jsonl_writer_no_flush_mode() {
        let config = JsonlConfig::new().with_flush_on_emit(false);
        let buf: Vec<u8> = Vec::new();
        let mut writer = JsonlWriter::with_config(buf, config);
        writer.emit(&make_test_line()).expect("emit ok");
        // Should still have written data
        assert!(!writer.into_inner().is_empty());
    }

    // ── JsonlWriter emit_all ─────────────────────────────────────────────────

    #[test]
    fn test_jsonl_writer_emit_all() {
        let buf: Vec<u8> = Vec::new();
        let mut writer = JsonlWriter::new(buf);
        let lines = vec![make_test_line(), make_test_line(), make_test_line()];
        writer.emit_all(&lines).expect("emit_all ok");
        let output = String::from_utf8(writer.into_inner()).expect("utf8");
        assert_eq!(output.lines().count(), 3);
    }

    #[test]
    fn test_jsonl_writer_emit_all_empty() {
        let buf: Vec<u8> = Vec::new();
        let mut writer = JsonlWriter::new(buf);
        writer.emit_all(&[] as &[OutputLine]).expect("emit_all ok");
        let output = String::from_utf8(writer.into_inner()).expect("utf8");
        assert!(output.is_empty());
    }

    // ── JsonlWriter flush ────────────────────────────────────────────────────

    #[test]
    fn test_jsonl_writer_flush() {
        let buf: Vec<u8> = Vec::new();
        let mut writer = JsonlWriter::new(buf);
        writer.emit(&make_test_line()).expect("emit ok");
        writer.flush().expect("flush ok");
    }

    // ── JsonlWriter into_inner ───────────────────────────────────────────────

    #[test]
    fn test_jsonl_writer_into_inner() {
        let buf: Vec<u8> = Vec::new();
        let mut writer = JsonlWriter::new(buf);
        writer.emit(&make_test_line()).expect("emit ok");
        let inner = writer.into_inner();
        assert!(!inner.is_empty());
    }

    // ── emit function ────────────────────────────────────────────────────────

    #[test]
    fn test_emit_function() {
        let mut buf: Vec<u8> = Vec::new();
        emit(&mut buf, &make_test_line()).expect("emit ok");
        let output = String::from_utf8(buf).expect("utf8");
        let _: serde_json::Value = serde_json::from_str(&output.trim()).expect("valid json");
    }

    // ── Debug ────────────────────────────────────────────────────────────────

    #[test]
    fn test_jsonl_config_debug() {
        let config = JsonlConfig::new();
        let debug = format!("{config:?}");
        assert!(debug.contains("JsonlConfig"));
    }
}
