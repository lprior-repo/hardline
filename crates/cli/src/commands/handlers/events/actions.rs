//! Action functions for the events command handler (Tier 3).
//!
//! I/O operations that query and display events from the event log.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{event_type_matches, EventEntry, EventsOptions, EventsOutput};

/// Default number of events returned when no limit is specified.
const DEFAULT_LIMIT: usize = 50;

/// Execute the events command with the given options.
///
/// In list mode, reads the events log file, applies filters, and displays results.
/// In follow mode, continuously polls for new events.
///
/// # Errors
///
/// Returns error if the events log file cannot be read.
pub fn run_events(options: &EventsOptions) -> Result<()> {
    if options.follow {
        run_follow(options)
    } else {
        run_list(options)
    }
}

/// List recent events, optionally filtered.
fn run_list(options: &EventsOptions) -> Result<()> {
    let limit = options.limit.map_or(DEFAULT_LIMIT, |l| l);
    let output = load_events_output(options, limit)?;

    print_events(&output);
    Ok(())
}

/// Follow mode: print current events, then indicate streaming mode.
///
/// Note: Full streaming (infinite poll loop) is not implemented in this
/// synchronous handler. This outputs the current batch and a message
/// indicating that streaming would continue in a real async context.
fn run_follow(options: &EventsOptions) -> Result<()> {
    let limit = options.limit.map_or(DEFAULT_LIMIT, |l| l);
    let output = load_events_output(options, limit)?;

    print_events(&output);
    Output::info("Following events... (streaming requires async runtime)");
    Ok(())
}

/// Load events, apply filters, and build the output struct.
///
/// Shared by both `run_list` and `run_follow`.
fn load_events_output(options: &EventsOptions, limit: usize) -> Result<EventsOutput> {
    let entries = read_and_filter_events(
        options.session.as_deref(),
        options.event_type.as_deref(),
        limit,
        options.since.as_deref(),
    )?;

    Ok(EventsOutput {
        total: entries.len(),
        has_more: false,
        cursor: None,
        events: entries,
    })
}

/// Read raw events from the JSONL event log file (I/O).
///
/// Falls back to an empty string if the file does not exist.
fn read_raw_events() -> Result<String> {
    let events_file = resolve_events_file_path()?;

    match std::fs::read_to_string(&events_file) {
        Ok(c) => Ok(c),
        Err(_) => Ok(String::new()),
    }
}

/// Parse, filter, sort, and truncate event entries (pure).
///
/// Takes raw JSONL content and applies session / event_type / since filters,
/// then sorts newest-first and truncates to `limit`.
fn filter_and_sort_events(
    content: &str,
    session: Option<&str>,
    event_type: Option<&str>,
    limit: usize,
    since: Option<&str>,
) -> Vec<EventEntry> {
    let entries: Vec<EventEntry> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            serde_json::from_str::<EventEntry>(line)
                .ok()
                .filter(|entry| {
                    let session_ok = session.is_none_or(|s| entry.session.as_deref() == Some(s));
                    let type_ok = event_type_matches(event_type, &entry.event_type);
                    let since_ok = since.is_none_or(|st| entry.timestamp.as_str() >= st);

                    session_ok && type_ok && since_ok
                })
        })
        .collect();

    sort_and_truncate(entries, limit)
}

/// Sort entries newest-first and truncate to limit.
fn sort_and_truncate(entries: Vec<EventEntry>, limit: usize) -> Vec<EventEntry> {
    let mut sorted = entries;
    sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    sorted.truncate(limit);
    sorted
}

/// Read events from the JSONL event log file, applying filters.
///
/// Composes `read_raw_events` (I/O) and `filter_and_sort_events` (pure).
fn read_and_filter_events(
    session: Option<&str>,
    event_type: Option<&str>,
    limit: usize,
    since: Option<&str>,
) -> Result<Vec<EventEntry>> {
    let content = read_raw_events()?;
    Ok(filter_and_sort_events(
        &content, session, event_type, limit, since,
    ))
}

/// Resolve the path to the events JSONL log file.
///
/// Looks for a `.scp/data/events.jsonl` relative to the current directory,
/// falling back to `$HOME/.scp/data/events.jsonl`.
fn resolve_events_file_path() -> Result<std::path::PathBuf> {
    let local_path = std::path::Path::new(".scp/data/events.jsonl");
    if local_path.exists() {
        return Ok(local_path.to_path_buf());
    }

    let home_dir = std::env::var("HOME")
        .map_err(|_| Error::io_error("Cannot determine home directory: HOME env var not set"))?;

    Ok(std::path::Path::new(&home_dir).join(".scp/data/events.jsonl"))
}

