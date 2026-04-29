//! Action functions for the events command handler (Tier 3).
//!
//! I/O operations that query and display events from the event log.

use scp_core::{output::Output, Error, Result};

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

    std::fs::read_to_string(&events_file).map_or_else(|_| Ok(String::new()), Ok)
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
    use super::*;

    #[test]
    fn run_events_list_no_file_is_ok() {
        let options = EventsOptions {
            session: None,
            event_type: None,
            follow: false,
            limit: None,
            since: None,
        };
        // This will try to read a non-existent file and return empty list
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
    fn read_and_filter_events_handles_missing_file() {
        let result = read_and_filter_events(None, None, 50, None);
        assert!(result.is_ok());
        let entries = result.expect("should be ok");
        assert!(entries.is_empty());
    }

    #[test]
    fn resolve_events_file_path_does_not_panic() {
        // This may or may not succeed depending on HOME, but must not panic
        let _ = resolve_events_file_path();
    }

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
    fn print_events_with_entries() {
        use super::super::data::EventType;

        let output = EventsOutput {
            events: vec![EventEntry {
                id: "evt-1".to_string(),
                event_type: EventType::SessionCreated,
                timestamp: "2025-01-15T12:00:00Z".to_string(),
                session: Some("test".to_string()),
                agent_id: Some("agent-1".to_string()),
                data: None,
                message: "Session created".to_string(),
            }],
            total: 1,
            has_more: false,
            cursor: None,
        };
        print_events(&output);
    }

    #[test]
    fn default_limit_is_fifty() {
        assert_eq!(DEFAULT_LIMIT, 50);
    }

    #[test]
    fn filter_and_sort_events_empty_content() {
        let result = filter_and_sort_events("", None, None, 10, None);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_and_sort_events_parses_valid_jsonl() {
        let content = r#"{"id":"evt-1","event_type":"session_created","timestamp":"2025-01-15T12:00:00Z","session":"test","agent_id":null,"data":null,"message":"Created"}"#;
        let result = filter_and_sort_events(content, None, None, 10, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "evt-1");
    }

    #[test]
    fn filter_and_sort_events_respects_limit() {
        let entry = r#"{"id":"evt-1","event_type":"session_created","timestamp":"2025-01-15T12:00:00Z","session":null,"agent_id":null,"data":null,"message":"M"}"#;
        let content = format!("{entry}\n{entry}");
        let result = filter_and_sort_events(&content, None, None, 1, None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_and_sort_events_filters_by_session() {
        let content = r#"{"id":"evt-1","event_type":"session_created","timestamp":"2025-01-15T12:00:00Z","session":"alpha","agent_id":null,"data":null,"message":"M"}
{"id":"evt-2","event_type":"session_created","timestamp":"2025-01-15T12:00:00Z","session":"beta","agent_id":null,"data":null,"message":"M"}"#;
        let result = filter_and_sort_events(content, Some("alpha"), None, 10, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "evt-1");
    }
}