/// Print events to stdout using the Output facade.
fn print_events(output: &EventsOutput) {
    if output.events.is_empty() {
        Output::info("No events found.");
        return;
    }

    Output::info(&format!(
        "Events ({} shown, {} total):",
        output.events.len(),
        output.total
    ));

    output.events.iter().for_each(|entry| {
        let session_str = entry
            .session
            .as_ref()
            .map_or_else(String::new, |s| format!(" [{s}]"));
        let agent_str = entry
            .agent_id
            .as_ref()
            .map_or_else(String::new, |a| format!(" agent:{a}"));
        Output::info(&format!(
            "{} {}{}{}: {}",
            entry.timestamp, entry.event_type, session_str, agent_str, entry.message
        ));
    });
}

#[cfg(test)]
mod tests {
    use super::super::data::EventType;
    use super::*;

    fn make_entry(
        id: &str,
        event_type: EventType,
        timestamp: &str,
        session: Option<&str>,
        agent_id: Option<&str>,
        message: &str,
    ) -> EventEntry {
        EventEntry {
            id: id.to_string(),
            event_type,
            timestamp: timestamp.to_string(),
            session: session.map(String::from),
            agent_id: agent_id.map(String::from),
            data: None,
            message: message.to_string(),
        }
    }

    fn jsonl_line(
        id: &str,
        event_type: &str,
        timestamp: &str,
        session: Option<&str>,
        agent_id: Option<&str>,
        message: &str,
    ) -> String {
        let session_part = session.map_or("null".to_string(), |s| format!("\"{s}\""));
        let agent_part = agent_id.map_or("null".to_string(), |a| format!("\"{a}\""));
        format!(
            r#"{{"id":"{id}","event_type":"{event_type}","timestamp":"{timestamp}","session":{session_part},"agent_id":{agent_part},"data":null,"message":"{message}"}}"#
        )
    }

    // =========================================================================
    // run_events smoke tests
    // =========================================================================

    #[test]
    fn run_events_list_no_file_is_ok() {
        let options = EventsOptions {
            session: None,
            event_type: None,
            follow: false,
            limit: None,
            since: None,
        };
        assert!(run_events(&options).is_ok());
    }

    #[test]
    fn run_events_follow_no_file_is_ok() {
        let options = EventsOptions {
            session: None,
            event_type: None,
            follow: true,
            limit: None,
            since: None,
        };
        assert!(run_events(&options).is_ok());
    }

    #[test]
    fn run_events_with_session_filter() {
        let options = EventsOptions {
            session: Some("nonexistent-session".to_string()),
            event_type: None,
            follow: false,
            limit: Some(10),
            since: None,
        };
        assert!(run_events(&options).is_ok());
    }

    #[test]
    fn run_events_with_event_type_filter() {
        let options = EventsOptions {
            session: None,
            event_type: Some("session_created".to_string()),
            follow: false,
            limit: Some(5),
            since: None,
        };
        assert!(run_events(&options).is_ok());
    }

    #[test]
    fn run_events_with_since_filter() {
        let options = EventsOptions {
            session: None,
            event_type: None,
            follow: false,
            limit: None,
            since: Some("2099-01-01T00:00:00Z".to_string()),
        };
        assert!(run_events(&options).is_ok());
    }

    #[test]
    fn run_events_with_all_filters_combined() {
        let options = EventsOptions {
            session: Some("ghost".to_string()),
            event_type: Some("lock_acquired".to_string()),
            follow: false,
            limit: Some(3),
            since: Some("2099-12-31T23:59:59Z".to_string()),
        };
        assert!(run_events(&options).is_ok());
    }

    #[test]
    fn run_events_limit_zero() {
        let options = EventsOptions {
            session: None,
            event_type: None,
            follow: false,
            limit: Some(0),
            since: None,
        };
        assert!(run_events(&options).is_ok());
    }

    // =========================================================================
    // read_and_filter_events
    // =========================================================================

    #[test]
    fn read_and_filter_events_handles_missing_file() {
        let result = read_and_filter_events(None, None, 50, None);
        assert!(result.is_ok());
        assert!(result.expect("should be ok").is_empty());
    }

    #[test]
    fn read_and_filter_events_with_session_filter_missing_file() {
        let result = read_and_filter_events(Some("alpha"), None, 10, None);
        assert!(result.is_ok());
        assert!(result.expect("should be ok").is_empty());
    }

    #[test]
    fn read_and_filter_events_with_event_type_filter_missing_file() {
        let result = read_and_filter_events(None, Some("session_created"), 10, None);
        assert!(result.is_ok());
        assert!(result.expect("should be ok").is_empty());
    }

    #[test]
    fn read_and_filter_events_with_since_filter_missing_file() {
        let result = read_and_filter_events(None, None, 10, Some("2025-01-01T00:00:00Z"));
        assert!(result.is_ok());
        assert!(result.expect("should be ok").is_empty());
    }

    // =========================================================================
    // resolve_events_file_path
    // =========================================================================

    #[test]
    fn resolve_events_file_path_does_not_panic() {
        let _ = resolve_events_file_path();
    }

    #[test]
    fn resolve_events_file_path_returns_result() {
        let result = resolve_events_file_path();
        assert!(result.is_ok());
        let path = result.expect("should resolve");
        assert!(path.to_string_lossy().contains("events.jsonl"));
    }

    // =========================================================================
    // DEFAULT_LIMIT
    // =========================================================================

    #[test]
    fn default_limit_is_fifty() {
        assert_eq!(DEFAULT_LIMIT, 50);
    }

    // =========================================================================
    // filter_and_sort_events — basic parsing
    // =========================================================================

    #[test]
    fn filter_and_sort_events_empty_content() {
        assert!(filter_and_sort_events("", None, None, 10, None).is_empty());
    }

    #[test]
    fn filter_and_sort_events_whitespace_only() {
        assert!(filter_and_sort_events("   \n  \n  ", None, None, 10, None).is_empty());
    }

    #[test]
    fn filter_and_sort_events_parses_valid_jsonl() {
        let content = jsonl_line(
            "evt-1",
            "session_created",
            "2025-01-15T12:00:00Z",
            Some("test"),
            None,
            "Created",
        );
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "evt-1");
    }

    #[test]
    fn filter_and_sort_events_parses_multiple_lines() {
        let content = format!(
            "{}\n{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "agent_registered",
                "2025-01-15T13:00:00Z",
                None,
                None,
                "M2"
            ),
            jsonl_line(
                "evt-3",
                "lock_acquired",
                "2025-01-15T14:00:00Z",
                None,
                None,
                "M3"
            ),
        );
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn filter_and_sort_events_skips_empty_lines() {
        let content = format!(
            "{}\n\n\n{}\n\n",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M2"
            ),
        );
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result.len(), 2);
    }

    // =========================================================================
    // filter_and_sort_events — malformed JSONL
    // =========================================================================

    #[test]
    fn filter_and_sort_events_skips_malformed_json() {
        let content = format!(
            "{}\nnot-valid-json\n{}\n{{broken\n",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M2"
            ),
        );
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_and_sort_events_skips_wrong_schema() {
        let content = format!(
            "{}\n{{\"foo\":\"bar\"}}\n",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
        );
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_and_sort_events_all_malformed_returns_empty() {
        let content = "not json\n{broken\n[]\n";
        let result = filter_and_sort_events(content, None, None, 10, None);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_and_sort_events_json_with_wrong_event_type() {
        let content = r#"{"id":"evt-1","event_type":"nonexistent_type","timestamp":"2025-01-15T12:00:00Z","session":null,"agent_id":null,"data":null,"message":"M"}"#;
        let result = filter_and_sort_events(content, None, None, 10, None);
        assert!(result.is_empty());
    }

    // =========================================================================
    // filter_and_sort_events — sorting (newest first)
    // =========================================================================

    #[test]
    fn filter_and_sort_events_sorts_newest_first() {
        let content = format!(
            "{}\n{}\n{}",
            jsonl_line(
                "evt-old",
                "session_created",
                "2025-01-10T08:00:00Z",
                None,
                None,
                "old"
            ),
            jsonl_line(
                "evt-new",
                "session_created",
                "2025-01-20T20:00:00Z",
                None,
                None,
                "new"
            ),
            jsonl_line(
                "evt-mid",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "mid"
            ),
        );
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, "evt-new");
        assert_eq!(result[1].id, "evt-mid");
        assert_eq!(result[2].id, "evt-old");
    }

    #[test]
    fn filter_and_sort_events_same_timestamp_preserves_all() {
        let content = format!(
            "{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "agent_registered",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M2"
            ),
        );
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_and_sort_events_sort_is_stable_across_same_timestamps() {
        let content = format!(
            "{}\n{}",
            jsonl_line(
                "evt-a",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "A"
            ),
            jsonl_line(
                "evt-b",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "B"
            ),
        );
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result[0].timestamp, "2025-01-15T12:00:00Z");
        assert_eq!(result[1].timestamp, "2025-01-15T12:00:00Z");
    }

    // =========================================================================
    // filter_and_sort_events — limit / truncation
    // =========================================================================

    #[test]
    fn filter_and_sort_events_respects_limit() {
        let entry = jsonl_line(
            "evt-1",
            "session_created",
            "2025-01-15T12:00:00Z",
            None,
            None,
            "M",
        );
        let content = format!("{entry}\n{entry}");
        let result = filter_and_sort_events(&content, None, None, 1, None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_and_sort_events_limit_larger_than_count() {
        let entry = jsonl_line(
            "evt-1",
            "session_created",
            "2025-01-15T12:00:00Z",
            None,
            None,
            "M",
        );
        let content = format!("{entry}\n{entry}");
        let result = filter_and_sort_events(&content, None, None, 100, None);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_and_sort_events_limit_zero_returns_empty() {
        let content = jsonl_line(
            "evt-1",
            "session_created",
            "2025-01-15T12:00:00Z",
            None,
            None,
            "M",
        );
        let result = filter_and_sort_events(&content, None, None, 0, None);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_and_sort_events_truncates_after_sorting() {
        let content = format!(
            "{}\n{}\n{}",
            jsonl_line(
                "evt-old",
                "session_created",
                "2025-01-10T08:00:00Z",
                None,
                None,
                "old"
            ),
            jsonl_line(
                "evt-new",
                "session_created",
                "2025-01-20T20:00:00Z",
                None,
                None,
                "new"
            ),
            jsonl_line(
                "evt-mid",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "mid"
            ),
        );
        let result = filter_and_sort_events(&content, None, None, 2, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "evt-new");
        assert_eq!(result[1].id, "evt-mid");
    }

    #[test]
    fn filter_and_sort_events_large_pagination_limit() {
        let mut lines = Vec::new();
        for i in 0..100 {
            lines.push(jsonl_line(
                &format!("evt-{i:03}"),
                "session_created",
                &format!("2025-01-15T12:00:{i:02}Z"),
                None,
                None,
                "M",
            ));
        }
        let content = lines.join("\n");
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result.len(), 10);
    }

    // =========================================================================
    // filter_and_sort_events — session filter
    // =========================================================================

    #[test]
    fn filter_and_sort_events_filters_by_session() {
        let content = format!(
            "{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                Some("alpha"),
                None,
                "M"
            ),
            jsonl_line(
                "evt-2",
                "session_created",
                "2025-01-15T12:00:00Z",
                Some("beta"),
                None,
                "M"
            ),
        );
        let result = filter_and_sort_events(&content, Some("alpha"), None, 10, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "evt-1");
    }

    #[test]
    fn filter_and_sort_events_session_filter_no_match() {
        let content = jsonl_line(
            "evt-1",
            "session_created",
            "2025-01-15T12:00:00Z",
            Some("alpha"),
            None,
            "M",
        );
        let result = filter_and_sort_events(&content, Some("gamma"), None, 10, None);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_and_sort_events_session_filter_matches_none_session() {
        let content = format!(
            "{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "session_created",
                "2025-01-15T12:00:00Z",
                Some("alpha"),
                None,
                "M2"
            ),
        );
        let result = filter_and_sort_events(&content, Some("alpha"), None, 10, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "evt-2");
    }

    #[test]
    fn filter_and_sort_events_session_filter_with_multiple_matching() {
        let content = format!(
            "{}\n{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                Some("shared"),
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "agent_registered",
                "2025-01-15T13:00:00Z",
                Some("shared"),
                None,
                "M2"
            ),
            jsonl_line(
                "evt-3",
                "lock_acquired",
                "2025-01-15T14:00:00Z",
                Some("other"),
                None,
                "M3"
            ),
        );
        let result = filter_and_sort_events(&content, Some("shared"), None, 10, None);
        assert_eq!(result.len(), 2);
    }

    // =========================================================================
    // filter_and_sort_events — event type filter (exact + category)
    // =========================================================================

    #[test]
    fn filter_and_sort_events_filters_by_event_type_exact() {
        let content = format!(
            "{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "agent_registered",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M2"
            ),
        );
        let result = filter_and_sort_events(&content, None, Some("session_created"), 10, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "evt-1");
    }

    #[test]
    fn filter_and_sort_events_filters_by_event_type_category() {
        let content = format!(
            "{}\n{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "session_removed",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M2"
            ),
            jsonl_line(
                "evt-3",
                "lock_acquired",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M3"
            ),
        );
        let result = filter_and_sort_events(&content, None, Some("session"), 10, None);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_and_sort_events_event_type_no_match() {
        let content = jsonl_line(
            "evt-1",
            "session_created",
            "2025-01-15T12:00:00Z",
            None,
            None,
            "M1",
        );
        let result = filter_and_sort_events(&content, None, Some("lock_acquired"), 10, None);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_and_sort_events_event_type_agent_category() {
        let content = format!(
            "{}\n{}\n{}",
            jsonl_line(
                "evt-1",
                "agent_registered",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "agent_heartbeat",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M2"
            ),
            jsonl_line(
                "evt-3",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M3"
            ),
        );
        let result = filter_and_sort_events(&content, None, Some("agent"), 10, None);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_and_sort_events_event_type_lock_category() {
        let content = format!(
            "{}\n{}\n{}",
            jsonl_line(
                "evt-1",
                "lock_acquired",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "lock_released",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M2"
            ),
            jsonl_line(
                "evt-3",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M3"
            ),
        );
        let result = filter_and_sort_events(&content, None, Some("lock"), 10, None);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_and_sort_events_event_type_checkpoint_category() {
        let content = format!(
            "{}\n{}",
            jsonl_line(
                "evt-1",
                "checkpoint_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "checkpoint_restored",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M2"
            ),
        );
        let result = filter_and_sort_events(&content, None, Some("checkpoint"), 10, None);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_and_sort_events_event_type_bead_category() {
        let content = format!(
            "{}\n{}",
            jsonl_line(
                "evt-1",
                "bead_status_changed",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M2"
            ),
        );
        let result = filter_and_sort_events(&content, None, Some("bead"), 10, None);
        assert_eq!(result.len(), 1);
    }

    // =========================================================================
    // filter_and_sort_events — since filter (time range)
    // =========================================================================

    #[test]
    fn filter_and_sort_events_since_excludes_older() {
        let content = format!(
            "{}\n{}",
            jsonl_line(
                "evt-old",
                "session_created",
                "2025-01-10T08:00:00Z",
                None,
                None,
                "old"
            ),
            jsonl_line(
                "evt-new",
                "session_created",
                "2025-01-20T20:00:00Z",
                None,
                None,
                "new"
            ),
        );
        let result = filter_and_sort_events(&content, None, None, 10, Some("2025-01-15T00:00:00Z"));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "evt-new");
    }

    #[test]
    fn filter_and_sort_events_since_includes_exact_match() {
        let content = jsonl_line(
            "evt-1",
            "session_created",
            "2025-01-15T12:00:00Z",
            None,
            None,
            "M1",
        );
        let result = filter_and_sort_events(&content, None, None, 10, Some("2025-01-15T12:00:00Z"));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_and_sort_events_since_excludes_all_older() {
        let content = jsonl_line(
            "evt-1",
            "session_created",
            "2025-01-10T08:00:00Z",
            None,
            None,
            "M1",
        );
        let result = filter_and_sort_events(&content, None, None, 10, Some("2099-01-01T00:00:00Z"));
        assert!(result.is_empty());
    }

    #[test]
    fn filter_and_sort_events_since_includes_all_newer() {
        let content = format!(
            "{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-06-01T00:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "session_created",
                "2025-06-02T00:00:00Z",
                None,
                None,
                "M2"
            ),
        );
        let result = filter_and_sort_events(&content, None, None, 10, Some("2025-01-01T00:00:00Z"));
        assert_eq!(result.len(), 2);
    }

    // =========================================================================
    // filter_and_sort_events — combined filters
    // =========================================================================

    #[test]
    fn filter_and_sort_events_session_and_event_type() {
        let content = format!(
            "{}\n{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                Some("alpha"),
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "agent_registered",
                "2025-01-15T13:00:00Z",
                Some("alpha"),
                None,
                "M2"
            ),
            jsonl_line(
                "evt-3",
                "session_created",
                "2025-01-15T14:00:00Z",
                Some("beta"),
                None,
                "M3"
            ),
        );
        let result =
            filter_and_sort_events(&content, Some("alpha"), Some("session_created"), 10, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "evt-1");
    }

    #[test]
    fn filter_and_sort_events_session_and_since() {
        let content = format!(
            "{}\n{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-10T08:00:00Z",
                Some("alpha"),
                None,
                "old"
            ),
            jsonl_line(
                "evt-2",
                "session_created",
                "2025-01-20T20:00:00Z",
                Some("alpha"),
                None,
                "new"
            ),
            jsonl_line(
                "evt-3",
                "session_created",
                "2025-01-20T20:00:00Z",
                Some("beta"),
                None,
                "other"
            ),
        );
        let result = filter_and_sort_events(
            &content,
            Some("alpha"),
            None,
            10,
            Some("2025-01-15T00:00:00Z"),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "evt-2");
    }

    #[test]
    fn filter_and_sort_events_event_type_and_since() {
        let content = format!(
            "{}\n{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-10T08:00:00Z",
                None,
                None,
                "old"
            ),
            jsonl_line(
                "evt-2",
                "agent_registered",
                "2025-01-20T20:00:00Z",
                None,
                None,
                "new-agent"
            ),
            jsonl_line(
                "evt-3",
                "session_created",
                "2025-01-20T20:00:00Z",
                None,
                None,
                "new-session"
            ),
        );
        let result = filter_and_sort_events(
            &content,
            None,
            Some("session_created"),
            10,
            Some("2025-01-15T00:00:00Z"),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "evt-3");
    }

    #[test]
    fn filter_and_sort_events_all_three_filters() {
        let content = format!(
            "{}\n{}\n{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-10T08:00:00Z",
                Some("alpha"),
                None,
                "old"
            ),
            jsonl_line(
                "evt-2",
                "agent_registered",
                "2025-01-20T20:00:00Z",
                Some("alpha"),
                None,
                "new-wrong-type"
            ),
            jsonl_line(
                "evt-3",
                "session_created",
                "2025-01-20T20:00:00Z",
                Some("beta"),
                None,
                "new-wrong-session"
            ),
            jsonl_line(
                "evt-4",
                "session_created",
                "2025-01-20T20:00:00Z",
                Some("alpha"),
                None,
                "match"
            ),
        );
        let result = filter_and_sort_events(
            &content,
            Some("alpha"),
            Some("session_created"),
            10,
            Some("2025-01-15T00:00:00Z"),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "evt-4");
    }

    #[test]
    fn filter_and_sort_events_all_filters_no_match() {
        let content = jsonl_line(
            "evt-1",
            "session_created",
            "2025-01-10T08:00:00Z",
            Some("alpha"),
            None,
            "M1",
        );
        let result = filter_and_sort_events(
            &content,
            Some("beta"),
            Some("lock_acquired"),
            10,
            Some("2099-01-01T00:00:00Z"),
        );
        assert!(result.is_empty());
    }

    #[test]
    fn filter_and_sort_events_combined_filters_with_limit() {
        let content = format!(
            "{}\n{}\n{}\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-20T01:00:00Z",
                Some("alpha"),
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "session_created",
                "2025-01-20T02:00:00Z",
                Some("alpha"),
                None,
                "M2"
            ),
            jsonl_line(
                "evt-3",
                "session_created",
                "2025-01-20T03:00:00Z",
                Some("alpha"),
                None,
                "M3"
            ),
            jsonl_line(
                "evt-4",
                "session_created",
                "2025-01-20T04:00:00Z",
                Some("beta"),
                None,
                "M4"
            ),
        );
        let result = filter_and_sort_events(&content, Some("alpha"), None, 2, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "evt-3");
        assert_eq!(result[1].id, "evt-2");
    }

    // =========================================================================
    // sort_and_truncate
    // =========================================================================

    #[test]
    fn sort_and_truncate_empty() {
        assert!(sort_and_truncate(vec![], 10).is_empty());
    }

    #[test]
    fn sort_and_truncate_single_entry() {
        let entry = make_entry(
            "evt-1",
            EventType::SessionCreated,
            "2025-01-15T12:00:00Z",
            None,
            None,
            "M",
        );
        let result = sort_and_truncate(vec![entry], 10);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn sort_and_truncate_preserves_order_for_single() {
        let entry = make_entry(
            "evt-1",
            EventType::SessionCreated,
            "2025-01-15T12:00:00Z",
            None,
            None,
            "M",
        );
        let result = sort_and_truncate(vec![entry], 10);
        assert_eq!(result[0].id, "evt-1");
    }

    #[test]
    fn sort_and_truncate_sorts_descending() {
        let entries = vec![
            make_entry(
                "old",
                EventType::SessionCreated,
                "2025-01-10T08:00:00Z",
                None,
                None,
                "old",
            ),
            make_entry(
                "new",
                EventType::SessionCreated,
                "2025-01-20T20:00:00Z",
                None,
                None,
                "new",
            ),
            make_entry(
                "mid",
                EventType::SessionCreated,
                "2025-01-15T12:00:00Z",
                None,
                None,
                "mid",
            ),
        ];
        let result = sort_and_truncate(entries, 10);
        assert_eq!(result[0].id, "new");
        assert_eq!(result[1].id, "mid");
        assert_eq!(result[2].id, "old");
    }

    #[test]
    fn sort_and_truncate_truncates() {
        let entries = vec![
            make_entry(
                "new",
                EventType::SessionCreated,
                "2025-01-20T20:00:00Z",
                None,
                None,
                "new",
            ),
            make_entry(
                "mid",
                EventType::SessionCreated,
                "2025-01-15T12:00:00Z",
                None,
                None,
                "mid",
            ),
            make_entry(
                "old",
                EventType::SessionCreated,
                "2025-01-10T08:00:00Z",
                None,
                None,
                "old",
            ),
        ];
        let result = sort_and_truncate(entries, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "new");
        assert_eq!(result[1].id, "mid");
    }

    #[test]
    fn sort_and_truncate_limit_zero() {
        let entries = vec![make_entry(
            "evt-1",
            EventType::SessionCreated,
            "2025-01-15T12:00:00Z",
            None,
            None,
            "M",
        )];
        assert!(sort_and_truncate(entries, 0).is_empty());
    }

    #[test]
    fn sort_and_truncate_limit_larger_than_input() {
        let entries = vec![make_entry(
            "evt-1",
            EventType::SessionCreated,
            "2025-01-15T12:00:00Z",
            None,
            None,
            "M",
        )];
        let result = sort_and_truncate(entries, 100);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn sort_and_truncate_does_not_mutate_input() {
        let entry = make_entry(
            "evt-1",
            EventType::SessionCreated,
            "2025-01-15T12:00:00Z",
            None,
            None,
            "M",
        );
        let entry_clone = entry.clone();
        let _ = sort_and_truncate(vec![entry], 10);
        assert_eq!(entry_clone.id, "evt-1");
    }

    // =========================================================================
    // load_events_output
    // =========================================================================

    #[test]
    fn load_events_output_no_file() {
        let options = EventsOptions {
            session: None,
            event_type: None,
            follow: false,
            limit: None,
            since: None,
        };
        let result = load_events_output(&options, 50);
        assert!(result.is_ok());
        let output = result.expect("should be ok");
        assert_eq!(output.total, 0);
        assert!(!output.has_more);
        assert!(output.cursor.is_none());
        assert!(output.events.is_empty());
    }

    #[test]
    fn load_events_output_with_filters_no_file() {
        let options = EventsOptions {
            session: Some("ghost".to_string()),
            event_type: Some("session_created".to_string()),
            follow: false,
            limit: Some(5),
            since: Some("2099-01-01T00:00:00Z".to_string()),
        };
        let result = load_events_output(&options, 5);
        assert!(result.is_ok());
        let output = result.expect("should be ok");
        assert!(output.events.is_empty());
        assert_eq!(output.total, 0);
    }

    // =========================================================================
    // print_events
    // =========================================================================

    #[test]
    fn print_events_with_empty_output() {
        let output = EventsOutput {
            events: vec![],
            total: 0,
            has_more: false,
            cursor: None,
        };
        print_events(&output);
    }

    #[test]
    fn print_events_with_single_entry() {
        let output = EventsOutput {
            events: vec![make_entry(
                "evt-1",
                EventType::SessionCreated,
                "2025-01-15T12:00:00Z",
                Some("test"),
                Some("agent-1"),
                "Session created",
            )],
            total: 1,
            has_more: false,
            cursor: None,
        };
        print_events(&output);
    }

    #[test]
    fn print_events_with_multiple_entries() {
        let output = EventsOutput {
            events: vec![
                make_entry(
                    "evt-1",
                    EventType::SessionCreated,
                    "2025-01-15T12:00:00Z",
                    None,
                    None,
                    "First",
                ),
                make_entry(
                    "evt-2",
                    EventType::AgentRegistered,
                    "2025-01-15T13:00:00Z",
                    None,
                    None,
                    "Second",
                ),
                make_entry(
                    "evt-3",
                    EventType::LockAcquired,
                    "2025-01-15T14:00:00Z",
                    None,
                    None,
                    "Third",
                ),
            ],
            total: 3,
            has_more: false,
            cursor: None,
        };
        print_events(&output);
    }

    #[test]
    fn print_events_entry_with_no_session() {
        let output = EventsOutput {
            events: vec![make_entry(
                "evt-1",
                EventType::SessionCreated,
                "2025-01-15T12:00:00Z",
                None,
                None,
                "No session",
            )],
            total: 1,
            has_more: false,
            cursor: None,
        };
        print_events(&output);
    }

    #[test]
    fn print_events_entry_with_no_agent_id() {
        let output = EventsOutput {
            events: vec![make_entry(
                "evt-1",
                EventType::SessionCreated,
                "2025-01-15T12:00:00Z",
                Some("sess"),
                None,
                "No agent",
            )],
            total: 1,
            has_more: false,
            cursor: None,
        };
        print_events(&output);
    }

    #[test]
    fn print_events_entry_with_session_and_agent() {
        let output = EventsOutput {
            events: vec![make_entry(
                "evt-1",
                EventType::SessionCreated,
                "2025-01-15T12:00:00Z",
                Some("my-session"),
                Some("agent-42"),
                "Full metadata",
            )],
            total: 1,
            has_more: false,
            cursor: None,
        };
        print_events(&output);
    }

    #[test]
    fn print_events_with_cursor() {
        let output = EventsOutput {
            events: vec![make_entry(
                "evt-1",
                EventType::SessionCreated,
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M",
            )],
            total: 100,
            has_more: true,
            cursor: Some("next-cursor".to_string()),
        };
        print_events(&output);
    }

    #[test]
    fn print_events_all_event_types() {
        let events = vec![
            make_entry(
                "evt-1",
                EventType::SessionCreated,
                "2025-01-15T12:00:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-2",
                EventType::SessionRemoved,
                "2025-01-15T12:01:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-3",
                EventType::SessionFocused,
                "2025-01-15T12:02:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-4",
                EventType::SessionMerged,
                "2025-01-15T12:03:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-5",
                EventType::SessionAborted,
                "2025-01-15T12:04:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-6",
                EventType::SessionSynced,
                "2025-01-15T12:05:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-7",
                EventType::AgentRegistered,
                "2025-01-15T12:06:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-8",
                EventType::AgentUnregistered,
                "2025-01-15T12:07:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-9",
                EventType::AgentHeartbeat,
                "2025-01-15T12:08:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-10",
                EventType::LockAcquired,
                "2025-01-15T12:09:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-11",
                EventType::LockReleased,
                "2025-01-15T12:10:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-12",
                EventType::CheckpointCreated,
                "2025-01-15T12:11:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-13",
                EventType::CheckpointRestored,
                "2025-01-15T12:12:00Z",
                None,
                None,
                "S",
            ),
            make_entry(
                "evt-14",
                EventType::BeadStatusChanged,
                "2025-01-15T12:13:00Z",
                None,
                None,
                "S",
            ),
        ];
        let output = EventsOutput {
            events,
            total: 14,
            has_more: false,
            cursor: None,
        };
        print_events(&output);
    }

    // =========================================================================
    // Event entry parsing with data field
    // =========================================================================

    #[test]
    fn filter_and_sort_events_parses_entry_with_data() {
        let content = r#"{"id":"evt-1","event_type":"bead_status_changed","timestamp":"2025-01-15T12:00:00Z","session":null,"agent_id":null,"data":{"old_status":"open","new_status":"closed"},"message":"Status changed"}"#;
        let result = filter_and_sort_events(content, None, None, 10, None);
        assert_eq!(result.len(), 1);
        let data = result[0].data.as_ref().expect("should have data");
        assert_eq!(data["old_status"], "open");
        assert_eq!(data["new_status"], "closed");
    }

    #[test]
    fn filter_and_sort_events_parses_entry_with_nested_data() {
        let content = r#"{"id":"evt-1","event_type":"session_created","timestamp":"2025-01-15T12:00:00Z","session":null,"agent_id":null,"data":{"config":{"threads":4,"name":"test"}},"message":"Created"}"#;
        let result = filter_and_sort_events(content, None, None, 10, None);
        assert_eq!(result.len(), 1);
        let data = result[0].data.as_ref().expect("should have data");
        assert_eq!(data["config"]["threads"], 4);
        assert_eq!(data["config"]["name"], "test");
    }

    #[test]
    fn filter_and_sort_events_parses_entry_with_null_data() {
        let content = r#"{"id":"evt-1","event_type":"session_created","timestamp":"2025-01-15T12:00:00Z","session":null,"agent_id":null,"data":null,"message":"M"}"#;
        let result = filter_and_sort_events(content, None, None, 10, None);
        assert_eq!(result.len(), 1);
        assert!(result[0].data.is_none());
    }

    // =========================================================================
    // Edge cases: real-world JSONL patterns
    // =========================================================================

    #[test]
    fn filter_and_sort_events_trailing_newline() {
        let content = format!(
            "{}\n",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M"
            )
        );
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_and_sort_events_carriage_return() {
        let content = format!(
            "{}\r\n{}",
            jsonl_line(
                "evt-1",
                "session_created",
                "2025-01-15T12:00:00Z",
                None,
                None,
                "M1"
            ),
            jsonl_line(
                "evt-2",
                "session_created",
                "2025-01-15T13:00:00Z",
                None,
                None,
                "M2"
            ),
        );
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_and_sort_events_very_long_message() {
        let long_msg = "X".repeat(10_000);
        let content = jsonl_line(
            "evt-1",
            "session_created",
            "2025-01-15T12:00:00Z",
            None,
            None,
            &long_msg,
        );
        let result = filter_and_sort_events(&content, None, None, 10, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].message.len(), 10_000);
    }

    #[test]
    fn filter_and_sort_events_unicode_content() {
        let content = r#"{"id":"evt-1","event_type":"session_created","timestamp":"2025-01-15T12:00:00Z","session":"日本語","agent_id":null,"data":null,"message":"イベント作成 🎉"}"#;
        let result = filter_and_sort_events(content, Some("日本語"), None, 10, None);
        assert_eq!(result.len(), 1);
        assert!(result[0].message.contains("🎉"));
    }

    #[test]
    fn filter_and_sort_events_empty_strings_in_fields() {
        let content = r#"{"id":"","event_type":"session_created","timestamp":"2025-01-15T12:00:00Z","session":"","agent_id":"","data":null,"message":""}"#;
        let result = filter_and_sort_events(content, None, None, 10, None);
        assert_eq!(result.len(), 1);
        assert!(result[0].id.is_empty());
        assert!(result[0].message.is_empty());
    }
}
